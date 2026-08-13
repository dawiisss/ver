# Handoff Report: GTK4 / Libadwaita Preferences Window, Discovery Dialog, & Entrypoint Integration

## 1. Observation

Direct observations from codebase inspection:
- **`Cargo.toml` (Lines 15–24)**: Dependencies include `gtk` (`gtk4` 0.7), `libadwaita` (0.5), `serde` (1.0), `serde_json` (1.0), `vnc` (0.4.0), `anyhow` (1.0), `oo7` (0.3), `tokio` (1.34), `dirs` (5.0).
- **`src/models.rs` (Lines 239–256)**: `AppConfig` struct is currently defined with only `pub theme: String`. Missing fields for `default_protocol`, `auto_connect_last`, `default_vnc_scaling`, and `last_connected_id`.
- **`src/storage.rs` (Lines 130–189)**: Provides `load_config() -> Result<AppConfig>` and `save_config(config: &AppConfig) -> Result<()>`, targeting `~/.config/ver/config.json` formatted with 4-space JSON indentation.
- **`src/ui/preferences.py` (Lines 9–70)**: Original Python implementation used `Adw.PreferencesWindow` with an Appearance group (`Adw.ComboRow` for System Default, Dark Mode, Light Mode) and Data Management group (`Adw.ActionRow` for Export).
- **`src/ui/discovery.py` (Lines 9–104)**: Original Python implementation used `Adw.Window` modal dialog with `Gtk.Spinner`, `Gtk.ListBox`, zeroconf service browser for `_ssh._tcp` and `_rfb._tcp`, and `on_add_callback` connection creation.
- **`src/main.rs` (Lines 1–21)**: Current Rust main is a stub returning a bare `libadwaita::ApplicationWindow`.
- **`src/lib.rs` (Lines 1–19)** & **`src/ui/mod.rs` (Lines 1–10)**: Export structure exists but needs re-exports for `apply_theme`, `PreferencesWindow`, `DiscoveryDialog`, and `DiscoveredService`.

---

## 2. Logic Chain

1. **AppConfig Backwards Compatibility & Serialization**:
   - Expanding `AppConfig` with `#[serde(default)]` ensures Serde seamlessly deserializes legacy or partial `config.json` files while populating missing preference fields (`default_protocol`, `auto_connect_last`, `default_vnc_scaling`).
   - Standardizing `storage::save_config` invocations ensures configuration modifications are immediately auto-persisted.

2. **PreferencesWindow Architecture**:
   - `PreferencesWindow` must wrap/build `adw::PreferencesWindow` modal dialog transient to `MainWindow`.
   - Modifying settings (theme, default protocol, auto-connect toggle, VNC scaling) updates the `Rc<RefCell<AppConfig>>` model, applies theme changes immediately via `adw::StyleManager::default().set_color_scheme()`, and persists config to disk.

3. **DiscoveryDialog Architecture**:
   - `DiscoveryDialog` builds an `adw::Window` modal dialog with a header bar, refresh action, loading spinner, and list box of discovered network hosts.
   - Asynchronous subnet scanning (probing ports 5900, 3389, 22) runs in background Tokio tasks / threads and uses `glib::MainContext::channel` to safely dispatch `DiscoveredService` records to the GTK main thread.
   - Discovered item `adw::ActionRow`s feature an "Add" button that constructs a `Connection` struct pre-populated with host, port, protocol, name and triggers `on_add_callback(Connection)`.

4. **Entrypoint Lifecycle**:
   - `src/main.rs` calls `libadwaita::init()`, loads `AppConfig` and `Vec<Connection>` from disk via `storage`, applies the saved theme scheme, builds `MainWindow`, and starts the `libadwaita::Application` main loop.

---

## 3. Caveats

1. **Read-Only Scope**: In accordance with the Explorer archetype, no source files under `src/` or `tests/` were modified by this agent. All designs are presented for implementation by `worker_m2`.
2. **Network Subnet Scanning**: Socket probing on local subnets depends on network interface configuration and short connection timeouts (200–300ms) to ensure non-blocking UI responsiveness.
3. **GTK Thread Safety**: All UI widget updates must occur on the GTK main thread using `glib::MainContext::channel` or `glib::idle_add_local`.

---

## 4. Conclusion & Implementation Blueprint

### 4.1 Struct Definitions & Interfaces

#### Extended `AppConfig` (`src/models.rs`)
```rust
fn default_theme() -> String {
    "system".to_string()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    #[serde(default = "default_theme")]
    pub theme: String,
    pub default_protocol: Protocol,
    pub auto_connect_last: bool,
    pub default_vnc_scaling: VncScaling,
    pub last_connected_id: Option<String>,
}
```

#### `PreferencesWindow` (`src/ui/preferences.rs`)
```rust
pub struct PreferencesWindow;

impl PreferencesWindow {
    pub fn build_window(
        parent: &impl IsA<gtk::Window>,
        config: Rc<RefCell<AppConfig>>,
    ) -> adw::PreferencesWindow;
}

pub fn apply_theme(theme_str: &str);
```

#### `DiscoveryDialog` (`src/ui/discovery.rs`)
```rust
#[derive(Debug, Clone)]
pub struct DiscoveredService {
    pub name: String,
    pub protocol: Protocol,
    pub host: String,
    pub port: u16,
}

pub struct DiscoveryDialog;

impl DiscoveryDialog {
    pub fn build_window<F>(
        parent: &impl IsA<gtk::Window>,
        on_add_callback: F,
    ) -> adw::Window
    where
        F: Fn(Connection) + 'static;
}
```

---

## 5. Verification Method

To independently verify the implementation when applied by `worker_m2`:

1. **Compilation Check**:
   ```bash
   cargo check
   ```
2. **Unit Tests**:
   ```bash
   cargo test
   ```
3. **Binary Build**:
   ```bash
   cargo build
   ```
4. **Runtime Verification**:
   - Run `cargo run`
   - Open Preferences dialog -> change theme to Dark / Light / System -> verify `~/.config/ver/config.json` updates automatically.
   - Open Network Discovery dialog -> verify spinner spins and discovered hosts populate list box.
