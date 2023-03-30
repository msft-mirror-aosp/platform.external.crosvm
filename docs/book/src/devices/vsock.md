# Vsock device

crosvm supports [virtio-vsock] device for communication between the host and a guest VM.

Assign a context id to a guest VM by passing it with `--cid` flag.

```sh
GUEST_CID=3

crosvm run \
  --cid "${GUEST_CID}" \
  <usual crosvm arguments>
  /path/to/bzImage
```

Then, the guest and the host can communicate with each other via vsock. Host always has 2 as its
context id.

crosvm assumes that the host has a vsock device at `/dev/vhost-vsock`. If you want to use a device
at a different path or one given as an fd, you can use `--vhost-vsock-device` flag or
`--vhost-vsock-fd` flag respectively.

## Example usage

At host shell:

```sh
PORT=11111

# Listen at host
ncat -l --vsock ${PORT}
```

At guest shell:

```sh
HOST_CID=2
PORT=11111

# Make a connection to the host
ncat --vsock ${HOST_CID} ${PORT}
```

If a vsock device is configured properly in the guest VM, a connection between the host and the
guest can be established and packets can be sent from both side. In the above example, your inputs
to a shell on one's side should be shown at the shell on the other side if a connection is
successfully established.

[virtio-vsock]: https://docs.oasis-open.org/virtio/virtio/v1.1/csprd01/virtio-v1.1-csprd01.html#x1-389001r356
