# Virtio Vhost-User device (VVU)

Crosvm also supports the [virtio vhost-user (VVU)] device to run a vhost-user device back-end inside
of another VM's guest. The following diagram shows how VVU works for virtio-block.

<!-- Image from https://docs.google.com/presentation/d/1s6wH5L_F8NNiXls5UgWbD34jtBmijoZuiyLu76Fc2NM/edit#slide=id.g12aad4d534e_0_4 -->

![vvu diagram](images/vvu.png)

The "virtio vhost-user device", which is also called "vvu-proxy", is a virtio PCI device that works
as a proxy of vhost-user messages between the vhost-user device back-end in the guest of a VM
(device VM) and the vhost-user front-end in another VM (sibling VM).

## How to run

Let's take a block device as an example and see how to start VVU devices.

First, start a device VM with a usual `crosvm run` command. At this time, put a crosvm binary in the
guest in some way. (e.g. putting it in a disk, sharing the host's crosvm with virtiofs, building
crosvm in the guest, etc)

```sh
# On the host.

VHOST_USER_SOCK=/tmp/vhost-user.socket

# Specify the PCI address that the VVU proxy device will be allocated.
# If you don't pass `addr=` as an argument of `--vvu-proxy` below, crosvm will
# allocate it to the first available address.
VVU_PCI_ADDR="0000:00:10.0"

# Start the device VM.
crosvm run \
  --vvu-proxy "${VHOST_USER_SOCK},addr=${VVU_PCI_ADDR}" \
  ... # usual crosvm args
  vmlinux
```

Then you can check that the VVU proxy device is allocated at the specified address by running
`lspci` in the guest.

```sh
# Inside of the device VM guet.

lspci -s $VVU_PCI_ADDR
# Expected output:
# > 00:10.0 Unclassified device [00ff]: Red Hat, Inc. Device 107d (rev 01)
# '107d' is the device ID for the VVU proxy device.
```

Second, start a VVU block device backend in the guest that you just started. Although the command
`crosvm device` is the same as [vhost-user's example](./vhost_user.md), you need to use the `--vfio`
flag instead of the `--socket` flag.

```sh
# Inside of the device VM guest

crosvm device block \
  --vfio ${VVU_PCI_ADDR} \
  --file disk.img
```

Finally, open another terminal and start a vmm process with `--vhost-user-blk` flag on the host.

```sh
# On the host, start a sibling VM. This can be done in the same way as the vhost-user block front-end.

crosvm run \
  --vhost-user-blk ${VHOST_USER_SOCK} \
  ... # usual crosvm args
  vmlinux
```

As a result, `disk.img` in the device VM should be exposed as `/dev/vda` in the guest of the sibling
VM.

[virtio vhost-user (vvu)]: https://wiki.qemu.org/Features/VirtioVhostUser
