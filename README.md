# VER - Very Easy Remote

VER is a modern, native Linux Remote Connection Manager built with **Rust**, **GTK4**, and **Libadwaita**. It provides a beautiful, seamless, and blazingly fast experience for managing and connecting to your remote machines via RDP, XRDP, VNC, SSH, and SPICE.

## Features

- **Multi-Protocol Support**: First-class support for:
  - **RDP & XRDP**: Integrated with `xfreerdp3` supporting dynamic resolution, audio/microphone redirection, clipboard sharing, gateway servers, shared folders, and performance tuning.
  - **VNC**: Integration with `vncviewer` with secure automatic credential management.
  - **SPICE**: High-performance SPICE connections via `remote-viewer`.
  - **SSH**: Direct terminal launching using your favorite desktop terminal emulator (`ptyxis`, `kgx`, `gnome-terminal`, `konsole`, `alacritty`, `xterm`).
- **External Session Tracker**: Built-in live log viewer capturing connector output with graceful POSIX signal termination and error reporting toasts.
- **Auto-Connect**: Option to automatically reconnect to the last used session immediately upon application startup.
- **Wake-on-LAN (WoL)**: Send magic packets directly from the connection editor to power on remote machines.
- **Network Auto-Discovery**: Rapidly scan your local subnet and local targets using a multi-threaded worker pool to detect open VNC, RDP, and SSH services.
- **Secure Keyring Storage**: Passwords are securely stored in your OS's native keyring via the Freedesktop Secret Service API with temporary file hardening (`0600` permissions).
- **Modern Libadwaita Interface & System Tray**: Full GNOME HIG compliance, searchable and group-sorted connection lists, adaptive Dark/Light theme switching, and background system tray integration.

## Requirements

### Build Dependencies

#### Debian / Ubuntu
```bash
sudo apt update
sudo apt install build-essential rustc cargo libgtk-4-dev libadwaita-1-dev
```

#### Arch Linux / Manjaro
```bash
sudo pacman -S base-devel rust gtk4 libadwaita
```

#### Fedora
```bash
sudo dnf install @development-tools rust cargo gtk4-devel libadwaita-devel
```

### Runtime Dependencies
- **RDP / XRDP**: `freerdp3` / `freerdp3-x11` (provides `xfreerdp3`)
- **VNC**: `tigervnc` or compatible `vncviewer`
- **SPICE**: `virt-viewer` (provides `remote-viewer`)
- **SSH**: Any standard terminal emulator (`ptyxis`, `gnome-terminal`, `konsole`, etc.)

## Running Locally

To compile and run the application directly from source:
```bash
cargo run
```

## Running Tests

To run the complete test suite:
```bash
cargo test
```

## Packaging and Distribution

Package scripts are provided in the repository:
- **Arch Linux (Pacman)**: `./build_pacman.sh`
- **Debian / Ubuntu (.deb)**: `./build_deb.sh`
- **RPM**: `./build_rpm.sh`
- **AppImage**: `./build_appimage.sh`

Or build an optimized binary with Cargo:
```bash
cargo build --release
```
The resulting executable will be in `target/release/ver`.

