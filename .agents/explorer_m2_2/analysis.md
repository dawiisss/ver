# Analysis Report: ConnectionEditor Widget Design (`src/ui/editor.rs`)

**Author**: Explorer 2 (Milestone 2)  
**Target Path**: `src/ui/editor.rs`  
**Date**: 2026-08-12  

---

## 1. Problem Boundary & Objectives

The goal of Milestone 2 (M2) is to implement the GTK4 / Libadwaita Connection Manager UI for `beautiful-goodall` (VER - Very Easy Remote). As part of M2, Explorer 2's objective is to investigate and design `src/ui/editor.rs` (`ConnectionEditor` widget).

The `ConnectionEditor` widget must support:
1. **Viewing & Editing**: Displaying details of an existing connection and updating fields in real time or upon explicit save.
2. **Creating & Duplicating**: Setting up fresh connection instances with protocol defaults or cloning existing connections with a new UUID and name suffix.
3. **Saving & Deleting**: Persisting connection data to storage (`connections.json`) and passwords to system keyring (`oo7`), or purging connection records and credentials upon deletion.
4. **Action Handlers**: Initiating remote connections (embedded VNC, RDP, SSH) and Wake-on-LAN (WoL) magic packets.

---

## 2. Evidence Chain & Existing Infrastructure

### 2.1 Existing Models (`src/models.rs`)
- `Protocol`: Enum (`Rdp`, `Vnc`, `Ssh`).
  - Default ports: `Rdp` => 3389, `Vnc` => 5900, `Ssh` => 22.
  - Serde rename: `"rdp"`, `"vnc"`, `"ssh"`.
- `VncScaling`: Enum (`OriginalSize`, `FitToWindow`, `Stretch`).
  - Serde rename: `"Original Size"`, `"Fit to Window"`, `"Stretch"`.
- `AdvancedSettings`: Struct holding:
  - `rdp_multimon: bool`
  - `rdp_fullscreen: bool`
  - `rdp_audio: bool`
  - `vnc_viewonly: bool`
  - `vnc_shared: bool`
  - `clipboard_sharing: bool`
  - `color_depth: u8` (0, 8, 16, 24, 32)
  - `vnc_scaling: VncScaling`
- `Connection`: Primary model containing:
  - `id: String` (UUID v4)
  - `name: String` (Default: "New Connection")
  - `protocol: Protocol`
  - `host: String`
  - `port: u16` (Default: 3389 / protocol default)
  - `username: String`
  - `mac_address: String`
  - `group: String` (Default: "Default")
  - `advanced_settings: AdvancedSettings`

### 2.2 Storage & Keyring API (`src/storage.rs`, `src/secrets.rs`)
- `storage::save_connections(&[Connection])`: Atomically writes connection vector to `~/.config/ver/connections.json` with 4-space indentation.
- `secrets::get_password_sync(id)` / `secrets::get_password(id)`: Async/sync password retrieval from Secret Service (`oo7`).
- `secrets::set_password_sync(id, password)` / `secrets::set_password(id, password)`: Stores credential in Secret Service.
- `secrets::delete_password_sync(id)` / `secrets::delete_password(id)`: Purges stored credential from Secret Service.

---

## 3. UI Component Architecture (`libadwaita` & `gtk4`)

### 3.1 Container Layout Hierarchy

