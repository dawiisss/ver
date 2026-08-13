# Handoff Report: ConnectionEditor Widget Design (`src/ui/editor.rs`)

**Agent ID**: `explorer_m2_2`  
**Milestone**: M2 (GTK4 / Libadwaita Connection Manager UI)  
**Target File**: `src/ui/editor.rs`  
**Handoff Type**: Hard Handoff (Task Complete)  

---

## 1. Observation

1. **Original Python Editor UI (`AppDir/usr/share/ver/ui/editor.py`)**:
   - Lines 31-70: Built using `Adw.PreferencesGroup` ("General Settings") containing `Adw.EntryRow` for Name, Group, Host, Port, Username, and `Adw.PasswordEntryRow` for Password.
   - Lines 39-52: Protocol selection uses `Adw.ComboRow` with `Gtk.StringList.new(["rdp", "vnc", "ssh"])`.
   - Lines 73-81: `Adw.PreferencesGroup` ("Network & Hardware") containing `Adw.EntryRow` for MAC Address.
   - Lines 84-107: `Adw.PreferencesGroup` ("Advanced Protocol Settings") containing `Adw.SwitchRow` for RDP multimon, RDP fullscreen, RDP audio, VNC viewonly, VNC shared.
   - Lines 110-135: Button bar containing `Delete` (`destructive-action`), `Duplicate`, `Save` (`suggested-action`), `Wake`, and `Connect` (`pill`).
   - Lines 138-157: Protocol change handler toggles visibility of protocol-specific advanced settings and populates default ports (`3389` for RDP, `5900` for VNC, `22` for SSH).

2. **Serde Data Models (`src/models.rs`)**:
   - Lines 5-36: `Protocol` enum (`Rdp`, `Vnc`, `Ssh`), `default_port()`, `as_str()`.
   - Lines 44-74: `VncScaling` enum (`OriginalSize`, `FitToWindow`, `Stretch`).
   - Lines 77-103: `AdvancedSettings` struct (`rdp_multimon`, `rdp_fullscreen`, `rdp_audio`, `vnc_viewonly`, `vnc_shared`, `clipboard_sharing`, `color_depth`, `vnc_scaling`).
   - Lines 133-169: `Connection` struct (`id`, `name`, `protocol`, `host`, `port`, `username`, `mac_address`, `group`, `advanced_settings`).
   - Lines 225-236: `Connection::validate_mac(&self)` validates 12 hex digits.

3. **Storage & Keyring Infrastructure (`src/storage.rs`, `src/secrets.rs`)**:
   - `storage::save_connections(&[Connection])`: Writes to `~/.config/ver/connections.json` using 4-space indentation.
   - `secrets::get_password_sync(id)` / `secrets::set_password_sync(id, pass)` / `secrets::delete_password_sync(id)`: Handles Secret Service credentials via `oo7`.

4. **Existing Stub (`src/ui/editor.rs`)**:
   - Lines 1-38: Minimal non-GTK struct with basic field mutation methods (`update_name`, `update_host`, `update_port`, `update_password`).

---

## 2. Logic Chain

1. **Observation 1 & 2** show that the existing data models (`Connection`, `AdvancedSettings`, `Protocol`, `VncScaling`) perfectly align with the UI controls used in the Python app, but add structured support for `vnc_scaling` ("Original Size", "Fit to Window", "Stretch"), `clipboard_sharing`, and `color_depth`.
2. **Observation 1 & 3** demonstrate how passwords should be handled: passwords must NOT be stored in `Connection` JSON objects; instead, `bind_connection` retrieves passwords synchronously/async from `secrets::get_password_sync(id)`, and `handle_save` updates credentials via `secrets::set_password_sync(id, pass)` or `secrets::delete_password_sync(id)`.
3. **Observation 1 & 4** dictate that `src/ui/editor.rs` must be upgraded from a stub to a full GTK4 / Libadwaita composite widget using `adw::PreferencesPage`, `adw::PreferencesGroup`, `adw::EntryRow`, `adw::ComboRow`, `adw::SwitchRow`, `adw::PasswordEntryRow`, and `adw::ToastOverlay`.
4. **Validation Logic**: `get_connection_from_form` must validate non-empty `name` and `host`, numeric `port` range 1..65535, and valid `mac_address` via `validate_mac()`. Errors must be presented to the user cleanly via `adw::Toast`.

---

## 3. Caveats

1. **Asynchronous Keyring vs Synchronous UI**: `secrets.rs` provides `get_password_sync`, `set_password_sync`, and `delete_password_sync` which handle Tokio runtime dispatch internally. For UI thread responsiveness, using sync wrappers or `glib::MainContext::default().spawn_local` for async keyring calls works seamlessly.
2. **UI Subclassing vs Wrapper Struct**: The proposed architecture uses an encapsulated GTK widget wrapper struct (`ConnectionEditor`) holding `adw::ToastOverlay` and `gtk::Box`. This avoids complex GObject macro boilerplates while remaining 100% type-safe and idiomatic in Rust GTK4 applications.

---

## 4. Conclusion

The complete specification and technical blueprint for `src/ui/editor.rs` is fully designed and documented in `/home/dawiisss/Documents/antigravity/beautiful-goodall/.agents/explorer_m2_2/analysis.md`. `worker_m2` has clear step-by-step instructions to implement the `ConnectionEditor` widget.

---

## 5. Verification Method

To verify the implementation once `worker_m2` completes `src/ui/editor.rs`:

1. **Compilation Verification**:
   ```bash
   cargo check --lib
   ```
   Must compile cleanly without warnings or errors.

2. **Unit & UI Data Flow Tests**:
   ```bash
   cargo test --lib
   ```
   Must pass all tests including new unit tests for `ConnectionEditor` form validation, default port switching, and data extraction.

3. **Inspection Checklist**:
   - `bind_connection(Some(&conn))` correctly populates all `adw::EntryRow`, `adw::ComboRow`, and `adw::SwitchRow` fields.
   - `get_connection_from_form()` returns `Err` for empty name/host or invalid port/MAC.
   - Password fields seamlessly integrate with `secrets::get_password_sync` and `secrets::set_password_sync`.
   - Protocol switching dynamically hides/shows RDP vs VNC advanced preference groups.
