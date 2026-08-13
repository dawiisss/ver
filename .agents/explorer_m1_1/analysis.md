# Milestone 1 (R1: Rust Skeleton & Serde Data Models) Technical Implementation Design

**Author:** explorer_m1_1  
**Directory:** `/home/dawiisss/Documents/antigravity/beautiful-goodall/.agents/explorer_m1_1`  
**Date:** 2026-08-12  
**Target Application:** VER - Very Easy Remote (beautiful-goodall)

---

## 1. Overview & Objectives

Milestone 1 (R1) establishes the core Rust architecture foundation for the VER connection manager. The primary objective is to replace the Python data layer (`models.py`, `core/storage.py`, `core/config.py`, `core/secrets.py`) with high-performance, idiomatically Rust-native modules, ensuring **100% loss-less reading, editing, and writing** of user configuration files and secrets.

This document provides exact, copy-pasteable implementation specifications for:
1. `Cargo.toml` package & dependency declaration.
2. `src/models.rs` Serde data models (`Connection`, `AdvancedSettings`, `AppConfig`, `Protocol`, `VncScaling`).
3. `src/storage.rs` 4-space indented JSON persistence engine.
4. `src/secrets.rs` Secret Service password manager via `oo7`.
5. Unit test suite specification for data model roundtrips, missing field fallbacks, and indentation compliance.

---

## 2. Cargo Package & Dependencies Specification (`Cargo.toml`)

### `Cargo.toml`
```toml
[package]
name = "beautiful-goodall"
version = "0.1.0"
edition = "2021"

[dependencies]
gtk4 = { version = "0.7", package = "gtk4" }
libadwaita = "0.5"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
vnc = "0.4.0"
oo7 = "0.3"
tokio = { version = "1.34", features = ["full"] }
anyhow = "1.0"
uuid = { version = "1.6", features = ["v4", "serde"] }
dirs = "5.0"

[dev-dependencies]
tempfile = "3.8"
```

### Key Dependency Rationale
- `gtk4` & `libadwaita`: Provides GTK4/Libadwaita UI bindings.
- `serde` & `serde_json`: High-performance JSON serialization/deserialization.
- `vnc`: Pure Rust RFB protocol implementation (v0.4.0).
- `oo7`: Modern Secret Service DBus library for GTK/Libadwaita applications.
- `tokio`: Async runtime supporting background VNC networking and secret service DBus calls.
- `uuid`: Enables `Uuid::new_v4()` for auto-generating unique connection IDs with Serde support.
- `dirs`: Cross-platform determination of user config directory (`~/.config/ver`).
- `tempfile`: Isolated temporary directory creation for storage unit tests.

---

## 3. Serde Data Models Specification (`src/models.rs`)

`src/models.rs` defines the Rust data representations corresponding to Python `dataclass` models and JSON schemas.

### Field Attributes & Serde Annotations Summary
- `Protocol`: `#[serde(rename_all = "lowercase")]` maps enum variants (`Rdp`, `Vnc`, `Ssh`) to `"rdp"`, `"vnc"`, `"ssh"`.
- `VncScaling`: `#[serde(rename = "...")]` maps variants (`OriginalSize`, `FitToWindow`, `Stretch`) to exact string representations `"Original Size"`, `"Fit to Window"`, `"Stretch"`.
- `AdvancedSettings`: `#[serde(default)]` on every field ensures missing keys in JSON default to `false`, `0`, or `VncScaling::OriginalSize`.
- `Connection`: `#[serde(default = "...")]` on all fields guarantees seamless deserialization of sparse or legacy JSON objects.
- `AppConfig`: `#[serde(default = "default_theme")]` defaults `theme` to `"default"`.

### Complete Code Specification (`src/models.rs`)

