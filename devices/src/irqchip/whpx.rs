// Copyright 2021 The Chromium OS Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::convert::TryFrom;
use std::sync::Arc;

use sync::Mutex;

cfg_if::cfg_if! {
    if #[cfg(test)] {
        use base::FakeClock as Clock;
    } else {
        use base::Clock;
    }
}
use hypervisor::whpx::{WhpxVcpu, WhpxVm};
use hypervisor::{
    IoapicState, IrqRoute, IrqSource, IrqSourceChip, LapicState, MPState, MsiAddressMessage,
    MsiDataMessage, PicSelect, PicState, PitState, Vcpu, VcpuX86_64, Vm,
};
use resources::SystemAllocator;

use base::{error, Error, Event, Result, Tube};

use crate::irqchip::{
    DelayedIoApicIrqEvents, InterruptData, InterruptDestination, Ioapic, IrqEvent, IrqEventIndex,
    Pic, Routes, VcpuRunState, IOAPIC_BASE_ADDRESS, IOAPIC_MEM_LENGTH_BYTES,
};
use crate::{
    Bus, IrqChip, IrqChipCap, IrqChipX86_64, IrqEdgeEvent, IrqEventSource, IrqLevelEvent, Pit,
    PitError,
};

/// PIT channel 0 timer is connected to IRQ 0
const PIT_CHANNEL0_IRQ: u32 = 0;
/// TODO(b/187464379): we don't know what the right frequency is here, but 100MHz gives
/// us better results than the frequency that WSL seems to use, which is 500MHz.
const WHPX_LOCAL_APIC_EMULATION_APIC_FREQUENCY: u32 = 100_000_000;

/// The WhpxSplitIrqChip supports
pub struct WhpxSplitIrqChip {
    vm: WhpxVm,
    routes: Arc<Mutex<Routes>>,
    pit: Arc<Mutex<Pit>>,
    pic: Arc<Mutex<Pic>>,
    ioapic: Arc<Mutex<Ioapic>>,
    ioapic_pins: usize,
    /// Delayed ioapic irq object, that contains the delayed events because the ioapic was locked
    /// when service_irq was called on the irqchip. This prevents deadlocks when a Vcpu thread has
    /// locked the ioapic and the ioapic sends a AddMsiRoute signal to the main thread (which
    /// itself may be busy trying to call service_irq).
    ///
    /// ## Note:
    /// This lock may be locked by itself to access the `DelayedIoApicIrqEvents`. If accessed in
    /// conjunction with the `irq_events` field, that lock should be taken first to prevent
    /// deadlocks stemming from lock-ordering issues.
    delayed_ioapic_irq_events: Arc<Mutex<DelayedIoApicIrqEvents>>,
    /// Array of Events that devices will use to assert ioapic pins.
    irq_events: Arc<Mutex<Vec<Option<IrqEvent>>>>,
}

impl WhpxSplitIrqChip {
    /// Construct a new WhpxSplitIrqChip.
    pub fn new(vm: WhpxVm, irq_tube: Tube, ioapic_pins: Option<usize>) -> Result<Self> {
        let pit_evt = IrqEdgeEvent::new()?;
        let pit = Pit::new(pit_evt.try_clone()?, Arc::new(Mutex::new(Clock::new()))).map_err(
            |e| match e {
                PitError::CloneEvent(err) => err,
                PitError::CreateEvent(err) => err,
                PitError::CreateWaitContext(err) => err,
                PitError::WaitError(err) => err,
                PitError::TimerCreateError(err) => err,
                PitError::SpawnThread(_) => Error::new(libc::EIO),
            },
        )?;

        let pit_event_source = IrqEventSource::from_device(&pit);

        let ioapic_pins = ioapic_pins.unwrap_or(hypervisor::NUM_IOAPIC_PINS);
        let ioapic = Ioapic::new(irq_tube, ioapic_pins)?;

        let mut chip = WhpxSplitIrqChip {
            vm,
            routes: Arc::new(Mutex::new(Routes::new())),
            pit: Arc::new(Mutex::new(pit)),
            pic: Arc::new(Mutex::new(Pic::new())),
            ioapic: Arc::new(Mutex::new(ioapic)),
            ioapic_pins,
            delayed_ioapic_irq_events: Arc::new(Mutex::new(DelayedIoApicIrqEvents::new()?)),
            irq_events: Arc::new(Mutex::new(Vec::new())),
        };

        // This is equivalent to setting this in the blank Routes object above because
        // WhpxSplitIrqChip doesn't interact with the WHPX API when setting routes, but in case
        // that changes we do it this way.
        chip.set_irq_routes(&Routes::default_pic_ioapic_routes(ioapic_pins))?;

        // Add the pit
        chip.register_edge_irq_event(PIT_CHANNEL0_IRQ, &pit_evt, pit_event_source)?;
        Ok(chip)
    }

