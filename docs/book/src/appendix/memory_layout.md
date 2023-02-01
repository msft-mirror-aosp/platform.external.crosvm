# Memory Layout

## x86-64 guest physical memory map

This is a survey of the existing memory layout for crosvm on x86-64 when booting a Linux kernel. Some of these values are different when booting a BIOS image or when compiled with features=direct (ManaTEE); see the source. All addresses are in hexadecimal.

| Name/source link             | Address       | End (exclusive) | Size      | Notes                                                                                    |
| ---------------------------- | ------------- | --------------- | --------- | ---------------------------------------------------------------------------------------- |
|                              | `0000`        | `7000`          |           | RAM (may start at 0x1000 for crosvm-direct)                                              |
| [`ZERO_PAGE_OFFSET`]         | `7000`        |                 |           | Linux boot_params structure                                                              |
| [`BOOT_STACK_POINTER`]       | `8000`        |                 |           | Boot SP value                                                                            |
| [`boot_pml4_addr`]           | `9000`        |                 |           | Boot page table                                                                          |
| [`boot_pdpte_addr`]          | `A000`        |                 |           | Boot page table                                                                          |
| [`boot_pde_addr`]            | `B000`        |                 |           | Boot page table                                                                          |
| [`CMDLINE_OFFSET`]           | `2_0000`      | `20_0000`       | ~1.87 MiB | Linux kernel command line                                                                |
| [`ACPI_HI_RSDP_WINDOW_BASE`] | `E_0000`      |                 |           | ACPI RSDP table (TODO: technically overlaps command line buffer; check CMDLINE_MAX_SIZE) |
| [`KERNEL_START_OFFSET`]      | `20_0000`     |                 |           | Linux kernel image load address                                                          |
| [`END_ADDR_BEFORE_32BITS`]   | `20_0000`     | `D000_0000`     | ~3.24 GiB | RAM (\<4G)                                                                               |
| [`END_ADDR_BEFORE_32BITS`]   | `D000_0000`   | `F400_0000`     | 576 MiB   | Low (\<4G) MMIO allocation area                                                          |
| [`PCIE_CFG_MMIO_START`]      | `F400_0000`   | `F800_0000`     | 64 MiB    | PCIe enhanced config (ECAM)                                                              |
| [`RESERVED_MEM_SIZE`]        | `F800_0000`   | `1_0000_0000`   | 128 MiB   | LAPIC/IOAPIC/HPET/…                                                                      |
| [`TSS_ADDR`]                 | `FFFB_D000`   |                 |           | Boot task state segment                                                                  |
|                              | `1_0000_0000` |                 |           | RAM (>4G)                                                                                |
|                              | (end of RAM)  |                 |           | High (>4G) MMIO allocation area                                                          |

[`zero_page_offset`]: https://crsrc.org/o/src/platform/crosvm-upstream/x86_64/src/lib.rs;l=235?q=ZERO_PAGE_OFFSET
[`boot_stack_pointer`]: https://crsrc.org/o/src/platform/crosvm-upstream/x86_64/src/lib.rs;l=208?q=BOOT_STACK_POINTER
[`boot_pml4_addr`]: https://crsrc.org/o/src/platform/crosvm-upstream/x86_64/src/regs.rs;l=310?q=boot_pml4_addr
[`boot_pdpte_addr`]: https://crsrc.org/o/src/platform/crosvm-upstream/x86_64/src/regs.rs;l=311?q=boot_pdpte_addr
[`boot_pde_addr`]: https://crsrc.org/o/src/platform/crosvm-upstream/x86_64/src/regs.rs;l=312?q=boot_pde_addr
[`cmdline_offset`]: https://crsrc.org/o/src/platform/crosvm-upstream/x86_64/src/lib.rs;l=239?q=CMDLINE_OFFSET
[`acpi_hi_rsdp_window_base`]: https://crsrc.org/o/src/platform/crosvm-upstream/x86_64/src/lib.rs;l=252?q=ACPI_HI_RSDP_WINDOW_BASE
[`kernel_start_offset`]: https://crsrc.org/o/src/platform/crosvm-upstream/x86_64/src/lib.rs;l=238?q=KERNEL_START_OFFSET
[`end_addr_before_32bits`]: https://crsrc.org/o/src/platform/crosvm-upstream/x86_64/src/lib.rs;l=230?q=END_ADDR_BEFORE_32BITS
[`pcie_cfg_mmio_start`]: https://crsrc.org/o/src/platform/crosvm-upstream/x86_64/src/lib.rs;l=227?q=PCIE_CFG_MMIO_START
[`reserved_mem_size`]: https://crsrc.org/o/src/platform/crosvm-upstream/x86_64/src/lib.rs;l=224?q=RESERVED_MEM_SIZE
[`tss_addr`]: https://crsrc.org/o/src/platform/crosvm-upstream/x86_64/src/lib.rs;l=236?q=TSS_ADDR

