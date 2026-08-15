# Changelog

All notable changes to **VER (Very Easy Remote)** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

## [1.3.0] - 2026-08-15

### Added
- **Quick Connect Bar (<kbd>Ctrl+K</kbd>)**:
  - Global Quick Connect dialog accessible from the header bar or keyboard shortcut.
  - Multi-protocol URI and shorthand syntax parser supporting `ssh://`, `rdp://`, `vnc://`, `spice://`, `xrdp://`, `user@host:port`, `user@host`, plain IPs/hostnames, and IPv6 brackets (`[fe80::1]:22`).
  - Dynamic live field auto-population supporting instant ad-hoc connection ("Connect") and persistent saving ("Save & Connect").
- **Global Keyboard Accelerators & Shortcuts Overlay (<kbd>Ctrl+?</kbd> / <kbd>F1</kbd>)**:
  - Registered GNOME accelerators: <kbd>Ctrl+K</kbd> (Quick Connect), <kbd>Ctrl+N</kbd> (New Connection), <kbd>Ctrl+F</kbd> (Search), <kbd>Ctrl+I</kbd> (Import), <kbd>Ctrl+E</kbd> (Export), <kbd>Ctrl+D</kbd> (Discovery), <kbd>Ctrl+,</kbd> (Preferences), <kbd>F5</kbd> / <kbd>Ctrl+R</kbd> (Refresh Reachability), <kbd>Enter</kbd> (Launch), <kbd>Delete</kbd> (Remove), and <kbd>Ctrl+Q</kbd> (Quit).
  - Built a native `GtkShortcutsWindow` cheat sheet with built-in search and clean <kbd>Esc</kbd> dismissal.
- **Import & Export Ecosystem**:
  - **Remmina Profile Importer**: Native INI parser with auto-discovery for `~/.local/share/remmina/*.remmina` profiles.
  - **OpenSSH Config Importer**: Auto-detects and extracts host aliases, ports, users, and expanded identity files from `~/.ssh/config`.
  - **Microsoft `.rdp` Support**: Import and export standard `.rdp` configuration files.
  - **JSON Backup / Restore**: Export and import full profile libraries with timestamp and version metadata.
  - **Conflict Resolution Engine**: Three strategies when importing matching profiles: *Skip Duplicates*, *Overwrite Existing*, or *Keep Both (Rename with Suffix)*.
  - **Interactive Import Dialog (<kbd>Ctrl+I</kbd>)**: Preview list with multi-selection checkboxes, auto-scan buttons, and Select/Deselect All controls.
  - **Interactive Export Dialog (<kbd>Ctrl+E</kbd>)**: Selective multi-connection checklist with live item counter, Select/Deselect All, format toggle, and smart focus pre-selection.
- **Live Host Reachability Prober & Integrated Wake-on-LAN**:
  - **Asynchronous TCP Prober**: Runtime-agnostic TCP reachability prober with roundtrip latency tracking and worker pool batching.
  - **Sidebar Status Indicators**: Real-time status dots next to connections in the sidebar (🟢 **Online**, 🔴 **Offline**, 🟡 **Probing**, ⚪ **Unknown**).
  - **Context-Aware Wake-on-LAN Workflow**: Sends magic packet and begins an automated 30-second polling loop, updating the status dot to 🟢 Online and displaying a desktop toast alert as soon as the target host responds.
- **SSH Private Key Identity Support**: Added `ssh_identity_file` to `AdvancedSettings` with an "Advanced SSH Settings" entry in the Connection Editor, enabling custom SSH private key paths (`-i <key>`).

### Security
- **RDP Stdin Credential Hardening**: Hardened `xfreerdp3` launching to pass passwords over standard input via `/from-stdin:force`, preventing cleartext credential exposure in `/proc/<pid>/cmdline` and process inspection tools (`ps`, `top`).
- **Configuration Directory Permissions**: Enforced strict Unix permissions (`0700`) on `~/.config/ver` parent directories to protect connection metadata and server lists.

### Changed
- **Process Group Termination**: Updated session disconnection and termination handlers to signal the entire process group (`-(pid as i32)`) with `SIGTERM` and fallback `SIGKILL`, ensuring terminal emulator wrappers and child SSH processes are terminated cleanly without orphaning.
- **Runtime-Agnostic Background Worker Pool**: Decoupled prober and keyring tasks from ambient Tokio runtime requirements to ensure full stability and crash prevention within the GLib/GTK event loop.
- **Modernized Discovery Channel**: Replaced deprecated `glib::MainContext::channel` with `async_channel` dispatched directly onto the GLib main context.
- **Collision-Resistant Corrupt File Backups**: Upgraded corrupt storage backups to millisecond timestamp precision (`.corrupt.<timestamp_millis>`) to prevent file collisions during rapid recovery.

---

## [1.1.0]

### Added
- **Active Session Tracking**: Live session management tracking active connection states with real-time "Active" sidebar badges and interactive session log streaming.
- **RDP Security Protocol Negotiation**: Added support for choosing RDP security negotiation protocols (Negotiate / Auto, NLA, TLS, RDP, and Extended NLA) with automatic disabling of unconfigured Kerberos fallback.
- **RDP Certificate Validation Policies**: Dedicated RDP Security & Certificates settings section supporting Ignore, Trust on First Use (TOFU), Strict Deny, and Prompt/Ask verification policies.
- **Multi-Format Packaging & CI Automation**: Added native package building scripts for Debian (`.deb`), RedHat (`.rpm`), Arch Linux (`.pkg.tar.zst` via pacman), and AppImage (`.AppImage`), with tag-triggered GitHub Actions release automation.

### Fixed
- Fixed packaging scripts to resolve dynamic versions directly from `Cargo.toml` and compile release binaries prior to packaging.
- Resolved all compiler and Clippy linter warnings across library, binary, and test targets.

---

## [1.0.0]

### Added
- **Native Rust GTK4/Libadwaita Architecture**: Full native reimplementation with modern GNOME Human Interface Guidelines (HIG) compliance, dark/light adaptive theming, and responsive layouts.
- **Multi-Protocol Support**: First-class support for RDP/XRDP (`xfreerdp3`), VNC (`vncviewer`), SPICE (`remote-viewer`), and SSH across popular Linux terminal emulators.
- **Secret Service Keyring Storage**: Passwords stored securely in the native system keyring via Freedesktop Secret Service API (`oo7`) with legacy credential migration support.
- **Wake-on-LAN (WoL)**: Magic packet generator and UDP broadcast dispatcher supporting colon, hyphen, Cisco dot, byte dot, and unseparated MAC address formats.
- **Network Service Discovery**: Multi-threaded scanner probing local interfaces and subnet targets for open RDP, VNC, and SSH ports with one-click connection import.
- **Atomic Persistence**: Indented JSON configuration storage with atomic file replacement (`NamedTempFile`) and automatic self-healing for corrupt files.
- **System Tray Integration**: Background StatusNotifierItem system tray support with quick show/hide and graceful shutdown actions.
