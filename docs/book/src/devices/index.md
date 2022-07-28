# Devices

This chapter describes emulated devices in crosvm. These devices work like hardware for the guest.

## List of devices

Here is a (non-comprehensive) list of emulated devices provided by crosvm.

### Emulated Devices

- [`CMOS/RTC`] - Used to get the current calendar time.
- [`i8042`] - Used by the guest kernel to exit crosvm.
- [`serial`] - x86 I/O port driven serial devices that print to stdout and take input from stdin.

### VirtIO Devices

- [`balloon`] - Allows the host to reclaim the guest's memories.
- [`block`] - Basic read/write block device.
- [`console`] - Input and outputs on console.
- [`fs`] - Shares file systems over the FUSE protocol.
- [`gpu`] - Graphics adapter.
- [`input`] - Creates virtual human interface devices such as keyboards.
- [`iommu`] - Emulates an IOMMU device to manage DMA from endpoints in the guest.
- [`net`] - Device to interface the host and guest networks.
- [`p9`] - Shares file systems over the 9P protocol.
- [`pmem`] - Persistent memory.
- [`rng`] - Entropy source used to seed guest OS's entropy pool.
- [`snd`] - Encodes and decodes audio streams.
- [`tpm`] - Creates a TPM (Trusted Platform Module) device backed by libtpm2 simulator or vTPM
  daemon.
- [`video`] - Allows the guest to leverage the host's video capabilities.
- [`wayland`] - Allows the guest to use the host's Wayland socket.
- [`vsock`] - Enables use of virtual sockets for the guest.
- [`vhost-user`] - VirtIO devices which offloads the device implementation to another process
  through the [vhost-user protocol].
  - [vmm side]: Shares its virtqueues.
  - [device side]: Consumes virtqueues.

[device side]: https://chromium.googlesource.com/crosvm/crosvm/+/refs/heads/main/devices/src/virtio/vhost/user/device/
[vhost-user protocol]: https://qemu.readthedocs.io/en/latest/interop/vhost-user.html
[vmm side]: https://chromium.googlesource.com/crosvm/crosvm/+/refs/heads/main/devices/src/virtio/vhost/user/vmm/
[`balloon`]: balloon.md
[`block`]: block.md
[`cmos/rtc`]: https://chromium.googlesource.com/crosvm/crosvm/+/refs/heads/main/devices/src/cmos.rs
[`console`]: https://chromium.googlesource.com/crosvm/crosvm/+/refs/heads/main/devices/src/virtio/console.rs
[`fs`]: https://chromium.googlesource.com/crosvm/crosvm/+/refs/heads/main/devices/src/virtio/fs/
[`gpu`]: https://chromium.googlesource.com/crosvm/crosvm/+/refs/heads/main/devices/src/virtio/gpu/
[`i8042`]: https://chromium.googlesource.com/crosvm/crosvm/+/refs/heads/main/devices/src/i8042.rs
[`input`]: https://chromium.googlesource.com/crosvm/crosvm/+/refs/heads/main/devices/src/virtio/input/
[`iommu`]: https://chromium.googlesource.com/crosvm/crosvm/+/refs/heads/main/devices/src/virtio/iommu.rs
[`net`]: net.md
[`p9`]: https://chromium.googlesource.com/crosvm/crosvm/+/refs/heads/main/devices/src/virtio/p9.rs
[`pmem`]: pmem.md
[`rng`]: https://chromium.googlesource.com/crosvm/crosvm/+/refs/heads/main/devices/src/virtio/rng.rs
[`serial`]: https://chromium.googlesource.com/crosvm/crosvm/+/refs/heads/main/devices/src/serial.rs
[`snd`]: https://chromium.googlesource.com/crosvm/crosvm/+/refs/heads/main/devices/src/virtio/snd/
[`tpm`]: https://chromium.googlesource.com/crosvm/crosvm/+/refs/heads/main/devices/src/virtio/tpm.rs
[`vhost-user`]: vhost_user.md
[`video`]: video.md
[`vsock`]: https://chromium.googlesource.com/crosvm/crosvm/+/refs/heads/main/devices/src/virtio/vhost/vsock.rs
[`wayland`]: wayland.md
