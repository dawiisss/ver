# Explorer M2 3: Technical Analysis & UI Architecture Report

## 1. Executive Summary & Objective

This report details the architectural investigation and implementation design for Milestone 2 UI components:
- **`src/models.rs`**: Extension of `AppConfig` to support all global preferences with backwards-compatible Serde serialization.
- **`src/ui/preferences.rs`**: `PreferencesWindow` modal dialog (`adw::PreferencesWindow`) for global settings (Theme, Default Protocol, Auto-connect, VNC Scaling) with automatic persistence to `~/.config/ver/config.json`.
- **`src/ui/discovery.rs`**: `DiscoveryDialog` modal window for scanning local network subnets for VNC (5900), RDP (3389), and SSH (22) hosts with asynchronous GLib main-thread UI dispatch and "Add Connection" actions.
- **`src/main.rs` & `src/ui/mod.rs`**: Full GTK4 / Libadwaita `adw::Application` entrypoint initialization, theme styling synchronization, and lifecycle management.

---

## 2. Codebase Baseline & Direct Observations

### 2.1 Storage & Data Models Analysis
- **`src/models.rs` (Lines 239–256)**:
  ```rust
  fn default_theme() -> String { "default".to_string() }
  pub struct AppConfig {
      #[serde(default = "default_theme")]
      pub theme: String,
  }
  ```
  *Observation*: Currently `AppConfig` only holds `theme: String`. It lacks fields for default protocol, auto-connect last session, default VNC scaling option, or last connected connection ID.

- **`src/storage.rs` (Lines 130–189)**:
  - `load_config_from_path(path: &Path) -> Result<AppConfig>`: Safely loads `AppConfig` from JSON, creating default `AppConfig` if missing or corrupted (with automatic `.corrupt.TIMESTAMP` backup creation).
  - `save_config_to_path(path: &Path, config: &AppConfig) -> Result<()>`: Atomically writes `AppConfig` with 4-space JSON formatting via `tempfile::NamedTempFile`.
  - `load_config() -> Result<AppConfig>` and `save_config(config: &AppConfig) -> Result<()>` target `~/.config/ver/config.json`.

- **`src/main.rs` (Lines 1–21)**:
  - Currently a minimal stub initializing `libadwaita::Application` and presenting a bare `libadwaita::ApplicationWindow`.
  - Does not currently load `AppConfig` or `Connection` vector from disk, nor does it hook up `MainWindow`.

- **`src/ui/mod.rs` (Lines 1–10)**:
  - Currently exports `MainWindow`, `ConnectionEditor`, `PreferencesWindow`, `DiscoveredService`, and `DiscoveryDialog`.

- **`src/ui/preferences.rs` (Lines 1–16)**:
  - Currently a non-GTK struct stub containing `config: AppConfig`.

- **`src/ui/discovery.rs` (Lines 1–24)**:
  - Currently a stub struct `DiscoveryDialog` with `discovered_services: Vec<DiscoveredService>`.

---

## 3. Detailed Component Designs

### 3.1 `AppConfig` Model Extension (`src/models.rs`)

To support all UI preferences without breaking existing `config.json` files on disk, `AppConfig` is extended using Serde defaults:

```rust
fn default_theme() -> String {
    "system".to_string()
}

/// Global application configuration model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    #[serde(default = "default_theme")]
    pub theme: String, // "system", "dark", "light"
    pub default_protocol: Protocol, // Protocol::Rdp, Protocol::Vnc, Protocol::Ssh
    pub auto_connect_last: bool, // false / true
    pub default_vnc_scaling: VncScaling, // VncScaling::OriginalSize, FitToWindow, Stretch
    pub last_connected_id: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            default_protocol: Protocol::Rdp,
            auto_connect_last: false,
            default_vnc_scaling: VncScaling::OriginalSize,
            last_connected_id: None,
        }
    }
}
```

---

### 3.2 Preferences Window Design (`src/ui/preferences.rs`)

`PreferencesWindow` builds an `adw::PreferencesWindow` modal dialog attached to `MainWindow`.