```
adw::ToastOverlay
└── gtk::ScrolledWindow (policy: Never, Automatic)
    └── adw::PreferencesPage
        ├── adw::PreferencesGroup ("General Settings")
        │   ├── adw::EntryRow ("Name")
        │   ├── adw::EntryRow ("Group")
        │   ├── adw::ComboRow ("Protocol": RDP, VNC, SSH)
        │   ├── adw::EntryRow ("Host (IP or Domain)")
        │   ├── adw::EntryRow ("Port")
        │   ├── adw::EntryRow ("Username")
        │   └── adw::PasswordEntryRow ("Password")
        ├── adw::PreferencesGroup ("Network & Hardware")
        │   └── adw::EntryRow ("MAC Address (for WoL)")
        ├── adw::PreferencesGroup ("Advanced RDP Settings") [Visible when Protocol == RDP]
        │   ├── adw::SwitchRow ("Fullscreen Mode")
        │   ├── adw::SwitchRow ("Multi-Monitor Support")
        │   └── adw::SwitchRow ("Audio Redirection")
        ├── adw::PreferencesGroup ("Advanced VNC Settings") [Visible when Protocol == VNC]
        │   ├── adw::ComboRow ("VNC Scaling": Original Size, Fit to Window, Stretch)
        │   ├── adw::SwitchRow ("View-Only Mode")
        │   └── adw::SwitchRow ("Shared Session")
        ├── adw::PreferencesGroup ("Common Advanced Settings")
        │   ├── adw::SwitchRow ("Clipboard Sharing")
        │   └── adw::ComboRow ("Color Depth": Auto, 8-bit, 16-bit, 24-bit, 32-bit)
        └── gtk::Box (Horizontal, Halignment: Center, Spacing: 12, Margin-Top: 24)
            ├── gtk::Button ("Delete") [.destructive-action]
            ├── gtk::Button ("Duplicate")
            ├── gtk::Button ("Wake")
            ├── gtk::Button ("Save") [.suggested-action]
            └── gtk::Button ("Connect") [.accent, .pill]
```

---

## 4. Complete Data Flow Specification

### 4.1 Binding Connection Data (`bind_connection`)

```
[Selected Connection / None]
          │
          ▼
   bind_connection(Option<&Connection>)
          │
          ├── If Some(conn):
          │     1. Store conn.id in current_connection_id (Rc<RefCell<Option<String>>>)
          │     2. Set entry_name text = conn.name
          │     3. Set entry_group text = conn.group
          │     4. Set combo_proto selected index = (Rdp: 0, Vnc: 1, Ssh: 2)
          │     5. Set entry_host text = conn.host
          │     6. Set entry_port text = conn.port.to_string()
          │     7. Set entry_user text = conn.username
          │     8. Fetch password: secrets::get_password_sync(&conn.id)
          │        └── Set entry_pass text = password.unwrap_or_default()
          │     9. Set entry_mac text = conn.mac_address
          │    10. Set Advanced RDP switches (rdp_fullscreen, rdp_multimon, rdp_audio)
          │    11. Set Advanced VNC combo & switches (vnc_scaling, vnc_viewonly, vnc_shared)
          │    12. Set Common switches (clipboard_sharing, color_depth)
          │    13. Set action buttons sensitive = true
          │    14. Reset is_dirty = false
          │
          └── If None:
                1. Clear all entry rows
                2. Set default protocol (RDP / index 0)
                3. Set action buttons sensitive = false
```

### 4.2 Extracting & Validating Connection Data (`get_connection_from_form`)

```
get_connection_from_form(&self) -> Result<Connection, String>
          │
          ├── 1. Read & Trim Fields:
          │     - name = entry_name.text().trim()
          │     - host = entry_host.text().trim()
          │     - group = entry_group.text().trim() (fallback: "Default" if empty)
          │     - username = entry_user.text().trim()
          │     - mac_address = entry_mac.text().trim()
          │
          ├── 2. Validation Checks:
          │     - If name.is_empty() ──► Return Err("Connection name cannot be empty")
          │     - If host.is_empty() ──► Return Err("Host address cannot be empty")
          │     - Parse port: entry_port.text().parse::<u16>()
          │       If Err or port == 0 ──► Return Err("Port must be a valid number between 1 and 65535")
          │
          ├── 3. Protocol Mapping:
          │     - Selected index 0 ──► Protocol::Rdp
          │     - Selected index 1 ──► Protocol::Vnc
          │     - Selected index 2 ──► Protocol::Ssh
          │
          ├── 4. Advanced Settings Extraction:
          │     - rdp_fullscreen = switch_rdp_fullscreen.is_active()
          │     - rdp_multimon = switch_rdp_multimon.is_active()
          │     - rdp_audio = switch_rdp_audio.is_active()
          │     - vnc_scaling = match combo_vnc_scaling.selected() (0 => OriginalSize, 1 => FitToWindow, 2 => Stretch)
          │     - vnc_viewonly = switch_vnc_viewonly.is_active()
          │     - vnc_shared = switch_vnc_shared.is_active()
          │     - clipboard_sharing = switch_clipboard.is_active()
          │     - color_depth = match combo_color_depth.selected() (0 => 0, 1 => 8, 2 => 16, 3 => 24, 4 => 32)
          │
          ├── 5. Construct Connection Struct:
          │     - id = current_connection_id or Uuid::new_v4().to_string()
          │     - Call conn.validate_mac() ──► Return Err if MAC invalid
          │     - Call conn.sanitize()
          │
          └── Return Ok(conn)
```

