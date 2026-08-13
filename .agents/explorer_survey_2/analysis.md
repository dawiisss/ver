# Connection Data Models & JSON Storage Survey for Rust Serde Compatibility

**Author:** explorer_survey_2  
**Date:** 2026-08-12  
**Target Application:** VER - Very Easy Remote (beautiful-goodall)

---

## 1. Executive Summary

This document provides a complete analysis of all data models, JSON storage formats, field specifications, default values, optionality, protocol semantics, and secrets management in the VER (Very Easy Remote) connection manager application.

The goal is to ensure **100% loss-less reading, editing, and saving** of connection data and configuration files when replacing the Python application with a native Rust application using `serde` and `serde_json`.

### Key Storage Files

| File Target | Path | Format / Structure | Purpose |
|-------------|------|---------------------|---------|
| **Connections Store** | `~/.config/ver/connections.json` | JSON Array of Connection Objects (`[ { ... }, { ... } ]`) | Stores user-configured remote connection entries |
| **App Config Store** | `~/.config/ver/config.json` | JSON Object (`{ "theme": "..." }`) | Stores application-wide settings |
| **System Keyring** | System Keyring Service (`ver_remote_connection_manager`) | Secret Service / Keyring key-value pairs | Stores connection passwords (indexed by `connection_id`) |

---

## 2. Connection Data Model (`Connection`)

The primary data entity is `Connection`. In Python (`src/models.py`), it is defined as a dataclass with dynamic dictionary conversion (`from_dict` and `to_dict`).

### Field Specification Table

| Field Name | JSON Key | Python Type | Rust Serde Type | Optional / Default | Description & Constraints |
|------------|----------|-------------|-----------------|--------------------|---------------------------|
| `id` | `"id"` | `str` | `String` | Optional in JSON<br>Default: UUID v4 string | Unique identifier for connection (UUID v4 string, e.g. `"6be87110-0e27-4b85-b8b5-f4d3cba2f2aa"`). Used as keyring lookup key. |
| `name` | `"name"` | `str` | `String` | Optional in JSON<br>Default: `"New Connection"` | Display title of connection in UI. |
| `protocol` | `"protocol"` | `str` | `Protocol` / `String` | Optional in JSON<br>Default: `"rdp"` | Protocol identifier. Case-insensitive in logic. Valid values: `"rdp"`, `"vnc"`, `"ssh"`. |
| `host` | `"host"` | `str` | `String` | Optional in JSON<br>Default: `""` | IP address or domain name (e.g. `"192.168.50.70"`). |
| `port` | `"port"` | `int` | `u16` | Optional in JSON<br>Default: `3389` | Network port number. Protocol default fallbacks: RDP=3389, VNC=5900, SSH=22. |
| `username` | `"username"` | `str` | `String` | Optional in JSON<br>Default: `""` | Login username for remote host or SSH user. |
| `mac_address` | `"mac_address"` | `str` | `String` | Optional in JSON<br>Default: `""` | MAC address for Wake-on-LAN (e.g., `"00:11:22:33:44:55"`). |
| `group` | `"group"` | `str` | `String` | Optional in JSON<br>Default: `"Default"` | Grouping category used for sidebar list organization. |
| `advanced_settings` | `"advanced_settings"` | `dict` | `AdvancedSettings` | Optional in JSON<br>Default: `{}` | Nested object containing protocol-specific flags and parameters. |

---

## 3. Protocol Types & Defaults

| Protocol | Identification String | Default Port | Primary Handling in Rust | UI / Launcher Options |
|----------|-----------------------|--------------|--------------------------|-----------------------|
| **RDP** | `"rdp"` | `3389` | External binary (`xfreerdp3` / `xfreerdp`) | `/v:host:port /u:user +clipboard /cert:ignore /dynamic-resolution`, `/multimon`, `/f`, `/sound`, `/bpp:color_depth` |
| **VNC** | `"vnc"` | `5900` | Native embedded Rust client (`vnc-rs` rendering to `gtk4::Picture`/`DrawingArea`) | View-only, shared session, clipboard sharing, color depth, scaling ("Original Size", "Fit to Window", "Stretch") |
| **SSH** | `"ssh"` | `22` | External terminal launcher (`ptyxis`, `kgx`, `gnome-terminal`, etc.) executing `ssh [-p port] user@host` | Quick-connect URI parsing support (`ssh://user@host:port`) |

---

## 4. Advanced Settings Data Model (`AdvancedSettings`)

