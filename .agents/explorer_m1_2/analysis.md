# Analysis Report: Edge Cases, Default Fallbacks & Backward Compatibility (Milestone 1)

**Agent:** explorer_m1_2  
**Directory:** `/home/dawiisss/Documents/antigravity/beautiful-goodall/.agents/explorer_m1_2`  
**Date:** 2026-08-12  

---

## 1. Executive Summary

Milestone 1 requires implementing the Serde data models (`Connection`, `AdvancedSettings`, `AppConfig`, `Protocol`, `VncScaling`, `Theme`) and the JSON storage engine for `~/.config/ver/connections.json` and `~/.config/ver/config.json`.

This analysis investigates real-world edge cases, missing and legacy fields, corrupt JSON scenarios, strict field validation rules, and backward compatibility with existing Python connections. Based on empirical analysis of `~/.config/ver/connections.json` (7 active user connections) and Python models in `src/models.py`, `src/core/storage.py`, `src/core/config.py`, and `src/ui/editor.py`, this report specifies default functions, validation rules, error recovery, and copy-pasteable Rust implementations.

---

## 2. Missing, Corrupted & Legacy JSON Field Inventory

Inspection of `~/.config/ver/connections.json` and legacy Python deserialization (`Connection.from_dict`) reveals 4 major categories of field variations in real user data:

### 2.1 Real-World Observed Field Patterns
1. **Fully Populated VNC Entry** (e.g. Entry 2):
   - Contains all root fields (`id`, `name`, `protocol`, `host`, `port`, `username`, `mac_address`, `group`) and all `advanced_settings` fields (`rdp_multimon`, `rdp_fullscreen`, `rdp_audio`, `vnc_viewonly`, `vnc_shared`, `clipboard_sharing`, `color_depth`, `vnc_scaling`).
2. **Partial Advanced Settings RDP Entry** (e.g. Entries 1, 3, 5, 6, 7):
   - `advanced_settings` contains only `{"rdp_multimon": false, "rdp_fullscreen": false, "rdp_audio": false, "vnc_viewonly": false, "vnc_shared": false}`.
   - Missing fields: `clipboard_sharing`, `color_depth`, `vnc_scaling`.
3. **Empty Advanced Settings Entry** (e.g. Entry 4):
   - `"advanced_settings": {}` — dictionary exists but contains zero key-value pairs.
4. **Unset Host / Missing Credentials**:
   - `host` is `""` or `username` is `""` or `mac_address` is `""`.

### 2.2 Theoretical & Legacy Migration Edge Cases
- **Missing Root Fields**: Very old files or manually created entries may omit `group`, `mac_address`, `username`, `port`, `protocol`, `name`, or `id`.
- **Missing `advanced_settings` Key**: Whole `advanced_settings` field absent at the root level.
- **Port Specified as 0 or Null**: Port is set to 0 or missing from JSON.
- **Invalid Protocol Case/String**: e.g., `"RDP"`, `"VNC"`, or unknown string like `"http"`.
- **Invalid Color Depth**: Integer outside of allowed set `{0, 8, 16, 24, 32}`.
- **Invalid Scaling Mode String**: Unknown scaling mode string like `"Fit"`.

---

## 3. Matrix of Default Value Functions

To ensure perfect Serde deserialization of any existing or incomplete JSON, every field in `Connection`, `AdvancedSettings`, and `AppConfig` must use `#[serde(default = "default_fn")]` or `#[serde(default)]`.

### 3.1 Field-Level Defaults Table