---

## 5. Struct Definition & Method Signatures

```rust
use gtk::prelude::*;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::models::{AdvancedSettings, Connection, Protocol, VncScaling};
use crate::network;
use crate::secrets;

#[derive(Clone)]
pub struct ConnectionEditor {
    pub toast_overlay: adw::ToastOverlay,
    pub container: gtk::Box,

    // Form Entry Rows
    entry_name: adw::EntryRow,
    entry_group: adw::EntryRow,
    combo_proto: adw::ComboRow,
    entry_host: adw::EntryRow,
    entry_port: adw::EntryRow,
    entry_user: adw::EntryRow,
    entry_pass: adw::PasswordEntryRow,
    entry_mac: adw::EntryRow,

    // Preference Groups
    group_rdp: adw::PreferencesGroup,
    group_vnc: adw::PreferencesGroup,

    // Advanced Switches & Combos
    switch_rdp_fullscreen: adw::SwitchRow,
    switch_rdp_multimon: adw::SwitchRow,
    switch_rdp_audio: adw::SwitchRow,

    combo_vnc_scaling: adw::ComboRow,
    switch_vnc_viewonly: adw::SwitchRow,
    switch_vnc_shared: adw::SwitchRow,

    switch_clipboard: adw::SwitchRow,
    combo_color_depth: adw::ComboRow,

    // Action Buttons
    btn_connect: gtk::Button,
    btn_save: gtk::Button,
    btn_duplicate: gtk::Button,
    btn_wake: gtk::Button,
    btn_delete: gtk::Button,

    // Internal State
    current_connection_id: Rc<RefCell<Option<String>>>,
    is_dirty: Rc<RefCell<bool>>,

    // Action Callbacks
    on_save: Rc<RefCell<Option<Box<dyn Fn(Connection) + 'static>>>>,
    on_connect: Rc<RefCell<Option<Box<dyn Fn(Connection) + 'static>>>>,
    on_duplicate: Rc<RefCell<Option<Box<dyn Fn(Connection) + 'static>>>>,
    on_delete: Rc<RefCell<Option<Box<dyn Fn(String) + 'static>>>>,
}

impl ConnectionEditor {
    pub fn new() -> Self;
    pub fn widget(&self) -> &adw::ToastOverlay;

    pub fn bind_connection(&self, conn: Option<&Connection>);
    pub fn get_connection_from_form(&self) -> Result<Connection, String>;
    pub fn get_password_from_form(&self) -> String;
    pub fn is_dirty(&self) -> bool;

    // Callback Registration
    pub fn connect_save<F: Fn(Connection) + 'static>(&self, f: F);
    pub fn connect_connect<F: Fn(Connection) + 'static>(&self, f: F);
    pub fn connect_duplicate<F: Fn(Connection) + 'static>(&self, f: F);
    pub fn connect_delete<F: Fn(String) + 'static>(&self, f: F);

    // Private Signal Handlers
    fn setup_signals(&self);
    fn update_protocol_visibility(&self, selected_idx: u32, is_user_change: bool);
    fn handle_save(&self);
    fn handle_connect(&self);
    fn handle_duplicate(&self);
    fn handle_delete(&self);
    fn handle_wake(&self);
    fn show_toast(&self, message: &str);
}
```

