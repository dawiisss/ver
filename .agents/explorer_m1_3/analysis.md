# Milestone 1 Architectural & Unit Test Suite Design Analysis

**Agent**: `explorer_m1_3`  
**Directory**: `/home/dawiisss/Documents/antigravity/beautiful-goodall/.agents/explorer_m1_3`  
**Date**: 2026-08-12  
**Target Project**: `beautiful-goodall` (VER Rust Rewrite)

---

## 1. Executive Summary

This report establishes the complete module export architecture (`src/lib.rs` and `src/main.rs`), crate target layout in `Cargo.toml`, Serde data models, 4-space indented JSON storage engine, Secret Service keyring integration, and unit test suite specifications for Milestone 1.

The primary design goals met in this specification:
1. **Integration Test Ergonomics**: Clean separation between `lib.rs` (library target `beautiful_goodall`) and `main.rs` (binary target `beautiful-goodall`), enabling seamless importing by integration tests (`tests/e2e_data_tests.rs`, `tests/e2e_ui_tests.rs`, etc.).
2. **Python 4-Space Indent JSON Compatibility**: Custom `serde_json::Serializer` using `PrettyFormatter::with_indent(b"    ")` ensuring exact byte-level roundtrip compatibility with Python's `json.dump(..., indent=4)`.
3. **Resilient Deserialization**: Total default fallback coverage (`#[serde(default)]` on all fields/structs) ensuring empty `{}` or minimal JSON objects deserialize safely into valid Rust defaults.
4. **Secret Isolation**: Passwords handled strictly via `oo7` Secret Service under service `"ver_remote_connection_manager"`, completely isolated from `Connection` JSON serialization.

---

## 2. Module Export & Build Setup Architecture

### 2.1 `Cargo.toml` Target Configuration

To enable both executable running (`cargo run`) and external integration testing (`cargo test`), `Cargo.toml` specifies dual library/binary targets:

```toml
[package]
name = "beautiful-goodall"
version = "0.1.0"
edition = "2021"

[lib]
name = "beautiful_goodall"
path = "src/lib.rs"

[[bin]]
name = "beautiful-goodall"
path = "src/main.rs"

[dependencies]
gtk = { package = "gtk4", version = "0.7" }
libadwaita = "0.5"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
vnc = "0.4.0"
anyhow = "1.0"
oo7 = "0.3"
tokio = { version = "1.34", features = ["full"] }
uuid = { version = "1.6", features = ["v4", "serde"] }
dirs = "5.0"
tempfile = "3.8"
```

### 2.2 `src/lib.rs` Library Crate Architecture

`src/lib.rs` exports all core submodules as `pub mod` and re-exports high-level types for public crate access.

```rust
//! beautiful_goodall - Remote Connection Manager (VER Rust Rewrite)

pub mod launcher;
pub mod models;
pub mod network;
pub mod secrets;
pub mod storage;
pub mod ui;
pub mod vnc;

// Ergonomic re-exports for crate consumers & integration tests
pub use models::{AdvancedSettings, AppConfig, Connection, Protocol, VncScaling};
pub use secrets::{delete_password, get_password, set_password};
pub use storage::{load_config, load_connections, save_config, save_connections};
```

### 2.3 `src/main.rs` Application Entrypoint

`src/main.rs` contains only the executable entry point.

```rust
use beautiful_goodall::ui;
use libadwaita::prelude::*;

fn main() {
    let app = libadwaita::Application::builder()
        .application_id("com.example.ver")
        .build();

    app.connect_activate(|app| {
        let window = ui::window::MainWindow::new(app);
        window.present();
    });

    app.run();
}
```

### 2.4 Test Visibility Hierarchy

| Context | Target File | Import Method | Access Scope |
|---|---|---|---|
| **Unit Tests** | `src/models.rs`, `src/storage.rs`, `src/secrets.rs` | `#[cfg(test)] mod tests` | Private & public module items |
| **Integration Tests** | `tests/e2e_data_tests.rs`, `tests/e2e_ui_tests.rs` | `use beautiful_goodall::models::*;` | Public exported library API (`lib.rs`) |
| **Binary Entry** | `src/main.rs` | `use beautiful_goodall::ui;` | Public exported library API (`lib.rs`) |