    /// Sends a Message Signaled Interrupt to one or more APICs. The WHPX API does not accept the
    /// MSI address and data directly, so we must parse them and supply WHPX with the vector,
    /// destination id, destination mode, trigger mode, and delivery mode (aka interrupt type).
    fn send_msi(&self, addr: u32, data: u32) -> Result<()> {
        let mut msi_addr = MsiAddressMessage::new();
        msi_addr.set(0, 32, addr as u64);
        let dest = InterruptDestination::try_from(&msi_addr).or(Err(Error::new(libc::EINVAL)))?;

        let mut msi_data = MsiDataMessage::new();
        msi_data.set(0, 32, data as u64);
        let data = InterruptData::from(&msi_data);

        self.vm.request_interrupt(
            data.vector,
            dest.dest_id,
            dest.mode,
            data.trigger,
            data.delivery,
        )
    }

    /// Return true if there is a pending interrupt for the specified vcpu. For WhpxSplitIrqChip
    /// this calls interrupt_requested on the pic.
    fn interrupt_requested(&self, vcpu_id: usize) -> bool {
        // Pic interrupts for the split irqchip only go to vcpu 0
        if vcpu_id != 0 {
            return false;
        }
        self.pic.lock().interrupt_requested()
    }

    /// Check if the specified vcpu has any pending interrupts. Returns None for no interrupts,
    /// otherwise Some(u32) should be the injected interrupt vector. For WhpxSplitIrqChip
    /// this calls get_external_interrupt on the pic.
    fn get_external_interrupt(&self, vcpu_id: usize) -> Result<Option<u32>> {
        // Pic interrupts for the split irqchip only go to vcpu 0
        if vcpu_id != 0 {
            return Ok(None);
        }
        if let Some(vector) = self.pic.lock().get_external_interrupt() {
            Ok(Some(vector as u32))
        } else {
            Ok(None)
        }
    }
}

impl WhpxSplitIrqChip {
    fn register_irq_event(
        &mut self,
        irq: u32,
        irq_event: &Event,
        resample_event: Option<&Event>,
        source: IrqEventSource,
    ) -> Result<Option<usize>> {
        let mut evt = IrqEvent {
            gsi: irq,
            event: irq_event.try_clone()?,
            resample_event: None,
            source,
        };

        if let Some(resample_event) = resample_event {
            evt.resample_event = Some(resample_event.try_clone()?);
        }

        let mut irq_events = self.irq_events.lock();
        let index = irq_events.len();
        irq_events.push(Some(evt));
        Ok(Some(index))
    }

    fn unregister_irq_event(&mut self, irq: u32, irq_event: &Event) -> Result<()> {
        let mut irq_events = self.irq_events.lock();
        for (index, evt) in irq_events.iter().enumerate() {
            if let Some(evt) = evt {
                if evt.gsi == irq && irq_event.eq(&evt.event) {
                    irq_events[index] = None;
                    break;
                }
            }
        }
        Ok(())
    }
}

/// This IrqChip only works with Whpx so we only implement it for WhpxVcpu.
impl IrqChip for WhpxSplitIrqChip {
    fn add_vcpu(&mut self, _vcpu_id: usize, _vcpu: &dyn Vcpu) -> Result<()> {
        // The WHPX API acts entirely on the VM partition, so we don't need to keep references to
        // the vcpus.
        Ok(())
    }

    fn register_edge_irq_event(
        &mut self,
        irq: u32,
        irq_event: &IrqEdgeEvent,
        source: IrqEventSource,
    ) -> Result<Option<IrqEventIndex>> {
        self.register_irq_event(irq, irq_event.get_trigger(), None, source)
    }

    fn unregister_edge_irq_event(&mut self, irq: u32, irq_event: &IrqEdgeEvent) -> Result<()> {
        self.unregister_irq_event(irq, irq_event.get_trigger())
    }

