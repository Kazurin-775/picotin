# picotin

A tiny Linux container host, with minimal supported features, written in Rust. (The name, "pico tin", is a synonym for "tiny container".)

## Build and usage

```sh
# Build
cargo install --path=.

# Obtain Ubuntu rootfs
wget 'https://cdimage.ubuntu.com/ubuntu-base/releases/focal/release/ubuntu-base-20.04.5-base-amd64.tar.gz'
sudo mkdir ubuntu
cd ubuntu
sudo tar xf '../ubuntu-base-20.04.5-base-amd64.tar.gz'

# Start the container!
sudo picotin new --root=.
```

## Features

- Minimal support for { user, pid, ipc, mount, net } namespace isolation
- CPU and memory quota via cgroups
- Point-to-point container intercommunication via veth pair