Nested within `Connection.advanced_settings`. Existing JSON files exhibit varying levels of completeness — older entries may omit newer keys or contain empty `{}`. All fields must be optional during deserialization.

### Field Specification Table

| Field Name | JSON Key | Type | Serde Rust Type | Default Value | Valid Values / Constraints |
|------------|----------|------|-----------------|---------------|----------------------------|
| `rdp_multimon` | `"rdp_multimon"` | `bool` | `bool` | `false` | Multi-monitor toggle for RDP |
| `rdp_fullscreen` | `"rdp_fullscreen"` | `bool` | `bool` | `false` | Fullscreen toggle for RDP |
| `rdp_audio` | `"rdp_audio"` | `bool` | `bool` | `false` | Audio redirection toggle for RDP |
| `vnc_viewonly` | `"vnc_viewonly"` | `bool` | `bool` | `false` | View-only mode for VNC |
| `vnc_shared` | `"vnc_shared"` | `bool` | `bool` | `false` | Shared session toggle for VNC |
| `clipboard_sharing` | `"clipboard_sharing"` | `bool` | `bool` | `false` | Clipboard sync toggle for RDP/VNC |
| `color_depth` | `"color_depth"` | `int` | `u32` | `0` | Color depth bpp: `0` (Auto/Default), `8`, `16`, `24`, `32` |
| `vnc_scaling` | `"vnc_scaling"` | `str` | `VncScaling` / `String` | `"Original Size"` | Scaling mode: `"Original Size"`, `"Fit to Window"`, `"Stretch"` |

---

## 5. Application Configuration (`AppConfig`)

File: `~/.config/ver/config.json`

### Field Specification Table

| Field Name | JSON Key | Python Type | Rust Serde Type | Default Value | Valid Values |
|------------|----------|-------------|-----------------|---------------|--------------|
| `theme` | `"theme"` | `str` | `String` | `"default"` | `"default"`, `"system"`, `"dark"`, `"light"` |

Sample `config.json` observed in practice:
```json
{
    "theme": "system"
}
```

---

## 6. Secrets & Keyring Management

**SECURITY REQUIREMENT:** Passwords MUST NOT be serialized into `connections.json` or `config.json`.

- **Storage Provider:** System Keyring (via Secret Service API / `keyring` crate in Rust).
- **Service Name:** `ver_remote_connection_manager`
- **Key (Account/User):** Connection `id` string (UUID v4, e.g. `"6be87110-0e27-4b85-b8b5-f4d3cba2f2aa"`).
- **Value:** Plaintext password string.
- **Operations:**
  - `save_password(id, password)`: Sets entry in keyring if password is non-empty.
  - `get_password(id)`: Fetches password from keyring, returning `""` on missing or error.
  - `delete_password(id)`: Removes password entry when connection is deleted.

---

## 7. JSON Storage Formatting Expectations

1. **Path Locations:**
   - Connections: `~/.config/ver/connections.json` (resolving `~` to `$HOME`).
   - Config: `~/.config/ver/config.json`.
   - Directory creation: Must create `~/.config/ver/` directory automatically if non-existent.
2. **Formatting:**
   - UTF-8 encoding.
   - 4-space indentation (`serde_json::to_string_pretty` or custom `Serializer` with 4-space indent).
3. **Migration & Missing Key Resilience:**
   - In Python, `from_dict` filters unknown keys and uses default keyword arguments for missing keys.
   - Serde must use `#[serde(default)]` on structs and fields to safely deserialize partial JSON inputs.
   - Serde default behavior (ignoring unrecognized extra fields) should be preserved to remain compatible with future/past schemas.

---

## 8. Recommended Rust Serde Implementation

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
    Stretch,
}

impl Default for VncScaling {
    fn default() -> Self {
        VncScaling::OriginalSize
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

fn default_group() -> String {
    "Default".to_string()
}

fn default_port() -> u16 {
    3389
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

## 9. Verification & Test Plan

1. **Parsing Real Data:**
   - Parse `~/.config/ver/connections.json` directly into `Vec<Connection>`.
   - Verify zero errors, all 7 objects deserialized correctly.
2. **Roundtrip Test:**
   - Serialize `Vec<Connection>` back to JSON with 4-space formatting.
   - Compare output keys and types against original python output.
3. **Missing Field Compatibility:**
   - Test deserializing `{ "name": "Minimal" }` -> verify default `id` generated, default port 3389, default empty strings, and default `advanced_settings`.
4. **Keyring Integration:**
   - Verify `id` string passed to keyring crate matches UUID string stored in `connections.json`.