```rust
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Supported remote connection protocols.
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

impl Protocol {
    pub fn default_port(&self) -> u16 {
        match self {
            Protocol::Rdp => 3389,
            Protocol::Vnc => 5900,
            Protocol::Ssh => 22,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Protocol::Rdp => "rdp",
            Protocol::Vnc => "vnc",
            Protocol::Ssh => "ssh",
        }
    }
}

impl std::fmt::Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// VNC display scaling modes.
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

impl VncScaling {
    pub fn as_str(&self) -> &'static str {
        match self {
            VncScaling::OriginalSize => "Original Size",
            VncScaling::FitToWindow => "Fit to Window",
            VncScaling::Stretch => "Stretch",
        }
    }
}

impl std::fmt::Display for VncScaling {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Advanced settings for connection parameters.
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
    #[serde(default)]
    pub clipboard_sharing: bool,
    #[serde(default)]
    pub color_depth: u32,
    #[serde(default)]
    pub vnc_scaling: VncScaling,
}

impl Default for AdvancedSettings {
    fn default() -> Self {
        Self {
            rdp_multimon: false,
            rdp_fullscreen: false,
            rdp_audio: false,
            vnc_viewonly: false,
            vnc_shared: false,
            clipboard_sharing: false,
            color_depth: 0,
            vnc_scaling: VncScaling::OriginalSize,
        }
    }
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

/// Primary remote connection entity.
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

impl Default for Connection {
    fn default() -> Self {
        Self {
            id: default_id(),
            name: default_name(),
            protocol: Protocol::Rdp,
            host: String::new(),
            port: 3389,
            username: String::new(),
            mac_address: String::new(),
            group: default_group(),
            advanced_settings: AdvancedSettings::default(),
        }
    }
}

impl Connection {
    pub fn new_with_protocol(protocol: Protocol) -> Self {
        let port = protocol.default_port();
        Self {
            protocol,
            port,
            ..Default::default()
        }
    }
}

fn default_theme() -> String {
    "default".to_string()
}

/// Global application configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_theme")]
    pub theme: String,
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

## 4. JSON Storage Engine Specification (`src/storage.rs`)

`src/storage.rs` manages file persistence in `~/.config/ver/`.

### 4-Space Indentation Formatting
Standard `serde_json::to_string_pretty` outputs 2 spaces. Python `json.dump(..., indent=4)` formats JSON with 4 spaces. To guarantee exact format parity, `src/storage.rs` utilizes `serde_json::ser::PrettyFormatter::with_indent(b"    ")`.

### Path Resolution & Directory Auto-Creation
- Connections path: `~/.config/ver/connections.json`
- Config path: `~/.config/ver/config.json`
- Missing directory check: `fs::create_dir_all` automatically invoked prior to writing files.
- Missing file fallback: `load_connections` returns `Ok(vec![])` if `connections.json` is missing; `load_config` returns `Ok(AppConfig::default())` if `config.json` is missing.

### Complete Code Specification (`src/storage.rs`)

```rust
use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::ser::PrettyFormatter;
use serde_json::Serializer;

use crate::models::{AppConfig, Connection};

/// Returns the configuration directory path (~/.config/ver).
pub fn get_config_dir() -> Result<PathBuf> {
    if let Some(config_base) = dirs::config_dir() {
        Ok(config_base.join("ver"))
    } else {
        let home = std::env::var("HOME").context("HOME environment variable not set")?;
        Ok(PathBuf::from(home).join(".config").join("ver"))
    }
}

/// Returns the full path to connections.json (~/.config/ver/connections.json).
pub fn get_connections_file_path() -> Result<PathBuf> {
    Ok(get_config_dir()?.join("connections.json"))
}

/// Returns the full path to config.json (~/.config/ver/config.json).
pub fn get_config_file_path() -> Result<PathBuf> {
    Ok(get_config_dir()?.join("config.json"))
}

/// Helper function to format serializable data to JSON string with 4-space indentation.
pub fn to_json_4_spaces<T: Serialize>(data: &T) -> Result<String> {
    let mut buf = Vec::new();
    let formatter = PrettyFormatter::with_indent(b"    ");
    let mut serializer = Serializer::with_formatter(&mut buf, formatter);
    data.serialize(&mut serializer)
        .context("Failed to serialize data to JSON with 4-space indent")?;
    buf.push(b'\n');
    let json_str = String::from_utf8(buf)
        .context("Serialized JSON is not valid UTF-8")?;
    Ok(json_str)
}

/// Loads connections from a specific file path.
/// Returns an empty vector if the file does not exist.
pub fn load_connections_from_path(path: &Path) -> Result<Vec<Connection>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read connections file at {:?}", path))?;
    if content.trim().is_empty() {
        return Ok(Vec::new());
    }
    let connections: Vec<Connection> = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse connections JSON from {:?}", path))?;
    Ok(connections)
}

