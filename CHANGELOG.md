# Changelog

All notable changes to **VER (Very Easy Remote Manager)** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [1.5.1] - 2026-10-13

### Security
- **SSH Target Flag Injection Protection**: Inserted `--` command delimiter before SSH remote host targets to prevent parameter injection attacks.
- **Pango Dynamic Markup Injection Protection**: Explicitly disabled markup rendering (`.use_markup(false)`) across dynamic labels and subtitles in Sidebar, Discovery, Import, and Export dialogs to prevent Pango markup injection from untrusted remote hostnames or service banners.
- **Strict MAC Address Validation & Normalization**: Enforced validation and normalized uppercase hex formatting for MAC addresses via `network::parse_mac_address` to prevent malformed or malicious broadcast payloads.
- **VNC Temporary Password Cleanup on Spawn Error**: Ensured temporary VNC password files (`/tmp/ver_vnc_*.pwd`) are synchronously removed if viewer process spawning fails.
- **Keyring Thread & Session Protection**: Cached Secret Service Keyring instances via `OnceCell<Option<Keyring>>` to eliminate redundant D-Bus handshakes, and guarded against duplicate concurrent session spawns for active connections.

### Fixed
- **System Tray Window Restoration**: Reused existing hidden application window on tray activation and desktop entry launch instead of instantiating redundant duplicate windows.
- **Keyboard Shortcut Focus Scoping**: Scoped `Delete` and `Return` key accelerators to the connection list sidebar so form entry fields (e.g. typing in editor, quick connect, or search) are not intercepted.
- **Network Discovery Rescan Race Condition**: Replaced global scanning state flags with per-scan cancellation tokens, enabling immediate rescan without deadlocks or missed sweep events.
- **Editor Wake-on-LAN Stale State**: Propagated live unsaved editor fields (MAC, host, port, name) to Wake-on-LAN actions rather than falling back to stale saved connection data.
- **Delete Confirmation Name Verification**: Ensured delete confirmation modal verifies against current editor entry name.
- **Sidebar Connection Row Live Update**: Added dynamic connection row updates (`update_connection_row`) on connection save to immediately refresh title, subtitle, icon, and reachability indicator without requiring a full sidebar rebuild.
- **XRDP Protocol Export**: Included `Protocol::Xrdp` in `.rdp` export format generation.
- **FreeRDP Graphics Flag Collision**: Eliminated conflicting `-gfx` / `/gfx` arguments when hardware acceleration is toggled.
- **VNC Color Level Preference**: Applied default VNC color level preferences to newly created VNC connections.

### Changed
- **IPv6 Host-Port Formatting**: Standardized bracketed IPv6 formatting (`[host]:port`) across RDP, FreeRDP, and VNC command launchers via `format_host_port`.
- **SSH Identity Path Expansion**: Expanded leading tildes (`~/`) in SSH identity file paths to absolute home paths before process execution.
- **Deterministic Test Harness**: Extracted `launch_vnc_with_binary` in `launcher.rs` to allow deterministic unit testing of process spawn failure and temporary file cleanup on systems with `vncviewer` installed on `PATH`.

## [1.5.0] - 2026-10-13

### Security
- **Credential Protection in FreeRDP**: Removed password injection into command-line arguments (`/p:<pass>`) in `build_rdp_args`, enforcing password transmission strictly via stdin pipe (`/from-stdin:force`) to prevent leakage in `/proc` and `ps` (CWE-214).
- **VNC Temporary Password Cleanup & Validation**: Added availability verification for the `vncpasswd` tool and improved temporary password file cleanup to monitor client lifecycle instead of relying on an arbitrary 5-second sleep (CWE-377 / CWE-379).
- **Safe Process Termination**: Guarded `libc::kill` process group signaling with PID sanity checks (`pid > 1`) and child liveness verification to prevent accidental self-termination or signaling recycled PIDs.

### Added
- **Deduplication in Network Discovery**: Discovered network devices (RDP, VNC, SSH) already present in the user's connection library (matching host, port, protocol, or loopback/localhost equivalents) are automatically filtered out from the Network Discovery dialog. Clicking "Add" immediately imports the connection and removes the row from the discovered section with dynamic summary reporting.

### Fixed
- **System Tray Startup Panic**: Gracefully handle missing StatusNotifierItem DBus service on minimal compositors and window managers without crashing on launch.
- **Keyring Threading & Panic Safety**: Fixed Tokio runtime panic when invoking `get_password_sync` from `CurrentThread` runtimes; moved Secret Service operations on connection save/delete/duplicate to asynchronous GLib tasks to prevent freezing the GTK UI.
- **Rust MSRV Compatibility**: Replaced `usize::is_multiple_of` with standard modulo arithmetic in RDP file decoder to restore full compatibility with Rust 1.75+.
- **Executable Detection**: Enhanced `find_binary_in_path` to verify Unix executable permissions (`0o111`) rather than only file existence.
- **Network Probing Latency**: Added fast-path IP address parsing in `probe_host_sync` to bypass blocking libc `getaddrinfo` lookups when probing numeric IP addresses.
- **SSH Config Importer Wildcard Filtering**: Filter out pattern match directives containing `*` or `?` from imported connection aliases.
- **Corrupt File Recovery Alerts**: Added UI toast notification on startup alerting the user if a corrupt connection file was detected and backed up, preventing silent data loss upon subsequent save.
- **Missing Window and System Tray Icons**: Embedded 64x64 ARGB32 tray icon pixmap directly in `VerTray::icon_pixmap` over DBus so the tray indicator renders on all StatusNotifierItem hosts without requiring external disk assets. Added automatic icon caching to `~/.local/share/icons/hicolor` and `~/.local/share/pixmaps` on startup, registered GTK icon theme search paths, and explicitly set `.icon_name("io.github.dawiisss.ver")` on `ApplicationWindow` to eliminate broken window icon placeholders in the top-left corner.