#### Structure & Signatures
```rust
use std::cell::RefCell;
use std::rc::Rc;
use libadwaita::prelude::*;
use gtk::prelude::*;
use crate::models::{AppConfig, Protocol, VncScaling};
use crate::storage::save_config;

pub struct PreferencesWindow;

impl PreferencesWindow {
    pub fn build_window(
        parent: &impl IsA<gtk::Window>,
        config: Rc<RefCell<AppConfig>>,
    ) -> adw::PreferencesWindow {
        // Implementation details
    }
}

pub fn apply_theme(theme_str: &str) {
    let style_manager = adw::StyleManager::default();
    match theme_str {
        "dark" => style_manager.set_color_scheme(adw::ColorScheme::ForceDark),
        "light" => style_manager.set_color_scheme(adw::ColorScheme::ForceLight),
        _ => style_manager.set_color_scheme(adw::ColorScheme::Default),
    }
}
```

#### Preferences Layout Hierarchy
1. **Window**: `adw::PreferencesWindow`
   - `set_title(Some("Preferences"))`
   - `set_transient_for(Some(parent))`
   - `set_modal(true)`
   - `set_default_size(520, 480)`

2. **Page 1: General Settings** (`adw::PreferencesPage`, title: `"General"`, icon: `"preferences-system-symbolic"`)
   - **Group 1: Appearance** (`adw::PreferencesGroup`, title: `"Appearance"`)
     - `adw::ComboRow` ("Application Theme")
       - Model: `gtk::StringList::new(&["System Default", "Dark Mode", "Light Mode"])`
       - Initial index: mapped from `config.borrow().theme` (`"system"` -> 0, `"dark"` -> 1, `"light"` -> 2).
       - Signal `connect_selected_notify`:
         ```rust
         let config_clone = config.clone();
         theme_row.connect_selected_notify(move |row| {
             let idx = row.selected();
             let theme = match idx {
                 1 => "dark",
                 2 => "light",
                 _ => "system",
             };
             config_clone.borrow_mut().theme = theme.to_string();
             apply_theme(theme);
             let _ = save_config(&config_clone.borrow());
         });
         ```
   - **Group 2: Default Connection Settings** (`adw::PreferencesGroup`, title: `"Defaults & Behavior"`)
     - `adw::ComboRow` ("Default Protocol")
       - Model: `gtk::StringList::new(&["RDP", "VNC", "SSH"])`
       - Initial index: `Protocol::Rdp` -> 0, `Protocol::Vnc` -> 1, `Protocol::Ssh` -> 2.
       - Signal `connect_selected_notify`: updates `config.borrow_mut().default_protocol` and calls `save_config`.
     - `adw::SwitchRow` ("Auto-connect Last Session")
       - Subtitle: `"Automatically launch the last used connection on startup"`
       - Active state: `config.borrow().auto_connect_last`
       - Signal `connect_active_notify`: updates `config.borrow_mut().auto_connect_last` and calls `save_config`.
     - `adw::ComboRow` ("Default VNC Display Scaling")
       - Model: `gtk::StringList::new(&["Original Size", "Fit to Window", "Stretch"])`
       - Initial index: `VncScaling::OriginalSize` -> 0, `VncScaling::FitToWindow` -> 1, `VncScaling::Stretch` -> 2.
       - Signal `connect_selected_notify`: updates `config.borrow_mut().default_vnc_scaling` and calls `save_config`.

---

### 3.3 Network Discovery Dialog Design (`src/ui/discovery.rs`)

`DiscoveryDialog` provides an interactive subnet scanner and results list.

#### Structure & Data Types
```rust
use std::sync::Arc;
use std::time::Duration;
use libadwaita::prelude::*;
use gtk::prelude::*;
use crate::models::{Connection, Protocol};

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
        F: Fn(Connection) + 'static,
    {
        // Implementation details
    }
}
```

#### Asynchronous Subnet Scanning & GTK Dispatch Architecture
1. **UI Layout**:
   - `adw::Window`: title `"Discover Network Devices"`, default size `(480, 560)`, modal `true`.
   - HeaderBar with title widget `"Network Discovery"` and a manual Refresh button (`view-refresh-symbolic`).
   - Spinner header bar area: `gtk::Spinner` + `gtk::Label` ("Scanning local network for VNC, RDP, SSH hosts...").
   - Central `gtk::ScrolledWindow` containing `gtk::ListBox` (`SelectionMode::None`).
   - Placeholder widget: `adw::StatusPage` displayed when no services are discovered.

2. **Async Background Scanning**:
   - Uses `glib::MainContext::channel::<Option<DiscoveredService>>(glib::Priority::default())`.
   - Receiver attached to GLib main thread loop.
   - Background Tokio task / thread probes IPs across local subnets (e.g. `192.168.1.1..=254` or local network interfaces) on target ports (5900, 3389, 22) with a 250ms TCP connect timeout.
   - When a port opens, `sender.send(Some(service))` is triggered.
   - When the subnet scan finishes, `sender.send(None)` is sent to stop the `gtk::Spinner` and show status.