    fn register_level_irq_event(
        &mut self,
        irq: u32,
        irq_event: &IrqLevelEvent,
        source: IrqEventSource,
    ) -> Result<Option<IrqEventIndex>> {
        self.register_irq_event(
            irq,
            irq_event.get_trigger(),
            Some(irq_event.get_resample()),
            source,
        )
    }

    fn unregister_level_irq_event(&mut self, irq: u32, irq_event: &IrqLevelEvent) -> Result<()> {
        self.unregister_irq_event(irq, irq_event.get_trigger())
    }

    fn route_irq(&mut self, route: IrqRoute) -> Result<()> {
        self.routes.lock().add(route)
    }

    fn set_irq_routes(&mut self, routes: &[IrqRoute]) -> Result<()> {
        self.routes.lock().replace_all(routes)
    }

    fn irq_event_tokens(&self) -> Result<Vec<(IrqEventIndex, IrqEventSource, Event)>> {
        let mut tokens: Vec<(IrqEventIndex, IrqEventSource, Event)> = Vec::new();
        for (index, evt) in self.irq_events.lock().iter().enumerate() {
            if let Some(evt) = evt {
                tokens.push((index, evt.source.clone(), evt.event.try_clone()?));
            }
        }
        Ok(tokens)
    }

    fn service_irq(&mut self, irq: u32, level: bool) -> Result<()> {
        for route in self.routes.lock()[irq as usize].iter() {
            match *route {
                IrqSource::Irqchip {
                    chip: IrqSourceChip::PicPrimary,
                    pin,
                }
                | IrqSource::Irqchip {
                    chip: IrqSourceChip::PicSecondary,
                    pin,
                } => {
                    self.pic.lock().service_irq(pin as u8, level);
                }
                IrqSource::Irqchip {
                    chip: IrqSourceChip::Ioapic,
                    pin,
                } => {
                    self.ioapic.lock().service_irq(pin as usize, level);
                }
                // service_irq's level parameter is ignored for MSIs.  MSI data specifies the level.
                IrqSource::Msi { address, data } => self.send_msi(address as u32, data)?,
                _ => {
                    error!("Unexpected route source {:?}", route);
                    return Err(Error::new(libc::EINVAL));
                }
            }
        }
        Ok(())
    }

    /// Services an IRQ event by asserting then deasserting an IRQ line.  The associated Event
    /// that triggered the irq event will be read from.  If the irq is associated with a resample
    /// Event, then the deassert will only happen after an EOI is broadcast for a vector
    /// associated with the irq line.
    /// For WhpxSplitIrqChip, this function identifies the destination(s) of the irq: PIC, IOAPIC,
    /// or APIC (MSI).  If it's a PIC or IOAPIC route, we attempt to call service_irq on those
    /// chips.  If the IOAPIC is unable to be immediately locked, we add the irq to the
    /// delayed_ioapic_irq_events (though we still read from the Event that triggered the irq
    /// event).  If it's an MSI route, we call send_msi to decode the MSI and send the interrupt
    /// to WHPX.
    fn service_irq_event(&mut self, event_index: IrqEventIndex) -> Result<()> {
        let irq_events = self.irq_events.lock();
        let evt = if let Some(evt) = &irq_events[event_index] {
            evt
        } else {
            return Ok(());
        };
        evt.event.read()?;

        for route in self.routes.lock()[evt.gsi as usize].iter() {
            match *route {
                IrqSource::Irqchip {
                    chip: IrqSourceChip::PicPrimary,
                    pin,
                }
                | IrqSource::Irqchip {
                    chip: IrqSourceChip::PicSecondary,
                    pin,
                } => {
                    let mut pic = self.pic.lock();
                    if evt.resample_event.is_some() {
                        pic.service_irq(pin as u8, true);
                    } else {
                        pic.service_irq(pin as u8, true);
                        pic.service_irq(pin as u8, false);
                    }
                }
                IrqSource::Irqchip {
                    chip: IrqSourceChip::Ioapic,
                    pin,
                } => {
                    if let Ok(mut ioapic) = self.ioapic.try_lock() {
                        if evt.resample_event.is_some() {
                            ioapic.service_irq(pin as usize, true);
                        } else {
                            ioapic.service_irq(pin as usize, true);
                            ioapic.service_irq(pin as usize, false);
                        }
                    } else {
                        let mut delayed_events = self.delayed_ioapic_irq_events.lock();
                        delayed_events.events.push(event_index);
                        delayed_events.trigger.write(1)?;
                    }
                }
                IrqSource::Msi { address, data } => self.send_msi(address as u32, data)?,
                _ => {
                    error!("Unexpected route source {:?}", route);
                    return Err(Error::new(libc::EINVAL));
                }
            }
        }

        Ok(())
    }