---

## 3. Data Models (`src/models.rs`) Implementation Specification

```rust
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Rdp,
    Vnc,
    Ssh,
}

impl Default for Protocol {
    fn default() -> Self {
        Protocol::Rdp
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VncScaling {
    #[serde(rename = "Original Size")]
    OriginalSize,
    #[serde(rename = "Fit to Window")]
    FitToWindow,
    #[serde(rename = "Stretch")]
    Stretch,
}

impl Default for VncScaling {
    fn default() -> Self {
        VncScaling::OriginalSize
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdvancedSettings {
    #[serde(default)]
    pub rdp_multimon: bool,
    #[serde(default)]
    pub rdp_fullscreen: bool,
    #[serde(default)]
    pub rdp_audio: bool,
    #[serde(default)]
    pub vnc_viewonly: bool,
    #[serde(default)]
    pub vnc_shared: bool,
    #[serde(default = "default_true")]
    pub clipboard_sharing: bool,
    #[serde(default = "default_color_depth")]
    pub color_depth: u32,
    #[serde(default)]
    pub vnc_scaling: VncScaling,
}

fn default_true() -> bool {
    true
}

fn default_color_depth() -> u32 {
    32
}

impl Default for AdvancedSettings {
    fn default() -> Self {
        Self {
            rdp_multimon: false,
            rdp_fullscreen: false,
            rdp_audio: false,
            vnc_viewonly: false,
            vnc_shared: false,
            clipboard_sharing: true,
            color_depth: 32,
            vnc_scaling: VncScaling::OriginalSize,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Connection {
    #[serde(default = "default_id")]
    pub id: String,
    #[serde(default = "default_name")]
    pub name: String,
    #[serde(default)]
    pub protocol: Protocol,
    #[serde(default)]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub mac_address: String,
    #[serde(default = "default_group")]
    pub group: String,
    #[serde(default)]
    pub advanced_settings: AdvancedSettings,
}

fn default_id() -> String {
    Uuid::new_v4().to_string()
}

fn default_name() -> String {
    "New Connection".to_string()
}

fn default_port() -> u16 {
    3389
}

fn default_group() -> String {
    "Default".to_string()
}

impl Default for Connection {
    fn default() -> Self {
        Self {
            id: default_id(),
            name: default_name(),
            protocol: Protocol::default(),
            host: String::new(),
            port: default_port(),
            username: String::new(),
            mac_address: String::new(),
            group: default_group(),
            advanced_settings: AdvancedSettings::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_theme() -> String {
    "default".to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: default_theme(),
        }
    }
}
```

---

## 4. Storage Engine & Python Indentation Matching (`src/storage.rs`)

Python `json.dump(..., indent=4)` formats JSON output using 4 spaces per nesting level. `serde_json::to_string_pretty` defaults to 2 spaces. We resolve this by instantiating `Serializer::with_formatter(&mut buf, PrettyFormatter::with_indent(b"    "))`.

