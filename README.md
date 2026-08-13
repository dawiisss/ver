# VER - Very Easy Remote

VER is a modern, native Linux Remote Connection Manager built with **Rust**, **GTK4**, and **Libadwaita**. It provides a beautiful, seamless, and blazingly fast experience for managing and connecting to your remote machines via RDP, VNC, SSH, and Spice.

## Features

- **Multi-Protocol Support**: First-class support for RDP (via `xfreerdp3`), SSH (via terminal emulators), Spice, and a highly performant **Custom Native VNC Client** written purely in Rust.
- **External Session Tracker**: A built-in live log viewer that captures standard output and standard error from external connectors like `xfreerdp3` natively in the app, giving you clear insights into connection failures.
- **Modern Interface**: A gorgeous Libadwaita interface for managing saved connections, connection groups, and advanced protocol settings (Color Depth, Gateways, Audio Redirection, etc).
- **Network Auto-Discovery**: Automatically scan your local network for active SSH and VNC servers using mDNS/Avahi (zeroconf) and Subnet Sweeping.
- **Global Preferences**: Override system themes to force Dark or Light mode seamlessly.
- **Secure Password Storage**: Passwords are encrypted and automatically securely stored in your OS's native keyring (Secret Service API).

## Requirements

Ensure you have the following dependencies installed on your system to build and run VER.

### Build Dependencies (Debian/Ubuntu)
```bash
sudo apt update
sudo apt install build-essential rustc cargo libgtk-4-dev libadwaita-1-dev
```
*Note: Arch Linux users should install `gtk4` and `libadwaita`.*

### Runtime Dependencies
- **RDP**: Requires `freerdp3-x11` (provides `xfreerdp3`) for external RDP launching.
- **SSH**: Uses your system's default terminal (e.g. `ptyxis`, `gnome-terminal`, `konsole`).
- **VNC**: Powered natively by Rust; no external packages required!

## Running Locally

To compile and run the application directly from the source code:
```bash
cargo run
```

## Packaging and Distribution

You can build a release binary using Cargo:
```bash
cargo build --release
```
The optimized binary will be located in `target/release/beautiful-goodall`. You can move this binary to `/usr/local/bin` to install it system-wide.