    /// Broadcasts an end of interrupt. For WhpxSplitIrqChip this sends the EOI to the Ioapic.
    fn broadcast_eoi(&self, vector: u8) -> Result<()> {
        self.ioapic.lock().end_of_interrupt(vector);
        Ok(())
    }

    /// Injects any pending interrupts for `vcpu`.
    /// For WhpxSplitIrqChip this injects any PIC interrupts on vcpu_id 0.
    fn inject_interrupts(&self, vcpu: &dyn Vcpu) -> Result<()> {
        let vcpu: &WhpxVcpu = vcpu
            .downcast_ref()
            .expect("WhpxSplitIrqChip::add_vcpu called with non-WhpxVcpu");
        let vcpu_id = vcpu.id();
        if !self.interrupt_requested(vcpu_id) || !vcpu.ready_for_interrupt() {
            return Ok(());
        }

        if let Some(vector) = self.get_external_interrupt(vcpu_id)? {
            vcpu.interrupt(vector as u32)?;
        }

        // The second interrupt request should be handled immediately, so ask vCPU to exit as soon as
        // possible.
        if self.interrupt_requested(vcpu_id) {
            vcpu.set_interrupt_window_requested(true);
        }
        Ok(())
    }

    /// Notifies the irq chip that the specified VCPU has executed a halt instruction.
    /// For WhpxSplitIrqChip this is a no-op because Whpx handles VCPU blocking.
    fn halted(&self, _vcpu_id: usize) {}

    /// Blocks until `vcpu` is in a runnable state or until interrupted by
    /// `IrqChip::kick_halted_vcpus`.  Returns `VcpuRunState::Runnable if vcpu is runnable, or
    /// `VcpuRunState::Interrupted` if the wait was interrupted.
    /// For WhpxSplitIrqChip this is a no-op and always returns Runnable because Whpx handles VCPU
    /// blocking.
    fn wait_until_runnable(&self, _vcpu: &dyn Vcpu) -> Result<VcpuRunState> {
        Ok(VcpuRunState::Runnable)
    }

    /// Makes unrunnable VCPUs return immediately from `wait_until_runnable`.
    /// For WhpxSplitIrqChip this is a no-op because Whpx handles VCPU blocking.
    fn kick_halted_vcpus(&self) {}

    fn get_mp_state(&self, _vcpu_id: usize) -> Result<MPState> {
        // WHPX does not seem to have an API for this, but luckily this API isn't used anywhere
        // except the plugin.
        Err(Error::new(libc::ENXIO))
    }

    fn set_mp_state(&mut self, _vcpu_id: usize, _state: &MPState) -> Result<()> {
        // WHPX does not seem to have an API for this, but luckily this API isn't used anywhere
        // except the plugin.
        Err(Error::new(libc::ENXIO))
    }