3. **Discovered Item Action Row**:
   - Each received `DiscoveredService` creates an `adw::ActionRow`:
     - Title: `service.name` (e.g., `"192.168.1.45"`)
     - Subtitle: `format!("{}:{} ({})", service.host, service.port, service.protocol.as_str().to_uppercase())`
     - Icon: `network-server-symbolic` / `computer-symbolic`
     - Suffix widget: `gtk::Button` labeled `"Add"` (`suggested-action`).
     - On click:
       - Constructs `Connection` pre-populated with host, port, protocol, name.
       - Calls `on_add_callback(connection)`.
       - Sets button sensitive to `false` and label to `"Added"`.

---

### 3.4 Application Entrypoint Integration (`src/main.rs`, `src/lib.rs`, `src/ui/mod.rs`)

#### `src/lib.rs` Export Contract
```rust
pub mod launcher;
pub mod models;
pub mod network;
pub mod secrets;
pub mod storage;
pub mod ui;
pub mod vnc;

pub use models::{AdvancedSettings, AppConfig, Connection, Protocol, VncScaling};
pub use secrets::{
    delete_password, delete_password_sync, get_password, get_password_sync, set_password,
    set_password_sync,
};
pub use storage::{
    get_config_dir, get_config_file_path, get_connections_file_path, load_config,
    load_config_from_path, load_connections, load_connections_from_path, save_config,
    save_config_to_path, save_connections, save_connections_to_path,
};
pub use ui::{ConnectionEditor, DiscoveredService, DiscoveryDialog, MainWindow, PreferencesWindow};
```

#### `src/ui/mod.rs` Module Organization
```rust
pub mod discovery;
pub mod editor;
pub mod preferences;
pub mod window;

pub use discovery::{DiscoveredService, DiscoveryDialog};
pub use editor::ConnectionEditor;
pub use preferences::{apply_theme, PreferencesWindow};
pub use window::MainWindow;
```

#### `src/main.rs` GTK Application Lifecycle
```rust
use libadwaita::prelude::*;
use beautiful_goodall::{load_config, load_connections, preferences::apply_theme, ui::MainWindow};

fn main() {
    // Initialize Libadwaita
    libadwaita::init().expect("Failed to initialize Libadwaita");

    let app = libadwaita::Application::builder()
        .application_id("com.example.ver")
        .build();

    app.connect_activate(|app| {
        // 1. Load global configuration (~/.config/ver/config.json)
        let config = load_config().unwrap_or_default();

        // 2. Apply theme scheme via adw::StyleManager
        apply_theme(&config.theme);

        // 3. Load connections (~/.config/ver/connections.json)
        let connections = load_connections().unwrap_or_default();

        // 4. Construct and present MainWindow
        let window = MainWindow::build_ui(app, connections, config);
        window.present();
    });

    app.run();
}
```

---

## 4. Implementation Plan for `worker_m2`

1. **Step 1 (`src/models.rs`)**:
   - Update `AppConfig` struct to include `theme`, `default_protocol`, `auto_connect_last`, `default_vnc_scaling`, and `last_connected_id`.
   - Add unit tests validating default values and JSON roundtrip serialization.

2. **Step 2 (`src/ui/preferences.rs`)**:
   - Implement `PreferencesWindow::build_window` with `adw::PreferencesWindow`, `adw::PreferencesPage`, `adw::PreferencesGroup`, `adw::ComboRow`, `adw::SwitchRow`.
   - Implement `apply_theme` helper function using `adw::StyleManager`.
   - Hook up signal handlers to auto-save config via `storage::save_config`.

3. **Step 3 (`src/ui/discovery.rs`)**:
   - Implement `DiscoveryDialog::build_window` using `adw::Window`, `gtk::HeaderBar`, `gtk::Spinner`, `gtk::ListBox`, and `adw::ActionRow`.
   - Implement async subnet scanning with `glib::MainContext::channel` dispatch to main thread.
   - Implement "Add Connection" callback execution on button click.

4. **Step 4 (`src/ui/mod.rs` & `src/lib.rs`)**:
   - Export `apply_theme`, `PreferencesWindow`, `DiscoveryDialog`, and `DiscoveredService`.

5. **Step 5 (`src/main.rs`)**:
   - Update `main.rs` to invoke `libadwaita::init()`, load config/connections from `storage`, apply theme, and launch `MainWindow`.
