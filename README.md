# VER - Very Easy Remote

VER is a modern, native Linux Remote Connection Manager built with Python, GTK4, and Libadwaita. It provides a beautiful and seamless experience for managing and connecting to your remote machines via RDP, VNC, and SSH.

## Features

- **Multi-Protocol Support**: First-class support for RDP (via `xfreerdp3`), VNC (via `vncviewer`), and SSH (embedded directly in the app using `Vte.Terminal`).
- **Tabbed Interface**: Manage your connections in one tab, while multiple SSH sessions live natively inside their own tabs within the same window.
- **Advanced Protocol Flags**: Easily toggle Multi-monitor, Fullscreen, and Audio redirection for RDP, or View-Only/Shared modes for VNC.
- **Wake-on-LAN (WoL)**: Send magic packets to wake up your servers directly from the app before connecting.
- **Network Auto-Discovery**: Automatically scan your local network for active SSH and VNC servers using mDNS/Avahi (zeroconf) and add them with one click.
- **Quick Connect Bar**: Instantly connect to a machine without saving it by typing a URI (e.g., `ssh://user@192.168.1.10`) into the top bar.
- **Global Preferences**: Override system themes to force Dark or Light mode seamlessly.
- **Secure Password Storage**: RDP passwords are automatically securely stored in your OS's native keyring.

## Requirements

Before running the application from source, ensure you have the following dependencies installed on your system:

### System Packages (Debian/Ubuntu)
```bash
sudo apt update
sudo apt install python3 python3-gi gir1.2-gtk-4.0 gir1.2-adw-1 gir1.2-vte-2.91-gtk4
```
*Note: Arch Linux users should install `vte4`.*

### Python Packages
```bash
pip install -r requirements.txt
```

### External Connectors
- **RDP**: Requires `freerdp3-x11` (provides `xfreerdp3`) or `freerdp2-x11`.
- **VNC**: Requires `tigervnc-viewer` or `tightvnc-java` (provides `vncviewer`).

## Running Locally

To run the application directly from the source code:
```bash
python3 src/app.py
```

## Packaging and Distribution

VER includes multiple scripts to easily package the application for distribution across various Linux environments.

### Debian (.deb)
To build a `.deb` package for Debian, Ubuntu, Pop!_OS, or Linux Mint:
```bash
bash build_deb.sh
```
This generates `ver_1.0.0_all.deb`, which can be installed via `sudo apt install ./ver_1.0.0_all.deb`.

### RPM (.rpm)
To build an `.rpm` package for Fedora, CentOS, or RHEL:
```bash
bash build_rpm.sh
```
*Note: This script requires the `alien` package to be installed on your host system.*

### AppImage
To generate a portable AppImage:
```bash
bash build_appimage.sh
```
This requires `appimagetool` to be available in your PATH.

### Flatpak
A flatpak manifest (`com.example.ver.json`) is provided. You can build it using `flatpak-builder`:
```bash
flatpak-builder build-dir com.example.ver.json --force-clean
flatpak-builder --user --install build-dir com.example.ver.json
```