    fn try_clone(&self) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(WhpxSplitIrqChip {
            vm: self.vm.try_clone()?,
            routes: self.routes.clone(),
            pit: self.pit.clone(),
            pic: self.pic.clone(),
            ioapic: self.ioapic.clone(),
            ioapic_pins: self.ioapic_pins,
            delayed_ioapic_irq_events: self.delayed_ioapic_irq_events.clone(),
            irq_events: self.irq_events.clone(),
        })
    }

    fn finalize_devices(
        &mut self,
        resources: &mut SystemAllocator,
        io_bus: &Bus,
        mmio_bus: &Bus,
    ) -> Result<()> {
        // Insert pit into io_bus
        io_bus.insert(self.pit.clone(), 0x040, 0x8).unwrap();
        io_bus.insert(self.pit.clone(), 0x061, 0x1).unwrap();

        // Insert pic into io_bus
        io_bus.insert(self.pic.clone(), 0x20, 0x2).unwrap();
        io_bus.insert(self.pic.clone(), 0xa0, 0x2).unwrap();
        io_bus.insert(self.pic.clone(), 0x4d0, 0x2).unwrap();

        // Insert ioapic into mmio_bus
        mmio_bus
            .insert(
                self.ioapic.clone(),
                IOAPIC_BASE_ADDRESS,
                IOAPIC_MEM_LENGTH_BYTES,
            )
            .unwrap();

        // At this point, all of our devices have been created and they have registered their
        // irq events, so we can clone our resample events
        let mut ioapic_resample_events: Vec<Vec<Event>> =
            (0..self.ioapic_pins).map(|_| Vec::new()).collect();
        let mut pic_resample_events: Vec<Vec<Event>> =
            (0..self.ioapic_pins).map(|_| Vec::new()).collect();

        for evt in self.irq_events.lock().iter().flatten() {
            if (evt.gsi as usize) >= self.ioapic_pins {
                continue;
            }
            if let Some(resample_evt) = &evt.resample_event {
                ioapic_resample_events[evt.gsi as usize].push(resample_evt.try_clone()?);
                pic_resample_events[evt.gsi as usize].push(resample_evt.try_clone()?);
            }
        }

        // Register resample events with the ioapic
        self.ioapic
            .lock()
            .register_resample_events(ioapic_resample_events);
        // Register resample events with the pic
        self.pic
            .lock()
            .register_resample_events(pic_resample_events);

        // Make sure all future irq numbers are >= self.ioapic_pins
        let mut irq_num = resources.allocate_irq().unwrap();
        while irq_num < self.ioapic_pins as u32 {
            irq_num = resources.allocate_irq().unwrap();
        }

        Ok(())
    }

    fn process_delayed_irq_events(&mut self) -> Result<()> {
        let irq_events = self.irq_events.lock();
        let mut delayed_events = self.delayed_ioapic_irq_events.lock();
        delayed_events.events.retain(|&event_index| {
            if let Some(evt) = &irq_events[event_index] {
                if let Ok(mut ioapic) = self.ioapic.try_lock() {
                    if evt.resample_event.is_some() {
                        ioapic.service_irq(evt.gsi as usize, true);
                    } else {
                        ioapic.service_irq(evt.gsi as usize, true);
                        ioapic.service_irq(evt.gsi as usize, false);
                    }

                    false
                } else {
                    true
                }
            } else {
                true
            }
        });

        if delayed_events.events.is_empty() {
            delayed_events.trigger.read()?;
        }
        Ok(())
    }

    fn irq_delayed_event_token(&self) -> Result<Option<Event>> {
        Ok(Some(
            self.delayed_ioapic_irq_events.lock().trigger.try_clone()?,
        ))
    }

    fn check_capability(&self, c: IrqChipCap) -> bool {
        match c {
            // It appears as though WHPX does not have tsc deadline support because we get guest
            // MSR write failures if we enable it.
            IrqChipCap::TscDeadlineTimer => false,
            // TODO(b/180966070): Figure out how to query x2apic support.
            IrqChipCap::X2Apic => false,
        }
    }
}

impl IrqChipX86_64 for WhpxSplitIrqChip {
    fn try_box_clone(&self) -> Result<Box<dyn IrqChipX86_64>> {
        Ok(Box::new(self.try_clone()?))
    }

    fn as_irq_chip(&self) -> &dyn IrqChip {
        self
    }

    fn as_irq_chip_mut(&mut self) -> &mut dyn IrqChip {
        self
    }

    /// Get the current state of the PIC
    fn get_pic_state(&self, select: PicSelect) -> Result<PicState> {
        Ok(self.pic.lock().get_pic_state(select))
    }

    /// Set the current state of the PIC
    fn set_pic_state(&mut self, select: PicSelect, state: &PicState) -> Result<()> {
        self.pic.lock().set_pic_state(select, state);
        Ok(())
    }

    /// Get the current state of the IOAPIC
    fn get_ioapic_state(&self) -> Result<IoapicState> {
        Ok(self.ioapic.lock().get_ioapic_state())
    }

    /// Set the current state of the IOAPIC
    fn set_ioapic_state(&mut self, state: &IoapicState) -> Result<()> {
        self.ioapic.lock().set_ioapic_state(state);
        Ok(())
    }

    /// Get the current state of the specified VCPU's local APIC
    fn get_lapic_state(&self, vcpu_id: usize) -> Result<LapicState> {
        self.vm.get_vcpu_lapic_state(vcpu_id)
    }