| Struct | Field | Type | Default Value | Default Function | Rationale / Behavior |
|--------|-------|------|---------------|------------------|----------------------|
| `Connection` | `id` | `String` | UUID v4 | `default_id()` | Generates fresh `Uuid::new_v4().to_string()` if missing |
| `Connection` | `name` | `String` | `"New Connection"` | `default_name()` | Default connection name matching Python fallback |
| `Connection` | `protocol` | `Protocol` | `Protocol::Rdp` | `default_protocol()` | Primary default protocol |
| `Connection` | `host` | `String` | `""` | `default_host()` | Empty host string for draft entries |
| `Connection` | `port` | `u16` | `3389` | `default_port()` | Default RDP port (overridden by protocol resolution if 0) |
| `Connection` | `username` | `String` | `""` | `default_username()` | Empty username |
| `Connection` | `mac_address` | `String` | `""` | `default_mac_address()` | Empty MAC address string |
| `Connection` | `group` | `String` | `"Default"` | `default_group()` | Primary group name in sidebar |
| `Connection` | `advanced_settings` | `AdvancedSettings` | `AdvancedSettings::default()` | `default_advanced_settings()` | Default sub-struct instance |
| `AdvancedSettings` | `rdp_multimon` | `bool` | `false` | `Default::default` | Single monitor by default |
| `AdvancedSettings` | `rdp_fullscreen` | `bool` | `false` | `Default::default` | Windowed by default |
| `AdvancedSettings` | `rdp_audio` | `bool` | `false` | `Default::default` | Audio redirection off |
| `AdvancedSettings` | `vnc_viewonly` | `bool` | `false` | `Default::default` | Interactive mode on |
| `AdvancedSettings` | `vnc_shared` | `bool` | `false` | `Default::default` | Exclusive session by default |
| `AdvancedSettings` | `clipboard_sharing` | `bool` | `false` | `Default::default` | Clipboard disabled |
| `AdvancedSettings` | `color_depth` | `u8` | `0` | `default_color_depth()` | `0` represents "Default (Auto)" depth |
| `AdvancedSettings` | `vnc_scaling` | `VncScaling` | `VncScaling::OriginalSize` | `default_vnc_scaling()` | `"Original Size"` rendering |
| `AppConfig` | `theme` | `Theme` | `Theme::System` | `default_theme()` | `"system"` dark/light theme preference |

### 3.2 Protocol-Aware Port Resolution

When `port` is omitted or set to `0`, `Connection::resolve_port()` dynamically determines the port based on the active protocol:

```rust
impl Connection {
    pub fn resolve_port(&self) -> u16 {
        if self.port != 0 {
            return self.port;
        }
        match self.protocol {
            Protocol::Rdp => 3389,
            Protocol::Vnc => 5900,
            Protocol::Ssh => 22,
        }
    }
}
```

---

## 4. Strict Validation & Sanitization Rules

Data loaded from disk or input by users must pass validation and auto-sanitization before use:

### 4.1 Field Validation Requirements
1. **Connection ID (`id`)**:
   - Must be a valid v4 UUID (36 chars: `8-4-4-4-12` hex).
   - If invalid/malformed, replace with `Uuid::new_v4().to_string()` and log fixup.
2. **Connection Name (`name`)**:
   - Must not be empty after trimming whitespace (`!name.trim().is_empty()`).
   - If empty, reset to `"New Connection"`.
3. **Connection Group (`group`)**:
   - If empty after trimming whitespace (`group.trim().is_empty()`), reset to `"Default"`.
4. **Port Number (`port`)**:
   - Valid range: `1..=65535`.
   - If `port == 0`, invoke `resolve_port()` to assign 3389/5900/22.
5. **Color Depth (`color_depth`)**:
   - Allowed values: `0` (Auto), `8` (256 colors), `16` (High color), `24` (True color), `32` (Highest quality).
   - Any invalid depth integer (e.g. `12`, `64`, `255`) is sanitized to `0` (Auto).
6. **MAC Address (`mac_address`)**:
   - Used by `network::send_wol(mac)`.
   - Validation function `validate_mac()` strips whitespace, colons (`:`), and hyphens (`-`), checking that exactly 12 hexadecimal digits remain.
   - If valid, returns formatted upper-case hex string (e.g., `001122334455`); if invalid non-empty string, returns `Err(ValidationError::InvalidMacAddress)`.

---

## 5. Corrupt / Missing JSON Storage Recovery Strategy

The storage layer (`storage.rs`) must handle file system and format anomalies without crashing:

```
                  ┌───────────────────────────────┐
                  │ Read ~/.config/ver/connections│
                  └───────────────┬───────────────┘
                                  │
                 ┌────────────────┴────────────────┐
                 │ File exists & valid JSON array? │
                 └───────┬─────────────────┬───────┘
                     YES │                 │ NO
                         ▼                 ▼
          ┌────────────────────┐   ┌───────────────────────────────┐
          │ Deserialize Items  │   │ Check Failure Reason          │
          └──────────┬─────────┘   └───────────────┬───────────────┘
                     │                             │
                     ▼                             ├─ NotFound/Empty ──► Return vec![]
          ┌────────────────────┐                   │
          │ Run sanitize() on  │                   └─ Corrupt Syntax ──► 1. Backup file to
          │ each Connection    │                                          .corrupt.<timestamp>
          └──────────┬─────────┘                                       2. Log error
                     │                                                 3. Return vec![]
                     ▼
          ┌────────────────────┐
          │ Return Valid List  │
          └────────────────────┘
```

### 5.1 Recovery Procedures
1. **File Not Found (`std::io::ErrorKind::NotFound`)**:
   - Create parent directory `~/.config/ver` if missing.
   - Return empty `Vec<Connection>` or default `AppConfig`.
2. **Corrupted / Invalid JSON Syntax**:
   - Create a backup copy of the corrupted file: `connections.json.corrupt.<timestamp>`.
   - Log error output.
   - Return empty `Vec<Connection>` to allow application startup without panicking.
3. **Item-Level Resilience (Tolerant Array Deserialization)**:
   - When loading connections, parse root array as `Vec<serde_json::Value>`.
   - Attempt deserializing each element into `Connection`. If a single element fails due to structural corruption (e.g., `"port": "invalid_string"`), skip or sanitize that single element while preserving the remaining valid connections.
4. **Atomic Write Strategy**:
   - Write output to temporary file `connections.json.tmp`.
   - Perform `file.sync_all()` to ensure data hits physical storage.
   - Atomically replace target file using `std::fs::rename`.
5. **4-Space Formatting Compliance**:
   - Format output JSON using 4-space indentation to maintain 100% diff parity with Python `json.dump(..., indent=4)`.

---

## 6. Complete Production Rust Data Models & Storage Specification

### 6.1 `src/models.rs`