/// Saves connections to a specific file path using 4-space indentation.
/// Automatically creates parent directories if needed.
pub fn save_connections_to_path(path: &Path, connections: &[Connection]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory structure for {:?}", parent))?;
    }
    let json_str = to_json_4_spaces(connections)?;
    fs::write(path, json_str)
        .with_context(|| format!("Failed to write connections file to {:?}", path))?;
    Ok(())
}

/// Loads connections from standard location (~/.config/ver/connections.json).
pub fn load_connections() -> Result<Vec<Connection>> {
    let path = get_connections_file_path()?;
    load_connections_from_path(&path)
}

/// Saves connections to standard location (~/.config/ver/connections.json).
pub fn save_connections(connections: &[Connection]) -> Result<()> {
    let path = get_connections_file_path()?;
    save_connections_to_path(&path, connections)
}

/// Loads app configuration from a specific file path.
/// Returns default AppConfig if the file does not exist.
pub fn load_config_from_path(path: &Path) -> Result<AppConfig> {
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file at {:?}", path))?;
    if content.trim().is_empty() {
        return Ok(AppConfig::default());
    }
    let config: AppConfig = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse config JSON from {:?}", path))?;
    Ok(config)
}

/// Saves app configuration to a specific file path using 4-space indentation.
/// Automatically creates parent directories if needed.
pub fn save_config_to_path(path: &Path, config: &AppConfig) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory structure for {:?}", parent))?;
    }
    let json_str = to_json_4_spaces(config)?;
    fs::write(path, json_str)
        .with_context(|| format!("Failed to write config file to {:?}", path))?;
    Ok(())
}

/// Loads app configuration from standard location (~/.config/ver/config.json).
pub fn load_config() -> Result<AppConfig> {
    let path = get_config_file_path()?;
    load_config_from_path(&path)
}

/// Saves app configuration to standard location (~/.config/ver/config.json).
pub fn save_config(config: &AppConfig) -> Result<()> {
    let path = get_config_file_path()?;
    save_config_to_path(&path, config)
}
```

---

## 5. Secret Keyring Integration Specification (`src/secrets.rs`)

`src/secrets.rs` provides password retrieval and storage using the `oo7` crate for Linux Secret Service API (Freedesktop Keyring).

### Service Parameters
- Service Name: `"ver_remote_connection_manager"`
- Secret Lookup Key: Connection `id` (UUID v4 string)
- Legacy Lookup Fallback: Checks `"username"` attribute for entries generated by Python `keyring` package.
- Security Constraint: Passwords are NEVER written to disk or JSON files.

### Complete Code Specification (`src/secrets.rs`)

```rust
use anyhow::{Context, Result};
use oo7::Keyring;

const SERVICE_NAME: &str = "ver_remote_connection_manager";

/// Retrieves password for connection ID from Secret Service (oo7 keyring).
pub async fn get_password(id: &str) -> Result<Option<String>> {
    let keyring = match Keyring::new().await {
        Ok(k) => k,
        Err(e) => {
            eprintln!("Warning: Secret Service keyring unavailable: {}", e);
            return Ok(None);
        }
    };

    // Primary search using "service" and "connection_id"
    let items = keyring
        .search_items([("service", SERVICE_NAME), ("connection_id", id)])
        .await
        .context("Failed to search secret keyring for connection password")?;

    if let Some(item) = items.first() {
        let secret_bytes = item.secret().await.context("Failed to retrieve secret bytes")?;
        let password = String::from_utf8(secret_bytes.to_vec())
            .context("Secret is not valid UTF-8")?;
        return Ok(Some(password));
    }

    // Legacy fallback search matching Python keyring attributes ("username" = id)
    let legacy_items = keyring
        .search_items([("service", SERVICE_NAME), ("username", id)])
        .await
        .unwrap_or_default();

    if let Some(item) = legacy_items.first() {
        let secret_bytes = item.secret().await.context("Failed to retrieve secret bytes")?;
        let password = String::from_utf8(secret_bytes.to_vec())
            .context("Secret is not valid UTF-8")?;
        return Ok(Some(password));
    }

    Ok(None)
}

/// Stores password for connection ID in Secret Service (oo7 keyring).
pub async fn set_password(id: &str, password: &str) -> Result<()> {
    let keyring = Keyring::new().await.context("Failed to connect to Secret Service keyring")?;
    let label = format!("VER Connection Password ({})", id);
    
    keyring
        .create_item(
            &label,
            &[
                ("service", SERVICE_NAME),
                ("connection_id", id),
                ("username", id),
            ],
            password.as_bytes(),
            true,
        )
        .await
        .context("Failed to store password in Secret Service keyring")?;

    Ok(())
}

