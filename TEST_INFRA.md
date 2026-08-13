# E2E Test Infrastructure & Test Suite Specification

## Overview
This document specifies the requirement-driven opaque-box E2E test suite for the VER (Very Easy Remote) connection manager Rust application.

## Test Architecture & Isolation
- **Framework**: Standard Rust test harness (`cargo test`).
- **Isolation Strategy**: All storage and file-system tests utilize isolated temporary directories (`tempfile::tempdir()`). No tests modify production user directories (`~/.config/ver`). Keyring operations are isolated using unique UUID identifiers per test.
- **Independence**: Every test function sets up its own state, executes deterministically, and cleans up after itself.

## Test Suite Layout (`tests/`)

| File | Scope | Coverage / Tier |
|---|---|---|
| `tests/e2e_data_tests.rs` | Serde models, AppConfig defaults, Storage 4-space JSON pretty printing, Keyring operations, Protocol defaults | **Tier 1**: Feature Coverage (>=5 tests per feature) |
| `tests/e2e_boundary_tests.rs` | Empty/corrupt JSON files, missing fields, invalid MAC/IP, zero port, unknown protocol strings, Unicode & extreme ports | **Tier 2**: Boundary & Corner Cases |
| `tests/e2e_cross_feature_tests.rs` | Storage load/save roundtrip with keyring password retrieval, config file updates & theme persistence, UI editor mutations | **Tier 3**: Cross-Feature Combinations |
| `tests/e2e_lifecycle_tests.rs` | Python legacy connection format migration, multi-group persistence, full end-to-end RDP/VNC/SSH lifecycle flows | **Tier 4**: Real-World Workload Scenarios |
| `tests/e2e_launcher_tests.rs` | RDP argument construction (`xfreerdp3`), SSH command construction, Wake-on-LAN magic packet structure | Component Integration |
| `tests/e2e_ui_tests.rs` | MainWindow filtering, grouped connection lookup, ConnectionEditor dirty tracking, PreferencesWindow | Headless UI State |
| `tests/e2e_vnc_tests.rs` | VNC RGB to B8G8R8X8 framebuffer conversion, VncWidget rendering, Key and Pointer event propagation | VNC Client & Rendering |

## Tier Details

### Tier 1: Feature Coverage (>=5 tests per feature)
1. **Connection Model Serialization**: Roundtrip serialization, password isolation in JSON schema, default value injection on empty objects, ignoring unknown legacy fields, deserializing full advanced settings.
2. **AppConfig Defaults**: Default theme setting ("default"), Serde roundtrip, nonexistent config loading fallback, corrupt config recovery and backup generation, empty JSON object parsing.
3. **Storage Pretty Printing**: 4-space indent for connection arrays, 4-space indent for config objects, trailing newline enforcement, nested object indentation, auto-creation of parent directories.
4. **Keyring Operations Fallback**: Nonexistent item querying returns `None`, async set/get/delete lifecycle, special character & unicode password support, sync wrapper fallback, overwriting existing secrets.
5. **Protocol Defaults**: Default protocol enum (`Protocol::Rdp`), default port mapping (RDP=3389, VNC=5900, SSH=22), string representation (`as_str()`), Display trait implementation, port resolution when port is zero.

### Tier 2: Boundary & Corner Cases
- Empty JSON file parsing returning empty vector.
- Corrupt JSON syntax handling with backup creation.
- Missing optional JSON fields populated with default values.
- Invalid MAC address formats rejected by WoL validator.
- Port zero resolution via protocol defaults and `Connection::sanitize()`.
- Unknown protocol string rejection during deserialization.
- Extreme port boundaries (65535).
- Unicode connection names, groups, and username fields.

### Tier 3: Cross-Feature Combinations
- Storage JSON save/load combined with Keyring password storage and retrieval.
- Configuration file update, theme mutation via `PreferencesWindow`, and disk persistence.
- Combined UI editor mutation saving metadata to JSON storage and password to Keyring.
- VNC scaling mode switching during active rendering session.

### Tier 4: Real-World Workload Scenarios
- Migrating legacy Python `connections.json` files containing deprecated fields and missing properties into Rust models, then persisting back with 4-space indentation.
- Multi-group connection persistence with real-time grouping and search filtering in `MainWindow`.
- End-to-end full lifecycle tests for RDP, VNC embedded sessions, and SSH terminal session launchers.

## Running the Test Suite

Execute all tests:
```bash
cargo test
```

Execute a specific test file:
```bash
cargo test --test e2e_data_tests
cargo test --test e2e_boundary_tests
cargo test --test e2e_cross_feature_tests
cargo test --test e2e_lifecycle_tests
```