## aarch64 guest physical memory map

All addresses are IPA in hexadecimal.

### Common layout

These apply for all boot modes.

| Name/source link                  | Address         | End (exclusive) | Size       | Notes                                                         |
| --------------------------------- | --------------- | --------------- | ---------- | ------------------------------------------------------------- |
| [`SERIAL_ADDR[3]`][serial_addr]   | `2e8`           | `2f0`           | 8 bytes    | Serial port MMIO                                              |
| [`SERIAL_ADDR[1]`][serial_addr]   | `2f8`           | `300`           | 8 bytes    | Serial port MMIO                                              |
| [`SERIAL_ADDR[2]`][serial_addr]   | `3e8`           | `3f0`           | 8 bytes    | Serial port MMIO                                              |
| [`SERIAL_ADDR[0]`][serial_addr]   | `3f8`           | `400`           | 8 bytes    | Serial port MMIO                                              |
| [`AARCH64_RTC_ADDR`]              | `2000`          | `3000`          | 4 KiB      | Real-time clock                                               |
| [`AARCH64_VMWDT_ADDR`]            | `3000`          | `4000`          | 4 KiB      | Watchdog device                                               |
| [`AARCH64_PCI_CFG_BASE`]          | `1_0000`        | `2_0000`        | 64 KiB     | PCI configuration (CAM)                                       |
| [`AARCH64_PVTIME_IPA_START`]      | `1f0_0000`      | `200_0000`      | 64 KiB     | Paravirtualized time                                          |
| [`AARCH64_MMIO_BASE`]             | `200_0000`      | `400_0000`      | 32 MiB     | Low MMIO allocation area                                      |
| [`AARCH64_GIC_CPUI_BASE`]         | `3ffd_0000`     | `3fff_0000`     | 128 KiB    | vGIC                                                          |
| [`AARCH64_GIC_DIST_BASE`]         | `3fff_0000`     | `4000_0000`     | 64 KiB     | vGIC                                                          |
| [`AARCH64_AXI_BASE`]              | `4000_0000`     |                 |            | Seemingly unused? Is this hard-coded somewhere in the kernel? |
| [`AARCH64_PROTECTED_VM_FW_START`] | `7fc0_0000`     | `8000_0000`     | 4 MiB      | pVM firmware (if running a protected VM)                      |
| [`AARCH64_PHYS_MEM_START`]        | `8000_0000`     |                 | --mem size | RAM (starts at IPA = 2 GiB)                                   |
| [`plat_mmio_base`]                | after RAM       | +0x800000       | 8 MiB      | Platform device MMIO region                                   |
| [`high_mmio_base`]                | after plat_mmio | max phys addr   |            | High MMIO allocation area                                     |

### Layout when booting a kernel

These apply when no bootloader is passed, so crosvm boots a kernel directly.

| Name/source link          | Address           | End (exclusive) | Size  | Notes                        |
| ------------------------- | ----------------- | --------------- | ----- | ---------------------------- |
| [`AARCH64_KERNEL_OFFSET`] | `8000_0000`       |                 |       | Kernel load location in RAM  |
| [`initrd_addr`]           | after kernel      |                 |       | Linux initrd location in RAM |
| [`fdt_address`]           | before end of RAM |                 | 2 MiB | Flattened device tree in RAM |