/// Deletes stored password for connection ID from Secret Service.
pub async fn delete_password(id: &str) -> Result<()> {
    let keyring = match Keyring::new().await {
        Ok(k) => k,
        Err(_) => return Ok(()),
    };

    let items = keyring
        .search_items([("service", SERVICE_NAME), ("connection_id", id)])
        .await
        .unwrap_or_default();

    for item in items {
        let _ = item.delete().await;
    }

    let legacy_items = keyring
        .search_items([("service", SERVICE_NAME), ("username", id)])
        .await
        .unwrap_or_default();

    for item in legacy_items {
        let _ = item.delete().await;
    }

    Ok(())
}

/// Synchronous wrapper around get_password for non-async contexts.
pub fn get_password_sync(id: &str) -> Result<Option<String>> {
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        tokio::task::block_in_place(|| handle.block_on(get_password(id)))
    } else {
        let rt = tokio::runtime::Runtime::new().context("Failed to create Tokio runtime")?;
        rt.block_on(get_password(id))
    }
}

/// Synchronous wrapper around set_password for non-async contexts.
pub fn set_password_sync(id: &str, password: &str) -> Result<()> {
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        tokio::task::block_in_place(|| handle.block_on(set_password(id, password)))
    } else {
        let rt = tokio::runtime::Runtime::new().context("Failed to create Tokio runtime")?;
        rt.block_on(set_password(id, password))
    }
}

/// Synchronous wrapper around delete_password for non-async contexts.
pub fn delete_password_sync(id: &str) -> Result<()> {
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        tokio::task::block_in_place(|| handle.block_on(delete_password(id)))
    } else {
        let rt = tokio::runtime::Runtime::new().context("Failed to create Tokio runtime")?;
        rt.block_on(delete_password(id))
    }
}
```

---

## 6. Unit Testing Strategy

The test suite must cover unit testing across `models`, `storage`, and `secrets`.

### Test Coverage Plan
1. **Model Defaults & Serde Attributes (`tests/e2e_data_tests.rs` or `src/models.rs` unit tests):**
   - Test default instantiation (`Connection::default()`).
   - Test parsing sparse JSON `{}` -> verify default values generated for `id`, `name`, `protocol`, `port`, `group`, and `advanced_settings`.
   - Test parsing partial `advanced_settings` object `{"vnc_viewonly": true}` -> verify unspecified flags match default `false` / `OriginalSize`.
   - Test enum variants roundtrip (`Protocol::Rdp` <-> `"rdp"`, `VncScaling::FitToWindow` <-> `"Fit to Window"`).
2. **Storage Engine & 4-Space Indentation (`src/storage.rs` unit tests):**
   - Test roundtrip load/save with `tempfile::tempdir()`.
   - Test 4-space indent verification: verify saved JSON string contains `"    "` indentation.
   - Test auto-creation of missing parent directories during save.
   - Test non-existent file load returns empty vector `vec![]` without throwing error.
   - Test loading actual existing user `connections.json` sample.
3. **Secrets Management (`src/secrets.rs` unit tests):**
   - Test secret keyring operations and fallback handling when Secret Service daemon is active or unavailable.

---

## 7. Implementation Checklist for Implementer

- [ ] Update `Cargo.toml` with dependencies (`gtk4`, `libadwaita`, `serde`, `serde_json`, `vnc`, `oo7`, `tokio`, `anyhow`, `uuid`, `dirs`, `tempfile`).
- [ ] Create `src/models.rs` with `Connection`, `AdvancedSettings`, `AppConfig`, `Protocol`, `VncScaling` and `#[serde(...)]` attributes.
- [ ] Create `src/storage.rs` with `load_connections`, `save_connections`, `load_config`, `save_config` and 4-space JSON formatting helper.
- [ ] Create `src/secrets.rs` with `get_password`, `set_password`, `delete_password` using `oo7` keyring client under service `"ver_remote_connection_manager"`.
- [ ] Implement `tests/e2e_data_tests.rs` verifying models, storage roundtrips, and missing field fallback handling.
- [ ] Execute `cargo test` to verify 100% test pass rate.