    /// Set the current state of the specified VCPU's local APIC
    fn set_lapic_state(&mut self, vcpu_id: usize, state: &LapicState) -> Result<()> {
        self.vm.set_vcpu_lapic_state(vcpu_id, state)
    }

    fn lapic_frequency(&self) -> u32 {
        WHPX_LOCAL_APIC_EMULATION_APIC_FREQUENCY
    }

    /// Retrieves the state of the PIT.
    fn get_pit(&self) -> Result<PitState> {
        Ok(self.pit.lock().get_pit_state())
    }

    /// Sets the state of the PIT.
    fn set_pit(&mut self, state: &PitState) -> Result<()> {
        self.pit.lock().set_pit_state(state);
        Ok(())
    }

    /// Returns true if the PIT uses port 0x61 for the PC speaker, false if 0x61 is unused.
    /// devices::Pit uses 0x61.
    fn pit_uses_speaker_port(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::*;
    use super::*;

    use crate::pci::CrosvmDeviceId;
    use crate::DeviceId;
    use base::EventReadResult;
    use hypervisor::whpx::{Whpx, WhpxFeature};
    use hypervisor::{CpuId, IoapicRedirectionTableEntry, PitRWMode, TriggerMode, VmX86_64};
    use resources::{AddressRange, SystemAllocatorConfig};
    use vm_memory::{GuestAddress, GuestMemory};

    fn split_supported() -> bool {
        Whpx::check_whpx_feature(WhpxFeature::LocalApicEmulation)
            .expect("failed to get whpx features")
    }

    /// Helper function for setting up a WhpxSplitIrqChip.
    fn get_chip(num_vcpus: usize) -> WhpxSplitIrqChip {
        let whpx = Whpx::new().expect("failed to instantiate Whpx");
        let mem = GuestMemory::new(&[(GuestAddress(0), 0x10000)]).unwrap();
        let vm = WhpxVm::new(&whpx, num_vcpus, mem, CpuId::new(0), true)
            .expect("failed tso instantiate vm");

        let (_, irq_tube) = Tube::pair().expect("failed to create irq tube");

        let mut chip =
            WhpxSplitIrqChip::new(vm.try_clone().expect("failed to clone vm"), irq_tube, None)
                .expect("failed to instantiate WhpxSplitIrqChip");

        for i in 0..num_vcpus {
            let vcpu = vm.create_vcpu(i).expect("failed to instantiate vcpu");
            chip.add_vcpu(i, vcpu.as_vcpu())
                .expect("failed to add vcpu");
        }

        chip
    }

    #[test]
    fn set_pic() {
        if !split_supported() {
            return;
        }
        test_set_pic(get_chip(1));
    }

    #[test]
    fn get_ioapic() {
        if !split_supported() {
            return;
        }
        test_get_ioapic(get_chip(1));
    }

    #[test]
    fn set_ioapic() {
        if !split_supported() {
            return;
        }
        test_set_ioapic(get_chip(1));
    }

    #[test]
    fn get_pit() {
        if !split_supported() {
            return;
        }
        test_get_pit(get_chip(1));
    }

    #[test]
    fn set_pit() {
        if !split_supported() {
            return;
        }
        test_set_pit(get_chip(1));
    }

    #[test]
    fn route_irq() {
        if !split_supported() {
            return;
        }
        test_route_irq(get_chip(1));
    }

    #[test]
    fn pit_uses_speaker_port() {
        if !split_supported() {
            return;
        }
        let chip = get_chip(1);
        assert!(chip.pit_uses_speaker_port());
    }

    #[test]
    fn routes_conflict() {
        if !split_supported() {
            return;
        }
        let mut chip = get_chip(1);
        chip.route_irq(IrqRoute {
            gsi: 32,
            source: IrqSource::Msi {
                address: 4276092928,
                data: 0,
            },
        })
        .expect("failed to set msi route");
        // this second route should replace the first
        chip.route_irq(IrqRoute {
            gsi: 32,
            source: IrqSource::Msi {
                address: 4276092928,
                data: 32801,
            },
        })
        .expect("failed to set msi route");
    }

    #[test]
    fn irq_event_tokens() {
        if !split_supported() {
            return;
        }
        let mut chip = get_chip(1);
        let tokens = chip
            .irq_event_tokens()
            .expect("could not get irq_event_tokens");

        // there should be one token on a fresh split irqchip, for the pit
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].1.device_name, "userspace PIT");

