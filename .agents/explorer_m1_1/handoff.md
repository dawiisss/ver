# Handoff Report: Milestone 1 (R1: Rust Skeleton & Serde Data Models) Implementation Specifications

**Agent:** explorer_m1_1  
**Directory:** `/home/dawiisss/Documents/antigravity/beautiful-goodall/.agents/explorer_m1_1`  
**Date:** 2026-08-12  

---

## 1. Observation

Directly observed files, line references, and project artifacts:

- **Original Project Request (`.agents/ORIGINAL_REQUEST.md:12-14`):**
  > "R1. Rust Project Skeleton & Data Models: The existing Python source files should be replaced with a Cargo project. The connection data model must parse and serialize the existing connections JSON file perfectly using `serde`."
- **Orchestrator Project Plan (`.agents/orchestrator/PROJECT.md:7-9`):**
  > "`models`: `Connection` struct, `AdvancedSettings` struct, `AppConfig` struct, `Protocol` enum, `VncScaling` enum with Serde JSON annotations. `storage`: Load/save `~/.config/ver/connections.json` and `~/.config/ver/config.json` with 4-space indentation. `secrets`: Password storage/retrieval via Secret Service (`oo7` crate) using service `ver_remote_connection_manager`."
- **Survey Analysis (`.agents/explorer_survey_2/handoff.md:14-46`):**
  Python connections are stored at `~/.config/ver/connections.json` with 4-space indentation. `advanced_settings` fields are frequently omitted or partial in existing JSON entries. Passwords are saved separately in system keyring under service `"ver_remote_connection_manager"`.
- **Existing Crate Configuration (`Cargo.toml:1-15`):**
  Current `Cargo.toml` specifies `gtk`, `libadwaita`, `serde`, `serde_json`, `vnc`, `oo7`, `tokio`, `anyhow`, but lacks `uuid` (required for UUID v4 ID generation) and `dirs` (for path resolution), and lacks `tempfile` under `[dev-dependencies]`.

---

## 2. Logic Chain

1. **Cargo Configuration:** `Cargo.toml` must include `uuid` with features `["v4", "serde"]` to allow `Uuid::new_v4()` generation of default connection IDs and Serde integration. `dirs` is required for resolving `~/.config/ver/` across environments, and `tempfile = "3.8"` is needed for unit testing storage persistence without modifying `~/.config/ver/`.
2. **Serde Data Models (`src/models.rs`):**
   - Existing JSON connections contain missing or partial fields. Placing `#[serde(default)]` on every field of `AdvancedSettings` and `Connection` guarantees 100% deserialization resilience.
   - Enums `Protocol` (`rename_all = "lowercase"`) and `VncScaling` (`rename = "Original Size"`, `"Fit to Window"`, `"Stretch"`) ensure perfect mapping to JSON string values.
   - Passwords must be excluded from `Connection` serialization to prevent plaintext exposure on disk.
3. **Storage Engine (`src/storage.rs`):**
   - Default `serde_json::to_string_pretty` uses 2-space indentation. To maintain exact compatibility with Python's 4-space indent (`json.dump(..., indent=4)`), `src/storage.rs` uses `serde_json::ser::PrettyFormatter::with_indent(b"    ")`.
   - `save_connections` and `save_config` invoke `fs::create_dir_all` to automatically create parent directories.
   - `load_connections` and `load_config` handle missing files gracefully by returning `Ok(Vec::new())` or `Ok(AppConfig::default())`.
4. **Secret Keyring (`src/secrets.rs`):**
   - `oo7::Keyring` handles Secret Service DBus requests under service name `"ver_remote_connection_manager"`.
   - `get_password` searches by `connection_id`, falling back to `"username"` attribute to preserve compatibility with existing credentials saved by Python `keyring`.
   - Synchronous wrappers (`get_password_sync`, `set_password_sync`, `delete_password_sync`) allow calling secrets logic from sync or async GTK contexts.
5. **Unit Test Strategy:**
   - Unit tests verify roundtrip serialization, 4-space formatting indentation, missing key default fallbacks, and storage file operations using `tempfile::tempdir()`.

---

## 3. Caveats

No caveats. All target APIs, Serde annotations, storage paths, 4-space indentation logic, keyring attributes, and unit test strategies have been fully analyzed and specified.

---

## 4. Conclusion

Milestone 1 implementation design is complete. The exact code specifications for `Cargo.toml`, `src/models.rs`, `src/storage.rs`, `src/secrets.rs`, and unit test strategies are recorded in `.agents/explorer_m1_1/analysis.md`. The implementer can directly copy and integrate these specifications into the Cargo codebase.

---

## 5. Verification Method

To verify these specifications independently:

1. Inspect `.agents/explorer_m1_1/analysis.md` for complete, copy-pasteable code implementations.
2. Once implemented by implementer, run `cargo check` and `cargo test`.
3. Inspect generated JSON output in test suite to confirm 4-space indentation (`"    "`).
4. Verify deserialization of legacy `~/.config/ver/connections.json` sample entries using `cargo test`.