### Layout when booting a bootloader

These apply when a bootloader is passed with `--bios`.

| Name/source link                    | Address     | End (exclusive) | Size  | Notes                        |
| ----------------------------------- | ----------- | --------------- | ----- | ---------------------------- |
| [`AARCH64_FDT_OFFSET_IN_BIOS_MODE`] | `8000_0000` | `8020_0000`     | 2 MiB | Flattened device tree in RAM |
| [`AARCH64_BIOS_OFFSET`]             | `8020_0000` |                 |       | Bootloader image in RAM      |

[serial_addr]: https://crsrc.org/o/src/platform/crosvm-upstream/arch/src/serial.rs;l=70?q=SERIAL_ADDR
[`aarch64_rtc_addr`]: https://crsrc.org/o/src/platform/crosvm-upstream/aarch64/src/lib.rs;l=93?q=AARCH64_RTC_ADDR
[`aarch64_vmwdt_addr`]: https://crsrc.org/o/src/platform/crosvm-upstream/aarch64/src/lib.rs;l=93?q=AARCH64_VMWDT_ADDR
[`aarch64_pci_cfg_base`]: https://crsrc.org/o/src/platform/crosvm-upstream/aarch64/src/lib.rs;l=100?q=AARCH64_PCI_CFG_BASE
[`aarch64_mmio_base`]: https://crsrc.org/o/src/platform/crosvm-upstream/aarch64/src/lib.rs;l=104?q=AARCH64_MMIO_BASE
[`aarch64_gic_cpui_base`]: https://crsrc.org/o/src/platform/crosvm-upstream/devices/src/irqchip/kvm/aarch64.rs;l=44?q=AARCH64_GIC_CPUI_BASE
[`aarch64_gic_dist_base`]: https://crsrc.org/o/src/platform/crosvm-upstream/aarch64/src/lib.rs;l=64?q=AARCH64_GIC_DIST_BASE
[`aarch64_axi_base`]: https://crsrc.org/o/src/platform/crosvm-upstream/aarch64/src/lib.rs;l=45?q=AARCH64_AXI_BASE
[`aarch64_pvtime_ipa_start`]: https://crsrc.org/o/src/platform/crosvm-upstream/aarch64/src/lib.rs;l=59?q=AARCH64_PVTIME_IPA_START
[`aarch64_protected_vm_fw_start`]: https://crsrc.org/o/src/platform/crosvm-upstream/aarch64/src/lib.rs;l=55?q=AARCH64_PROTECTED_VM_FW_START
[`aarch64_phys_mem_start`]: https://crsrc.org/o/src/platform/crosvm-upstream/aarch64/src/lib.rs;l=44?q=AARCH64_PHYS_MEM_START
[`plat_mmio_base`]: https://crsrc.org/o/src/platform/crosvm-upstream/aarch64/src/lib.rs;l=551?q=plat_mmio_base
[`high_mmio_base`]: https://crsrc.org/o/src/platform/crosvm-upstream/aarch64/src/lib.rs;l=554?q=high_mmio_base
[`aarch64_kernel_offset`]: https://crsrc.org/o/src/platform/crosvm-upstream/aarch64/src/lib.rs;l=35?q=AARCH64_KERNEL_OFFSET
[`initrd_addr`]: https://crsrc.org/o/src/platform/crosvm-upstream/aarch64/src/lib.rs;l=270?q=initrd_addr
[`fdt_address`]: https://crsrc.org/o/src/platform/crosvm-upstream/aarch64/src/lib.rs;l=184?q=fdt_address
[`aarch64_fdt_offset_in_bios_mode`]: https://crsrc.org/o/src/platform/crosvm-upstream/aarch64/src/lib.rs;l=49?q=AARCH64_FDT_OFFSET_IN_BIOS_MODE
[`aarch64_bios_offset`]: https://crsrc.org/o/src/platform/crosvm-upstream/aarch64/src/lib.rs;l=51?q=AARCH64_BIOS_OFFSET