        // register another irq event
        let evt = IrqEdgeEvent::new().expect("failed to create event");
        let source = IrqEventSource {
            device_id: CrosvmDeviceId::DebugConsole.into(),
            queue_id: 0,
            device_name: "test".to_owned(),
        };
        chip.register_edge_irq_event(6, &evt, source)
            .expect("failed to register irq event");

        let tokens = chip
            .irq_event_tokens()
            .expect("could not get irq_event_tokens");

        // now there should be two tokens
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].1.device_name, "userspace PIT");
        assert_eq!(
            tokens[1].1.device_id,
            DeviceId::PlatformDeviceId(CrosvmDeviceId::DebugConsole)
        );
        assert_eq!(tokens[1].2, evt.get_trigger().try_clone().unwrap());
    }

    // TODO(srichman): Factor out of UserspaceIrqChip and KvmSplitIrqChip and WhpxSplitIrqChip.
    #[test]
    fn finalize_devices() {
        if !split_supported() {
            return;
        }
        let mut chip = get_chip(1);

        let mut mmio_bus = Bus::new();
        let mut io_bus = Bus::new();
        let mut resources = SystemAllocator::new(
            SystemAllocatorConfig {
                io: Some(AddressRange {
                    start: 0xc000,
                    end: 0xFFFF,
                }),
                low_mmio: AddressRange {
                    start: 0,
                    end: 2048,
                },
                high_mmio: AddressRange {
                    start: 2048,
                    end: 6143,
                },
                platform_mmio: None,
                first_irq: 5,
            },
            None,
            &[],
        )
        .expect("failed to create SystemAllocator");

        // setup an event for irq line 1
        let evt = IrqLevelEvent::new().expect("failed to create event");

        let source = IrqEventSource {
            device_id: CrosvmDeviceId::DebugConsole.into(),
            device_name: "test".to_owned(),
            queue_id: 0,
        };

        let evt_index = chip
            .register_level_irq_event(1, &evt, source)
            .expect("failed to register_level_irq_event")
            .expect("register_level_irq_event should not return None");

        // Once we finalize devices, the pic/pit/ioapic should be attached to io and mmio busses
        chip.finalize_devices(&mut resources, &io_bus, &mmio_bus)
            .expect("failed to finalize devices");

        // Should not be able to allocate an irq < 24 now
        assert!(resources.allocate_irq().expect("failed to allocate irq") >= 24);

        // set PIT counter 2 to "SquareWaveGen"(aka 3) mode and "Both" access mode
        io_bus.write(0x43, &[0b10110110]);

        let state = chip.get_pit().expect("failed to get pit state");
        assert_eq!(state.channels[2].mode, 3);
        assert_eq!(state.channels[2].rw_mode, PitRWMode::Both);

        // ICW1 0x11: Edge trigger, cascade mode, ICW4 needed.
        // ICW2 0x08: Interrupt vector base address 0x08.
        // ICW3 0xff: Value written does not matter.
        // ICW4 0x13: Special fully nested mode, auto EOI.
        io_bus.write(0x20, &[0x11]);
        io_bus.write(0x21, &[0x08]);
        io_bus.write(0x21, &[0xff]);
        io_bus.write(0x21, &[0x13]);

        let state = chip
            .get_pic_state(PicSelect::Primary)
            .expect("failed to get pic state");

        // auto eoi and special fully nested mode should be turned on
        assert!(state.auto_eoi);
        assert!(state.special_fully_nested_mode);

        // Need to write to the irq event before servicing it
        evt.trigger().expect("failed to write to event");

        // if we assert irq line one, and then get the resulting interrupt, an auto-eoi should
        // occur and cause the resample_event to be written to
        chip.service_irq_event(evt_index)
            .expect("failed to service irq");

        assert!(chip.interrupt_requested(0));
        assert_eq!(
            chip.get_external_interrupt(0)
                .expect("failed to get external interrupt"),
            // Vector is 9 because the interrupt vector base address is 0x08 and this is irq
            // line 1 and 8+1 = 9
            Some(0x9)
        );

        assert_eq!(
            evt.get_resample()
                .read_timeout(std::time::Duration::from_secs(1))
                .expect("failed to read_timeout"),
            EventReadResult::Count(1)
        );

        // setup a ioapic redirection table entry 14
        let mut entry = IoapicRedirectionTableEntry::default();
        entry.set_vector(44);

        let irq_14_offset = 0x10 + 14 * 2;
        mmio_bus.write(IOAPIC_BASE_ADDRESS, &[irq_14_offset]);
        mmio_bus.write(
            IOAPIC_BASE_ADDRESS + 0x10,
            &(entry.get(0, 32) as u32).to_ne_bytes(),
        );
        mmio_bus.write(IOAPIC_BASE_ADDRESS, &[irq_14_offset + 1]);
        mmio_bus.write(
            IOAPIC_BASE_ADDRESS + 0x10,
            &(entry.get(32, 32) as u32).to_ne_bytes(),
        );

        let state = chip.get_ioapic_state().expect("failed to get ioapic state");

        // redirection table entry 14 should have a vector of 44
        assert_eq!(state.redirect_table[14].get_vector(), 44);
    }

    // TODO(srichman): Factor out of UserspaceIrqChip and KvmSplitIrqChip and WhpxSplitIrqChip.
    #[test]
    fn broadcast_eoi() {
        if !split_supported() {
            return;
        }
        let mut chip = get_chip(1);

        let mut mmio_bus = Bus::new();
        let io_bus = Bus::new();
        let mut resources = SystemAllocator::new(
            SystemAllocatorConfig {
                io: Some(AddressRange {
                    start: 0xc000,
                    end: 0xFFFF,
                }),
                low_mmio: AddressRange {
                    start: 0,
                    end: 2048,
                },
                high_mmio: AddressRange {
                    start: 2048,
                    end: 6143,
                },
                platform_mmio: None,
                first_irq: 5,
            },
            None,
            &[],
        )
        .expect("failed to create SystemAllocator");

        // setup an event for irq line 1
        let evt = IrqLevelEvent::new().expect("failed to create event");

        let source = IrqEventSource {
            device_id: CrosvmDeviceId::DebugConsole.into(),
            device_name: "test".to_owned(),
            queue_id: 0,
        };

        chip.register_level_irq_event(1, &evt, source)
            .expect("failed to register_level_irq_event");

        // Once we finalize devices, the pic/pit/ioapic should be attached to io and mmio busses
        chip.finalize_devices(&mut resources, &io_bus, &mmio_bus)
            .expect("failed to finalize devices");

        // setup a ioapic redirection table entry 1 with a vector of 123
        let mut entry = IoapicRedirectionTableEntry::default();
        entry.set_vector(123);
        entry.set_trigger_mode(TriggerMode::Level);

        let irq_write_offset = 0x10 + 1 * 2;
        mmio_bus.write(IOAPIC_BASE_ADDRESS, &[irq_write_offset]);
        mmio_bus.write(
            IOAPIC_BASE_ADDRESS + 0x10,
            &(entry.get(0, 32) as u32).to_ne_bytes(),
        );
        mmio_bus.write(IOAPIC_BASE_ADDRESS, &[irq_write_offset + 1]);
        mmio_bus.write(
            IOAPIC_BASE_ADDRESS + 0x10,
            &(entry.get(32, 32) as u32).to_ne_bytes(),
        );

        // Assert line 1
        chip.service_irq(1, true).expect("failed to service irq");

        // resample event should not be written to
        assert_eq!(
            evt.get_resample()
                .read_timeout(std::time::Duration::from_millis(10))
                .expect("failed to read_timeout"),
            EventReadResult::Timeout
        );

        // irq line 1 should be asserted
        let state = chip.get_ioapic_state().expect("failed to get ioapic state");
        assert_eq!(state.current_interrupt_level_bitmap, 1 << 1);

        // Now broadcast an eoi for vector 123
        chip.broadcast_eoi(123).expect("failed to broadcast eoi");

        // irq line 1 should be deasserted
        let state = chip.get_ioapic_state().expect("failed to get ioapic state");
        assert_eq!(state.current_interrupt_level_bitmap, 0);

        // resample event should be written to by ioapic
        assert_eq!(
            evt.get_resample()
                .read_timeout(std::time::Duration::from_millis(10))
                .expect("failed to read_timeout"),
            EventReadResult::Count(1)
        );
    }
}
