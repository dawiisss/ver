## 2026-08-12T12:58:12Z

Task Objective:
Investigate and design `src/ui/editor.rs` (ConnectionEditor widget) for viewing, editing, creating, duplicating, saving, and deleting remote connections.

Scope of Investigation:
1. Examine existing models (`models::Connection`, `models::AdvancedSettings`, `models::Protocol`, `models::VncScaling`) and storage/keyring API (`storage.rs`, `secrets.rs`).
2. Design `src/ui/editor.rs` using `libadwaita` and `gtk4` Rust bindings:
   - Form layout using `adw::PreferencesPage` / `adw::PreferencesGroup`.
   - General rows: `AdwEntryRow` for Name, Host, Port (numeric entry), Username, Group; `AdwEntryRow` / `AdwPasswordEntryRow` for Password.
   - Protocol selection: `AdwComboRow` or `gtk::DropDown` for Protocol (`VNC`, `RDP`, `SSH`).
   - Advanced settings section: `AdwExpanderRow` or `AdwPreferencesGroup` for `vnc_scaling` (`FitWindow`, `OriginalSize`, `Custom`), `fullscreen` switch, `clipboard_sync` switch, `dynamic_resolution` switch, `view_only` switch, `mac_address` entry, `domain` entry.
   - Action buttons: "Connect" (primary blue button), "Save" (suggested button), "Duplicate", "Delete" (destructive red button).
3. Detail how data flows between `ConnectionEditor` UI widgets and `Connection` struct:
   - `bind_connection(&self, conn: Option<&Connection>)` to populate form fields.
   - `get_connection_from_form(&self) -> Result<Connection, String>` to extract current form values.
   - Integration with `secrets::get_password` and `secrets::set_password` / `secrets::delete_password`.
   - Handling new connection creation vs editing existing connection vs deletion callbacks.
4. Document exact struct definitions, methods, signal handlers, error validation (e.g. empty name/host validation), and step-by-step implementation plan for `worker_m2`.