---

## 6. Action Signal Handling & Keyring Integration

1. **Save Action (`handle_save`)**:
   - Call `get_connection_from_form()`.
   - If `Err(msg)`: Display toast notification (`self.show_toast(&msg)`).
   - If `Ok(conn)`:
     - Read password via `self.get_password_from_form()`.
     - If password is non-empty: `secrets::set_password_sync(&conn.id, &password)`.
     - If password is empty: `secrets::delete_password_sync(&conn.id)`.
     - Invoke `on_save` callback if registered.
     - Reset `is_dirty` to `false`.
     - Display toast notification ("Connection saved successfully").

2. **Connect Action (`handle_connect`)**:
   - First invoke `self.handle_save()`.
   - Call `get_connection_from_form()`.
   - If `Ok(conn)`: Invoke `on_connect` callback if registered.

3. **Duplicate Action (`handle_duplicate`)**:
   - Call `get_connection_from_form()`.
   - If `Ok(mut conn)`:
     - Set `conn.id = uuid::Uuid::new_v4().to_string()`.
     - Set `conn.name = format!("{} (Copy)", conn.name)`.
     - Save password for new ID if password present.
     - Invoke `on_duplicate` callback if registered.
     - Display toast notification ("Connection duplicated").

4. **Delete Action (`handle_delete`)**:
   - If `current_connection_id` contains `Some(id)`:
     - Delete password: `secrets::delete_password_sync(&id)`.
     - Invoke `on_delete` callback with `id`.
     - Call `self.bind_connection(None)`.

5. **Wake Action (`handle_wake`)**:
   - Read MAC string from `entry_mac`.
   - Validate MAC format via `Connection { mac_address: mac, ..Default::default() }.validate_mac()`.
   - If `Ok(Some(clean_mac))`: Call `network::send_wol(&clean_mac)`. Display toast notification ("Wake-on-LAN magic packet sent").
   - If `Err(e)`: Display toast notification (&e).

---

## 7. Step-by-Step Implementation Blueprint for `worker_m2`

1. **Step 1: Replace `src/ui/editor.rs` stub**
   - Import required GTK4/Libadwaita traits (`gtk::prelude::*`, `libadwaita::prelude::*`).
   - Define `ConnectionEditor` struct as specified above.

2. **Step 2: Implement `ConnectionEditor::new()` UI construction**
   - Instantiate `adw::ToastOverlay`.
   - Instantiate `gtk::ScrolledWindow` wrapping `adw::PreferencesPage`.
   - Build `PreferencesGroup` instances and add `EntryRow`, `ComboRow`, `SwitchRow`, `PasswordEntryRow`.
   - Build action button bar with CSS classes (`destructive-action`, `suggested-action`, `pill`, `accent`).
   - Pack into container and setup signal connections.

3. **Step 3: Implement Data Binding & Form Extraction**
   - Implement `bind_connection(&self, conn: Option<&Connection>)`.
   - Implement `get_connection_from_form(&self) -> Result<Connection, String>`.
   - Implement `get_password_from_form(&self) -> String`.

4. **Step 4: Wire Signals & Actions**
   - Connect `notify::selected` on `combo_proto` to toggle `group_rdp` vs `group_vnc` and update default ports (`3389`, `5900`, `22`).
   - Wire `Save`, `Connect`, `Duplicate`, `Delete`, and `Wake` buttons to their respective `handle_*` handlers.
   - Wire `changed`, `notify::text`, `notify::active` signals on form fields to set `is_dirty = true`.

5. **Step 5: Add Unit & UI Integration Tests**
   - Add unit tests in `src/ui/editor.rs` verifying:
     - `get_connection_from_form` validation (empty name, empty host, invalid port, invalid MAC).
     - `bind_connection` population of default and customized connection fields.
     - `VncScaling` and `Protocol` combo selection mapping.

---