```rust
use anyhow::{Context, Result};
use std::fs::{self, File};
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use serde_json::ser::PrettyFormatter;
use serde_json::Serializer;

use crate::models::{AppConfig, Connection};

pub fn get_config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("ver")
}

pub fn get_connections_path() -> PathBuf {
    get_config_dir().join("connections.json")
}

pub fn get_config_path() -> PathBuf {
    get_config_dir().join("config.json")
}

pub fn to_json_4spaces<T: serde::Serialize>(data: &T) -> Result<String> {
    let mut buf = Vec::new();
    let formatter = PrettyFormatter::with_indent(b"    ");
    let mut ser = Serializer::with_formatter(&mut buf, formatter);
    data.serialize(&mut ser)?;
    let string = String::from_utf8(buf)?;
    Ok(string)
}

pub fn load_connections_from_path(path: &Path) -> Result<Vec<Connection>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = File::open(path).with_context(|| format!("Failed to open connections file: {:?}", path))?;
    let reader = BufReader::new(file);
    let connections: Vec<Connection> = serde_json::from_reader(reader)
        .with_context(|| format!("Failed to parse connections JSON from: {:?}", path))?;
    Ok(connections)
}

pub fn save_connections_to_path(connections: &[Connection], path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json_str = to_json_4spaces(&connections)?;
    let mut file = File::create(path)?;
    file.write_all(json_str.as_bytes())?;
    file.write_all(b"\n")?;
    Ok(())
}

pub fn load_connections() -> Result<Vec<Connection>> {
    load_connections_from_path(&get_connections_path())
}

pub fn save_connections(connections: &[Connection]) -> Result<()> {
    save_connections_to_path(connections, &get_connections_path())
}

pub fn load_config_from_path(path: &Path) -> Result<AppConfig> {
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let config: AppConfig = serde_json::from_reader(reader)?;
    Ok(config)
}

pub fn save_config_to_path(config: &AppConfig, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json_str = to_json_4spaces(&config)?;
    let mut file = File::create(path)?;
    file.write_all(json_str.as_bytes())?;
    file.write_all(b"\n")?;
    Ok(())
}

pub fn load_config() -> Result<AppConfig> {
    load_config_from_path(&get_config_path())
}

pub fn save_config(config: &AppConfig) -> Result<()> {
    save_config_to_path(config, &get_config_path())
}
```

---

## 5. Secrets Management Specifications (`src/secrets.rs`)

```rust
use anyhow::{Context, Result};
use oo7::Keyring;

const SERVICE_NAME: &str = "ver_remote_connection_manager";

pub async fn get_password(connection_id: &str) -> Result<Option<String>> {
    let keyring = Keyring::new().await.context("Failed to connect to Secret Service keyring")?;
    let attributes = [("service", SERVICE_NAME), ("connection_id", connection_id)];
    let items = keyring.search_items(&attributes).await?;
    if let Some(item) = items.first() {
        let secret = item.secret().await?;
        let pass_str = String::from_utf8(secret.to_vec())?;
        Ok(Some(pass_str))
    } else {
        Ok(None)
    }
}

pub async fn set_password(connection_id: &str, password: &str) -> Result<()> {
    let keyring = Keyring::new().await.context("Failed to connect to Secret Service keyring")?;
    let attributes = [("service", SERVICE_NAME), ("connection_id", connection_id)];
    let label = format!("VER Connection Password ({})", connection_id);
    keyring.create_item(&label, &attributes, password.as_bytes(), true).await?;
    Ok(())
}

pub async fn delete_password(connection_id: &str) -> Result<()> {
    let keyring = Keyring::new().await.context("Failed to connect to Secret Service keyring")?;
    let attributes = [("service", SERVICE_NAME), ("connection_id", connection_id)];
    let items = keyring.search_items(&attributes).await?;
    for item in items {
        item.delete().await?;
    }
    Ok(())
}
```

---

## 6. Unit Test Suite Specifications