```rust
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Protocol type supported by VER
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

/// VNC Scaling modes for embedded viewer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

/// Advanced settings per connection
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AdvancedSettings {
    pub rdp_multimon: bool,
    pub rdp_fullscreen: bool,
    pub rdp_audio: bool,
    pub vnc_viewonly: bool,
    pub vnc_shared: bool,
    pub clipboard_sharing: bool,
    pub color_depth: u8,
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
            color_depth: 0, // 0 = Auto
            vnc_scaling: VncScaling::OriginalSize,
        }
    }
}

impl AdvancedSettings {
    pub fn sanitize(&mut self) -> bool {
        let mut modified = false;
        if !matches!(self.color_depth, 0 | 8 | 16 | 24 | 32) {
            self.color_depth = 0;
            modified = true;
        }
        modified
    }
}

/// Primary Connection data model
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Connection {
    #[serde(default = "default_id")]
    pub id: String,

    #[serde(default = "default_name")]
    pub name: String,

    #[serde(default = "default_protocol")]
    pub protocol: Protocol,

    #[serde(default = "default_host")]
    pub host: String,

    #[serde(default = "default_port")]
    pub port: u16,

    #[serde(default = "default_username")]
    pub username: String,

    #[serde(default = "default_mac_address")]
    pub mac_address: String,

    #[serde(default = "default_group")]
    pub group: String,

    #[serde(default = "default_advanced_settings")]
    pub advanced_settings: AdvancedSettings,
}

// Serde Default Helper Functions
fn default_id() -> String {
    Uuid::new_v4().to_string()
}
fn default_name() -> String {
    "New Connection".to_string()
}
fn default_protocol() -> Protocol {
    Protocol::Rdp
}
fn default_host() -> String {
    String::new()
}
fn default_port() -> u16 {
    3389
}
fn default_username() -> String {
    String::new()
}
fn default_mac_address() -> String {
    String::new()
}
fn default_group() -> String {
    "Default".to_string()
}
fn default_advanced_settings() -> AdvancedSettings {
    AdvancedSettings::default()
}

impl Default for Connection {
    fn default() -> Self {
        Self {
            id: default_id(),
            name: default_name(),
            protocol: default_protocol(),
            host: default_host(),
            port: default_port(),
            username: default_username(),
            mac_address: default_mac_address(),
            group: default_group(),
            advanced_settings: default_advanced_settings(),
        }
    }
}

impl Connection {
    /// Resolve port defaults when port is unset (0)
    pub fn resolve_port(&self) -> u16 {
        if self.port != 0 {
            return self.port;
        }
        match self.protocol {
            Protocol::Rdp => 3389,
            Protocol::Vnc => 5900,
            Protocol::Ssh => 22,
        }
    }

    /// Sanitize fields, ensuring non-empty names/groups, valid UUIDs, and clean settings
    pub fn sanitize(&mut self) -> bool {
        let mut modified = false;

        if Uuid::parse_str(&self.id).is_err() {
            self.id = Uuid::new_v4().to_string();
            modified = true;
        }

        if self.name.trim().is_empty() {
            self.name = "New Connection".to_string();
            modified = true;
        }

        if self.group.trim().is_empty() {
            self.group = "Default".to_string();
            modified = true;
        }

        if self.port == 0 {
            self.port = self.resolve_port();
            modified = true;
        }

        if self.advanced_settings.sanitize() {
            modified = true;
        }

        modified
    }

    /// Validate MAC address format for Wake-on-LAN
    pub fn validate_mac(&self) -> Result<Option<String>, String> {
        let trimmed = self.mac_address.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }
        let clean: String = trimmed.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        if clean.len() == 12 {
            Ok(Some(clean.to_uppercase()))
        } else {
            Err(format!("Invalid MAC address format: '{}'", self.mac_address))
        }
    }
}

/// Global Application Configuration Model
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    System,
    Light,
    Dark,
}

impl Default for Theme {
    fn default() -> Self {
        Theme::System
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_theme")]
    pub theme: Theme,
}

fn default_theme() -> Theme {
    Theme::System
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

## 7. Verification Strategy & Test Matrix

The following test scenarios must be implemented in unit/integration tests (`tests/e2e_data_tests.rs`) to verify edge case handling:

| Test Case ID | Description | Input JSON / State | Expected Result |
|--------------|-------------|--------------------|-----------------|
| `TC-DATA-001` | Existing Real File Parsing | Deserialization of active `~/.config/ver/connections.json` | Parses all 7 connections without error |
| `TC-DATA-002` | Missing Advanced Settings Keys | Entry with `{}` as `advanced_settings` | Defaults populated (`color_depth=0`, `vnc_scaling=OriginalSize`, etc.) |
| `TC-DATA-003` | Omitted Root Fields | Connection JSON lacking `id`, `group`, `mac_address` | Auto-generates UUID, defaults group to `"Default"`, MAC to `""` |
| `TC-DATA-004` | Invalid UUID Recovery | Connection with `"id": "not-a-uuid"` | `sanitize()` replaces ID with new valid UUID v4 |
| `TC-DATA-005` | Blank Name & Group Fixup | Connection with `"name": "   "`, `"group": ""` | `sanitize()` updates name to `"New Connection"` and group to `"Default"` |
| `TC-DATA-006` | Invalid Color Depth Cleanup | `"color_depth": 99` | `sanitize()` resets depth to `0` (Auto) |
| `TC-DATA-007` | Zero Port Resolution | Protocol `vnc` with `"port": 0` | `resolve_port()` returns `5900` |
| `TC-DATA-008` | Corrupt Syntax Handling | Syntactically invalid JSON string | Backup created with `.corrupt` suffix, returns empty `vec![]` |
| `TC-DATA-009` | 4-Space Save Formatting | `save_connections(&conns)` | Produced JSON contains 4-space indentation matching Python |
