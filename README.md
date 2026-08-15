# VER - Very Easy Remote

[![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org)
[![GTK4](https://img.shields.io/badge/GTK-4.0-blue.svg)](https://www.gtk.org)
[![Libadwaita](https://img.shields.io/badge/Libadwaita-1.4+-purple.svg)](https://gnome.pages.gitlab.gnome.org/libadwaita/)
[![License: GPL-3.0](https://img.shields.io/badge/License-GPL3-green.svg)](https://www.gnu.org/licenses/gpl-3.0)

**VER (Very Easy Remote)** is a modern, native Linux Remote Connection Manager built from the ground up in **Rust**, **GTK4**, and **Libadwaita**. It provides a fast, secure, and intuitive interface for managing and connecting to remote infrastructure via RDP, XRDP, VNC, SSH, and SPICE.

---

## 🌟 Key Features

### ⚡ Quick Connect (<kbd>Ctrl+K</kbd>)
* **Universal URI & Shorthand Parsing**: Connect instantly using URIs (`ssh://user@host:port`, `rdp://admin@server`, `vnc://10.0.0.5:5901`, `spice://hypervisor:5900`) or shorthands (`user@host:2222`, `host:3389`, IPv6 `[fe80::1]:22`).
* **Live Autocomplete**: Protocol, host, port, and credentials auto-populate dynamically as you type.
* **Dual Execution Modes**: Launch immediately for one-off ad-hoc sessions ("Connect") or persist to your library ("Save & Connect").

### 🟢 Live Host Reachability Prober & Smart Wake-on-LAN
* **Non-Blocking Reachability Status**: Asynchronous background TCP probing displays real-time connection status in the sidebar:
  * 🟢 **Online**: Tooltip displays roundtrip latency in milliseconds (e.g. `Online (12 ms)`).
  * 🔴 **Offline**: Tooltip shows specific failure reasons (e.g. `Connection refused`, `Connection timed out`).
  * 🟡 **Probing**: Live verification in progress.
  * ⚪ **Unknown**: Pending initial probe.
* **Integrated Wake-on-LAN (WoL)**: Send magic packets directly from the Connection Editor.
* **Automated Post-WoL Polling**: Initiates a 30-second background polling cycle, automatically transitioning the status dot to 🟢 Online and triggering a desktop toast notification as soon as the target machine responds.

### 🔄 Import & Export Ecosystem
* **Remmina Migration**: Auto-scan `~/.local/share/remmina/*.remmina` profiles or import individual files with automatic extraction of display depths, audio redirection, multimon, and SSH keys.
* **OpenSSH `~/.ssh/config`**: Auto-detects and imports host blocks, hostnames, usernames, custom ports, and expanded identity files (`~/.ssh/id_*`).
* **Microsoft `.rdp` Support**: Seamless import and export of standard Windows Remote Desktop `.rdp` configuration files.
* **JSON Backups**: Full encrypted-password-safe JSON library backups with schema version metadata.
* **Selective Export (<kbd>Ctrl+E</kbd>)**: Granular multi-selection checklist with "Select All" / "Deselect All", dynamic counter, and smart focus pre-selection.
* **Conflict Resolution**: Choose between *Skip Duplicates*, *Overwrite Existing*, or *Keep Both (Rename with Suffix)* when importing.

### 🖥️ Multi-Protocol Support
* **RDP & XRDP**: Powered by `xfreerdp3` with support for dynamic resolution, multimon, audio redirection, clipboard sharing, gateway servers, shared folders, security protocol negotiation (NLA, TLS, RDP, ExtNLA), and certificate verification policies (TOFU, Strict Deny, Ignore).
* **VNC**: Seamless integration with `vncviewer` with automated credential passing and color level selection.
* **SPICE**: High-performance hypervisor connections via `remote-viewer`.
* **SSH**: Direct launch into your favorite desktop terminal emulator (`ptyxis`, `kgx`, `gnome-terminal`, `konsole`, `alacritty`, `foot`, `wezterm`, `xterm`) with custom private key identity paths.

### 🔒 Enterprise-Grade Security
* **RDP Stdin Credential Hardening**: Passwords are piped over standard input via `xfreerdp3 /from-stdin:force`, eliminating credential exposure in `/proc/<pid>/cmdline` and process monitoring utilities (`ps`, `top`).
* **Secret Service Keyring Integration**: Securely store connection passwords in your native system keyring (`gnome-keyring`, `kwallet`, `keepassxc`) via the Freedesktop Secret Service API (`oo7`).
* **Configuration Hardening**: Enforced `0700` Unix directory permissions on all user configuration paths.
* **Clean Process Group Signaling**: Process group signal termination (`-(pid as i32)`) ensures child shells, SSH sessions, and terminal wrappers exit cleanly without orphaned processes.

---

## ⌨️ Global Keyboard Shortcuts

VER features comprehensive keyboard accelerators adhering to GNOME HIG standards:

| Shortcut | Action |
| :--- | :--- |
| <kbd>Ctrl</kbd> + <kbd>K</kbd> | Open **Quick Connect** dialog |
| <kbd>Ctrl</kbd> + <kbd>N</kbd> | Create **New Connection** |
| <kbd>Ctrl</kbd> + <kbd>F</kbd> | Focus **Search Connections** bar |
| <kbd>Ctrl</kbd> + <kbd>I</kbd> | Open **Import Connections** dialog |
| <kbd>Ctrl</kbd> + <kbd>E</kbd> | Open **Export Connections** dialog |
| <kbd>Ctrl</kbd> + <kbd>D</kbd> | Open **Network Device Discovery** |
| <kbd>Ctrl</kbd> + <kbd>,</kbd> | Open **Preferences** window |
| <kbd>F5</kbd> / <kbd>Ctrl</kbd> + <kbd>R</kbd> | **Refresh Reachability** status for all hosts |
| <kbd>Return</kbd> | **Launch** selected connection |
| <kbd>Delete</kbd> | **Delete** selected connection |
| <kbd>Ctrl</kbd> + <kbd>?</kbd> / <kbd>F1</kbd> | Open **Keyboard Shortcuts** cheat sheet |
| <kbd>Ctrl</kbd> + <kbd>Q</kbd> | **Quit** application |

---

## 📦 Installation & Packaging

### Runtime Dependencies
Ensure the backend tools for your required protocols are installed:
* **RDP / XRDP**: `freerdp3` or `freerdp3-x11` (provides `xfreerdp3`)
* **VNC**: `tigervnc` (provides `vncviewer`)
* **SPICE**: `virt-viewer` (provides `remote-viewer`)
* **SSH**: Any desktop terminal emulator (`ptyxis`, `gnome-terminal`, `konsole`, `alacritty`, etc.)

### Build from Source

#### 1. Install Build Dependencies

**Debian / Ubuntu / Pop!_OS:**
```bash
sudo apt update
sudo apt install build-essential rustc cargo libgtk-4-dev libadwaita-1-dev
```

**Arch Linux / Manjaro / EndeavourOS:**
```bash
sudo pacman -S base-devel rust gtk4 libadwaita
```

**Fedora / RHEL:**
```bash
sudo dnf install @development-tools rust cargo gtk4-devel libadwaita-devel
```

#### 2. Compile and Run
```bash
# Run in development mode
cargo run

# Build optimized release binary
cargo build --release
```
The compiled binary will be available at `target/release/ver`.

### Distribution Packages

Helper scripts are available to generate native Linux distribution packages:
```bash
./build_deb.sh        # Generates Debian / Ubuntu .deb package
./build_pacman.sh     # Generates Arch Linux pacman .pkg.tar.zst package
./build_rpm.sh        # Generates RedHat / Fedora .rpm package
./build_appimage.sh   # Generates standalone x86_64 AppImage
```

---

## 🧪 Running Tests

Run the complete test suite across all unit, boundary, network, prober, and UI targets:
```bash
cargo test --all-targets
```

Verify linter compliance:
```bash
cargo clippy --all-targets --all-features -- -D warnings
```

---

## 📄 License & Changelog

* **License**: Distributed under the [GNU General Public License v3.0](LICENSE).
* **Changelog**: See [CHANGELOG.md](CHANGELOG.md) for detailed version history and release notes.