### Changed
- **Model Cleanliness**: Maintained legacy `color_depth: u8` in `AdvancedSettings` with serde defaults for complete backward configuration compatibility while standardizing internal usage on `rdp_color_depth: RdpColorDepth`.
- **Test Consolidation**: Consolidated scratch integration tests into unit test suites and removed redundant test files.
- **Repository Hygiene**: Purged untracked precompiled binary distribution archives from the workspace.

## [1.4.0] - 2026-08-18

### Added
- **Keyboard Accelerators & Shortcuts**:
  - Added <kbd>Delete</kbd> accelerator in main window to trigger connection deletion with confirmation dialog.
  - Added <kbd>F1</kbd> (Open Shortcuts Window) and <kbd>Ctrl+R</kbd> (Probe / Refresh) accelerator entries in the `GtkShortcutsWindow` cheat sheet.
- **In-Dialog Visual Validation & Banners**:
  - Added `adw::Banner` visual feedback in Quick Connect dialog for target format validation errors.
  - Added `adw::Banner` in Export dialog displaying format requirements (e.g., RDP single-connection export) and surfacing file write error messages.
- **Confirmation Prompts for Destructive Actions**:
  - Added modal `adw::MessageDialog` confirmation prompts when deleting connections via the Connection Editor delete button or <kbd>Delete</kbd> key accelerator.
- **Smart Port Preservation**:
  - Added protocol default tracking in Connection Editor to only auto-update port numbers if matching the previous protocol's default, preserving custom configured ports (e.g., `2222`, `3390`).
- **Thread Cancellation in Network Discovery**:
  - Added `Arc<AtomicBool>` cancellation tokens in local network discovery worker threads to immediately terminate background subnet sweeps upon dialog dismissal or re-scan.
- **Unit & Prober Test Coverage**:
  - Added unit test suite in `src/prober.rs` validating `HostStatus` display methods and TCP stream probing.

### Security
- **Terminal Emulator Argument Shell Escaping**:
  - Added safe POSIX argument escaping for single-command terminal launchers (e.g., `kgx` / GNOME Console via `-e`), preventing injection or argument splitting on paths with spaces or special characters.
- **VNC Temporary Password Cleanup Hardening**:
  - Enforced strict `0600` permissions on temporary VNC password files with asynchronous cleanup upon viewer process spawn.

### Changed
- **Prober Timeout Budgeting**:
  - Updated TCP prober to calculate and distribute remaining timeout (`per_addr_timeout = timeout - elapsed`) across multi-IP resolved hosts, preventing total probe time from exceeding requested timeouts.
- **Unified Host & Port Parsing**:
  - Consolidated duplicate host/port parsing logic across RDP and Remmina importers into a shared, robust `parse_host_port` function in `src/importers/mod.rs` supporting IPv4, bracketed IPv6, and hostnames.

### Fixed
- **Storage Error Propagation & Surfacing**:
  - Fixed `load_connections_from_path` and `load_config_from_path` to distinguish non-existent files (`ErrorKind::NotFound`) from permission or disk I/O errors (`EACCES`), propagating genuine errors instead of silently returning empty configurations.
  - Added `adw::Toast` notifications in main window to surface storage save failures and keyring deletion errors.
- **RDP File UTF-16 / BOM Decoding**:
  - Implemented multi-encoding decoding (`decode_rdp_bytes`) supporting UTF-16LE/BE with BOM, UTF-8 with BOM, and standard Windows `mstsc` UTF-16 exports without BOM.
- **OpenSSH Config Directive Precedence**:
  - Fixed directive parsing in `src/importers/ssh_config.rs` to enforce OpenSSH's standard "first-match-wins" precedence rule so specific host directives are not overridden by wildcard sections.
- **Test Assertion Coverage**:
  - Fixed `tests/serde_test.rs` to assert deserialized field values explicitly instead of printing to stdout.

## [1.3.1] - 2026-08-17

### Changed
- **Branding & Identity**: Updated project title to **VER - Very Easy Remote Manager** across the application, desktop metadata, and documentation.
- **Documentation Overhaul**: Added Table of Contents, moved Quick Install to top of README, added Runtime Dependencies guidance, and documented low memory footprint (~25 MB RAM).
- **Build Reproducibility**: Added `Cargo.lock` to version control for deterministic builds across all platforms and CI runners.

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
