# Project: beautiful-goodall (VER Rust Rewrite)

## Architecture
Rust-based GTK4/Libadwaita application using `gtk4-rs`, `libadwaita-rs`, `serde`, `vnc-rs`, `oo7` (keyring), and standard process execution.

Modules & Data Flow:
- `models`: `Connection` struct, `AdvancedSettings` struct, `AppConfig` struct, `Protocol` enum, `VncScaling` enum with Serde JSON annotations.
- `storage`: Load/save `~/.config/ver/connections.json` and `~/.config/ver/config.json` with 4-space indentation.
- `secrets`: Password storage/retrieval via Secret Service (`oo7` crate) using service `"ver_remote_connection_manager"`.
- `ui`: GTK4/Libadwaita application UI:
  - `window`: `MainWindow` with header bar, sidebar connection list (grouped), search/quick connect, editor pane, preferences modal.
  - `editor`: `ConnectionEditor` form (`AdwPreferencesGroup`, `AdwEntryRow`, `AdwComboRow`, `AdwSwitchRow`, Save/Connect/Delete buttons).
  - `vnc_widget`: Custom VNC client widget wrapping `gtk4::Picture` + `gdk::MemoryTexture` (B8G8R8X8 format), updating frames via async channels and forwarding key/pointer events.
- `vnc_client`: Async background VNC connection runner using `vnc` crate (v0.4.0), handling RFB handshake, password auth, framebuffer decoding, and event message queue.
- `launcher`: Protocol execution manager launching `xfreerdp3` (RDP) and external terminal emulators (SSH) via `std::process::Command`.
- `network`: Wake-on-LAN (WoL) UDP magic packet generator.

## Feature Inventory
Every feature from ORIGINAL_REQUEST and existing Python app assigned to a milestone:

| # | Feature | Description | Milestone | Source |
|---|---------|-------------|-----------|--------|
| 1 | Cargo Skeleton & Models | Cargo crate setup, dependencies, `Connection` & `AdvancedSettings` Serde data models, unit tests | M1 | ORIGINAL_REQUEST §R1 |
| 2 | JSON Storage Engine | Load/save `connections.json` and `config.json` with 4-space pretty printing & default fallbacks | M1 | ORIGINAL_REQUEST §R1 |
| 3 | Keyring Integration | Save/retrieve passwords in system keyring under service `ver_remote_connection_manager` via `oo7` | M1 | Survey 1/2 |
| 4 | GTK4/Libadwaita Window Layout | Main window with HeaderBar, Sidebar, Grouped ListBox, SearchEntry, Editor Bin | M2 | ORIGINAL_REQUEST §R2 |
| 5 | Connection Editor UI Form | Full form for viewing, editing, creating, duplicating, saving, and deleting connections | M2 | ORIGINAL_REQUEST §R2 |
| 6 | Real-time Search & Grouping | Group connections by group string, filter connections via Quick Connect search bar | M2 | Survey 1 |
| 7 | Preferences & Theme Toggle | Theme selection (System, Light, Dark) saving to `config.json` | M2 | Survey 1 |
| 8 | Embedded VNC Framebuffer Rendering | Render `vnc-rs` RFB framebuffers to `gtk4::Picture` using `gdk::MemoryTexture` (B8G8R8X8) | M3 | ORIGINAL_REQUEST §R3 |
| 9 | VNC Mouse & Keyboard Event Mapping | Forward GDK key events and pointer motion/click events to remote VNC session | M3 | ORIGINAL_REQUEST §R3 |
| 10 | RDP Session Launcher | Spawn `xfreerdp3` process detached with appropriate CLI flags (/v, /u, /p, /dynamic-resolution, +clipboard) | M4 | ORIGINAL_REQUEST §R4 |
| 11 | SSH Session Launcher | Launch external terminal emulator (`ptyxis`, `kgx`, `gnome-terminal`, etc.) with `ssh` command | M4 | ORIGINAL_REQUEST §R4 |
| 12 | Wake-on-LAN Generator | Send UDP WoL magic packet (`FF*6 + MAC*16`) on port 9 | M4 | Survey 1 |
| 13 | E2E Requirement Test Suite | Comprehensive opaque-box test suite (Tiers 1-4) validating data, UI, VNC, RDP, SSH | E2E Track | Dual Track |

## Milestones

| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| M1 | R1: Rust Skeleton & Serde Data Models | Cargo setup, dependencies, Serde models, JSON storage engine, keyring integration | none | DONE |
| M2 | R2: GTK4 / Libadwaita Connection UI | Main window, sidebar connection list, quick search, connection editor form, preferences modal | M1 | DONE |
| M3 | R3: Native VNC Client & Rendering Widget | Embedded `vnc-rs` async client, GTK4 Picture rendering, keyboard & mouse input propagation | M1, M2 | DONE |
| M4 | R4: RDP, SSH & WoL Integration | `xfreerdp3` launching, SSH terminal spawning, WoL UDP magic packets | M1, M2 | DONE |
| M5 | Final Milestone: E2E Test Suite & Hardening | Pass 100% E2E test suite + Tier 5 Adversarial Coverage Hardening | M1-M4, TEST_READY | DONE |

## Interface Contracts
### `models` ↔ `storage`
- `Connection::from_json(s: &str) -> Result<Vec<Connection>>`
- `Connection::to_json(conns: &[Connection]) -> Result<String>` (4-space indent)
- `AppConfig::load() / save()`

### `models` ↔ `secrets`
- `secrets::get_password(id: &str) -> Result<Option<String>>`
- `secrets::set_password(id: &str, pass: &str) -> Result<()>`
- `secrets::delete_password(id: &str, pass: &str) -> Result<()>`

### `vnc_client` ↔ `vnc_widget`
- Channel message: `VncFrameUpdate { width: u32, height: u32, stride: usize, pixels: Vec<u8> }`
- Channel message: `VncEvent { Key { keysym: u32, down: bool }, Pointer { x: u16, y: u16, mask: u8 } }`

### `launcher` ↔ `models`
- `launcher::launch_rdp(conn: &Connection, password: Option<&str>) -> Result<Child>`
- `launcher::launch_ssh(conn: &Connection) -> Result<Child>`
- `network::send_wol(mac: &str) -> Result<()>`

## Code Layout
```
src/
├── main.rs               # Application entrypoint (libadwaita::Application)
├── models.rs             # Connection, AdvancedSettings, AppConfig, Protocol models
├── storage.rs            # JSON storage load/save routines
├── secrets.rs            # Keyring password management (oo7)
├── network.rs            # Wake-on-LAN UDP magic packet logic
├── launcher.rs           # Process spawning for xfreerdp3 and SSH terminals
├── vnc/
│   ├── mod.rs            # VNC module export
│   ├── client.rs         # Async vnc-rs connection & RFB thread handler
│   └── widget.rs         # gtk4::Picture widget & event controllers
└── ui/
    ├── mod.rs            # UI module export
    ├── window.rs         # AdwApplicationWindow, layout, headerbar, sidebar
    ├── editor.rs         # ConnectionEditor form widget
    ├── preferences.rs    # PreferencesWindow modal
    └── discovery.rs     # Network discovery dialog
tests/
├── e2e_data_tests.rs
├── e2e_ui_tests.rs
└── e2e_vnc_tests.rs
```