### 6.1 `src/models.rs` Unit Test Suite (`#[cfg(test)] mod tests`)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_defaults() {
        let conn = Connection::default();
        assert!(!conn.id.is_empty());
        assert_eq!(conn.name, "New Connection");
        assert_eq!(conn.protocol, Protocol::Rdp);
        assert_eq!(conn.port, 3389);
        assert_eq!(conn.group, "Default");
        assert_eq!(conn.advanced_settings.color_depth, 32);
        assert_eq!(conn.advanced_settings.clipboard_sharing, true);
        assert_eq!(conn.advanced_settings.vnc_scaling, VncScaling::OriginalSize);
    }

    #[test]
    fn test_deserialize_empty_json_object() {
        let json_data = "{}";
        let conn: Connection = serde_json::from_str(json_data).expect("Should deserialize empty JSON object into defaults");
        assert!(!conn.id.is_empty());
        assert_eq!(conn.name, "New Connection");
        assert_eq!(conn.protocol, Protocol::Rdp);
        assert_eq!(conn.port, 3389);
        assert_eq!(conn.group, "Default");
        assert_eq!(conn.advanced_settings.color_depth, 32);
    }

    #[test]
    fn test_deserialize_minimal_partial_json() {
        let json_data = r#"{
            "id": "fixed-uuid-1234",
            "name": "Production Jump",
            "protocol": "vnc",
            "host": "10.0.0.50"
        }"#;
        let conn: Connection = serde_json::from_str(json_data).expect("Should deserialize partial JSON");
        assert_eq!(conn.id, "fixed-uuid-1234");
        assert_eq!(conn.name, "Production Jump");
        assert_eq!(conn.protocol, Protocol::Vnc);
        assert_eq!(conn.host, "10.0.0.50");
        assert_eq!(conn.port, 3389);
        assert_eq!(conn.group, "Default");
        assert_eq!(conn.advanced_settings.clipboard_sharing, true);
    }

    #[test]
    fn test_deserialize_unknown_json_fields() {
        let json_data = r#"{
            "id": "uuid-999",
            "name": "Legacy Conn",
            "unknown_legacy_field_1": 12345,
            "deprecated_flag": true,
            "advanced_settings": {
                "extra_setting": "foo"
            }
        }"#;
        let conn: Connection = serde_json::from_str(json_data).expect("Should ignore unknown JSON fields cleanly");
        assert_eq!(conn.id, "uuid-999");
        assert_eq!(conn.name, "Legacy Conn");
    }

    #[test]
    fn test_protocol_enum_serde_representations() {
        assert_eq!(serde_json::to_string(&Protocol::Rdp).unwrap(), r#""rdp""#);
        assert_eq!(serde_json::to_string(&Protocol::Vnc).unwrap(), r#""vnc""#);
        assert_eq!(serde_json::to_string(&Protocol::Ssh).unwrap(), r#""ssh""#);

        let p_rdp: Protocol = serde_json::from_str(r#""rdp""#).unwrap();
        let p_vnc: Protocol = serde_json::from_str(r#""vnc""#).unwrap();
        let p_ssh: Protocol = serde_json::from_str(r#""ssh""#).unwrap();

        assert_eq!(p_rdp, Protocol::Rdp);
        assert_eq!(p_vnc, Protocol::Vnc);
        assert_eq!(p_ssh, Protocol::Ssh);
    }

    #[test]
    fn test_vnc_scaling_enum_serde_representations() {
        assert_eq!(serde_json::to_string(&VncScaling::OriginalSize).unwrap(), r#""Original Size""#);
        assert_eq!(serde_json::to_string(&VncScaling::FitToWindow).unwrap(), r#""Fit to Window""#);
        assert_eq!(serde_json::to_string(&VncScaling::Stretch).unwrap(), r#""Stretch""#);

        let s_orig: VncScaling = serde_json::from_str(r#""Original Size""#).unwrap();
        let s_fit: VncScaling = serde_json::from_str(r#""Fit to Window""#).unwrap();
        let s_stretch: VncScaling = serde_json::from_str(r#""Stretch""#).unwrap();

        assert_eq!(s_orig, VncScaling::OriginalSize);
        assert_eq!(s_fit, VncScaling::FitToWindow);
        assert_eq!(s_stretch, VncScaling::Stretch);
    }

    #[test]
    fn test_password_isolation_in_json_schema() {
        let conn = Connection::default();
        let serialized = serde_json::to_string(&conn).expect("Serialization must succeed");
        assert!(!serialized.contains("password"));
        assert!(!serialized.contains("secret"));
        assert!(!serialized.contains("keyring"));
    }
}
```

### 6.2 `src/storage.rs` Unit Test Suite (`#[cfg(test)] mod tests`)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AdvancedSettings, Connection, Protocol, VncScaling};
    use tempfile::tempdir;

    #[test]
    fn test_python_4space_indent_formatting() {
        let mut conn = Connection::default();
        conn.id = "test-uuid-1234".to_string();
        conn.name = "Test Server".to_string();
        conn.protocol = Protocol::Vnc;
        conn.host = "192.168.1.100".to_string();
        conn.port = 5900;
        conn.username = "admin".to_string();
        conn.group = "Servers".to_string();

        let connections = vec![conn];
        let json_str = to_json_4spaces(&connections).expect("JSON serialization failed");

        let lines: Vec<&str> = json_str.lines().collect();
        assert!(lines.len() > 5);
        assert!(lines[1].starts_with("    {"), "Line 1 must start with 4 spaces: '{}'", lines[1]);
        assert!(lines[2].starts_with("        \"id\":"), "Line 2 must start with 8 spaces: '{}'", lines[2]);
    }

    #[test]
    fn test_roundtrip_storage_save_load() {
        let dir = tempdir().expect("Failed to create temp dir");
        let file_path = dir.path().join("connections.json");

        let mut conn1 = Connection::default();
        conn1.id = "id-1".to_string();
        conn1.name = "Server 1".to_string();
        conn1.protocol = Protocol::Rdp;

        let mut conn2 = Connection::default();
        conn2.id = "id-2".to_string();
        conn2.name = "Server 2".to_string();
        conn2.protocol = Protocol::Vnc;
        conn2.advanced_settings.vnc_scaling = VncScaling::FitToWindow;

        let original = vec![conn1, conn2];

        save_connections_to_path(&original, &file_path).expect("Save must succeed");
        assert!(file_path.exists());

        let loaded = load_connections_from_path(&file_path).expect("Load must succeed");
        assert_eq!(original, loaded, "Loaded connections must match original exactly");
    }

    #[test]
    fn test_load_nonexistent_file_returns_empty_vec() {
        let dir = tempdir().expect("Failed to create temp dir");
        let non_existent_path = dir.path().join("does_not_exist.json");

        let result = load_connections_from_path(&non_existent_path);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_load_corrupted_json_returns_error() {
        let dir = tempdir().expect("Failed to create temp dir");
        let corrupted_path = dir.path().join("corrupted.json");

        fs::write(&corrupted_path, "{ invalid json content ...").unwrap();

        let result = load_connections_from_path(&corrupted_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_save_config_roundtrip() {
        let dir = tempdir().expect("Failed to create temp dir");
        let config_path = dir.path().join("config.json");

        let config = AppConfig {
            theme: "dark".to_string(),
        };

        save_config_to_path(&config, &config_path).expect("Save config must succeed");
        let loaded = load_config_from_path(&config_path).expect("Load config must succeed");
        assert_eq!(config, loaded);
    }
}
```

### 6.3 `src/secrets.rs` Unit Test Suite (`#[cfg(test)] mod tests`)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secrets_service_name_constant() {
        assert_eq!(SERVICE_NAME, "ver_remote_connection_manager");
    }

    #[tokio::test]
    #[ignore = "Requires active Secret Service D-Bus daemon"]
    async fn test_keyring_password_lifecycle() {
        let test_id = "test-uuid-unit-test-999";
        let test_pass = "super_secret_p@ssw0rd";

        let _ = delete_password(test_id).await;

        let initial = get_password(test_id).await.expect("Keyring query should succeed");
        assert_eq!(initial, None);

        set_password(test_id, test_pass).await.expect("Set password should succeed");

        let retrieved = get_password(test_id).await.expect("Get password should succeed");
        assert_eq!(retrieved, Some(test_pass.to_string()));

        delete_password(test_id).await.expect("Delete password should succeed");

        let after_delete = get_password(test_id).await.expect("Keyring query should succeed");
        assert_eq!(after_delete, None);
    }
}
```

---

## 7. Verification Method

1. Verify Cargo builds `lib` and `bin` targets seamlessly:
   `cargo build --lib` and `cargo build --bin beautiful-goodall`.
2. Run unit tests across all modules:
   `cargo test --lib`.
3. Validate 4-space JSON output against Python test vectors:
   `python3 -c 'import json; print(json.dumps([{"id": "1"}], indent=4))'` vs `to_json_4spaces(&vec![conn])`.
