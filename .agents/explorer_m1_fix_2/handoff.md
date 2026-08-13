# Test Suite Compilation & Contract Mismatch Investigation Report

**Author**: explorer_m1_fix_2  
**Target Scope**: Test Suite (`tests/`) vs Source Models & Storage (`src/models.rs`, `src/storage.rs`, `src/secrets.rs`, `src/ui/`, `src/vnc/`, `src/launcher.rs`, `src/network.rs`)  
**Status**: Completed Read-Only Analysis  

---

## 1. Observation

Direct empirical observations from inspecting all 7 test files in `tests/` against public exported contracts in `src/lib.rs` and underlying module implementations:

### A. Summary of Affected Files in `tests/`
Out of 7 integration test files, 6 contain API contract mismatches preventing successful compilation or execution:
1. `tests/e2e_data_tests.rs` (5 constructor mismatches, 5 storage path parameter mismatches, 4 async/sync keyring calls)
2. `tests/e2e_ui_tests.rs` (6 constructor mismatches)
3. `tests/e2e_vnc_tests.rs` (0 mismatches - fully compliant)
4. `tests/e2e_boundary_tests.rs` (3 async/sync keyring calls, 1 storage path parameter mismatch, 1 storage corrupt file return type assertion mismatch)
5. `tests/e2e_cross_feature_tests.rs` (5 constructor mismatches, 4 storage path parameter mismatches, 3 async/sync keyring calls)
6. `tests/e2e_launcher_tests.rs` (4 constructor mismatches)
7. `tests/e2e_lifecycle_tests.rs` (3 constructor mismatches, 5 storage path parameter mismatches, 4 async/sync keyring calls)

---

### B. Detailed Contract Mismatches by Category

#### 1. Constructor Signature Mismatches (`Connection::new` vs `Connection::default`)
- **`src/models.rs` Definition**:
  ```rust
  impl Default for Connection {
      fn default() -> Self { ... }
  }
  impl Connection {
      pub fn new_with_protocol(protocol: Protocol) -> Self { ... }
  }
  ```
  `Connection::new()` does NOT exist on `Connection`.
- **Test File Violations**:
  - `tests/e2e_data_tests.rs`: lines 19, 20, 28, 79, 83 call `Connection::new()`
  - `tests/e2e_ui_tests.rs`: lines 6, 11, 31, 35, 39, 53 call `Connection::new()`
  - `tests/e2e_cross_feature_tests.rs`: lines 10, 14, 18, 22, 81 call `Connection::new()`
  - `tests/e2e_launcher_tests.rs`: lines 7, 33, 48, 64 call `Connection::new()`
  - `tests/e2e_lifecycle_tests.rs`: lines 20, 76, 104 call `Connection::new()`

#### 2. Storage Function Parameter & Name Mismatches
- **`src/storage.rs` Definitions**:
  - `save_connections(connections: &[Connection]) -> Result<()>` (1 param: uses default path `~/.config/ver/connections.json`)
  - `save_connections_to_path(path: &Path, connections: &[Connection]) -> Result<()>` (2 params: explicit path)
  - `load_connections() -> Result<Vec<Connection>>` (0 params: uses default path `~/.config/ver/connections.json`)
  - `load_connections_from_path(path: &Path) -> Result<Vec<Connection>>` (1 param: explicit path)
  - `save_config(config: &AppConfig) -> Result<()>` (1 param)
  - `save_config_to_path(path: &Path, config: &AppConfig) -> Result<()>` (2 params)
  - `load_config() -> Result<AppConfig>` (0 params)
  - `load_config_from_path(path: &Path) -> Result<AppConfig>` (1 param)
- **Test File Violations**:
  - `tests/e2e_data_tests.rs`:
    - Line 89: `storage::save_connections(&file_path, &connections)` (passed 2 args to 1-arg function)
    - Line 95 & 106: `storage::load_connections(&file_path)` (passed 1 arg to 0-arg function)
    - Line 118: `storage::save_config(&config_path, &config)` (passed 2 args to 1-arg function)
    - Line 119: `storage::load_config(&config_path)` (passed 1 arg to 0-arg function)
  - `tests/e2e_boundary_tests.rs`:
    - Line 85: `storage::load_connections(&corrupt_file)` (passed 1 arg to 0-arg function)
  - `tests/e2e_cross_feature_tests.rs`:
    - Line 51: `storage::save_config(&config_path, &window.config)`
    - Line 53: `storage::load_config(&config_path)`
    - Line 87 & 97: `storage::save_connections(&connections_file, ...)`
    - Line 100: `storage::load_connections(&connections_file)`
  - `tests/e2e_lifecycle_tests.rs`:
    - Line 16, 45, 69: `storage::load_connections(&connections_file)`
    - Line 37, 65: `storage::save_connections(&connections_file, ...)`

#### 3. Keyring Asynchronous vs Synchronous Call Mismatches
- **`src/secrets.rs` Definitions**:
  - `pub async fn get_password(id: &str) -> Result<Option<String>>`
  - `pub async fn set_password(id: &str, password: &str) -> Result<()>`
  - `pub async fn delete_password(id: &str) -> Result<()>`
  - `pub fn get_password_sync(id: &str) -> Result<Option<String>>`
  - `pub fn set_password_sync(id: &str, password: &str) -> Result<()>`
  - `pub fn delete_password_sync(id: &str) -> Result<()>`
- **Test File Violations**:
  Standard synchronous `#[test]` functions call the async versions `secrets::set_password`, `secrets::get_password`, `secrets::delete_password` directly expecting a `Result` type return value without calling `.await` or using synchronous helpers:
  - `tests/e2e_data_tests.rs`: lines 129, 132, 136, 137
  - `tests/e2e_boundary_tests.rs`: lines 65, 74, 75
  - `tests/e2e_cross_feature_tests.rs`: lines 86, 98, 103
  - `tests/e2e_lifecycle_tests.rs`: lines 38, 49, 66, 71

#### 4. Behavioral Return Type & Assertion Mismatch
- **`src/storage.rs` Corrupt File Handling Behavior**:
  `load_connections_from_path` catches JSON parse errors, backs up the file as `.corrupt.<timestamp>`, and returns `Ok(Vec::new())` (empty vector).
- **`tests/e2e_boundary_tests.rs` Violation**:
  Lines 85-86 call `storage::load_connections(&corrupt_file)` and assert `assert!(result.is_err())`. When updated to `load_connections_from_path`, this assertion would fail at runtime because the actual behavior returns `Ok(Vec::new())`.

---

## 2. Logic Chain

1. **Observation 1**: `src/models.rs` implements `Default` for `Connection` and `Connection::new_with_protocol(protocol: Protocol)`, but does not expose `Connection::new()`.
2. **Logic Step 1**: Every test invoking `Connection::new()` will fail compilation with `error[E0599]: no function or associated item named 'new' found for struct 'Connection'`. Replacing these calls with `Connection::default()` resolves the constructor compilation errors while maintaining default initialization behavior.

3. **Observation 2**: `src/storage.rs` distinguishes between default path operations (`load_connections()`, `save_connections(&conns)`) and explicit path operations (`load_connections_from_path(path)`, `save_connections_to_path(path, &conns)`).
4. **Logic Step 2**: All tests using `tempdir()` pass custom file paths to `storage` functions. Calling `save_connections(&file_path, &conns)` passes 2 arguments to a 1-argument function, and calling `load_connections(&file_path)` passes 1 argument to a 0-argument function. Updating tests to call `_from_path` and `_to_path` variants matches the public API contract in `src/storage.rs`.

5. **Observation 3**: `src/secrets.rs` provides async functions (`get_password`, `set_password`, `delete_password`) and sync wrapper functions (`get_password_sync`, `set_password_sync`, `delete_password_sync`).
6. **Logic Step 3**: Test cases annotated with standard `#[test]` are synchronous functions. Calling async functions without `async` / `.await` returns a `Future` object which does not implement `.expect(...)` or `.unwrap()`. Updating standard `#[test]` calls to use `secrets::set_password_sync`, `secrets::get_password_sync`, and `secrets::delete_password_sync` aligns test execution with synchronous runtime semantics.

7. **Observation 4**: `tests/e2e_boundary_tests.rs:85-86` asserts `assert!(result.is_err())` for corrupted JSON.
8. **Logic Step 4**: `storage::load_connections_from_path` explicitly implements graceful recovery on corrupt files by creating a timestamped backup and returning `Ok(Vec::new())`. The test assertion must be updated to `let loaded = storage::load_connections_from_path(&corrupt_file).expect("Should recover gracefully"); assert!(loaded.is_empty());` to align with product specifications.

---

## 3. Caveats

- Investigation was strictly read-only per constraints. No files in `src/` or `tests/` were modified.
- Keyring operations (`oo7`) in tests execute against D-Bus / Secret Service when available, or fall back gracefully if unavailable as designed in `src/secrets.rs`.

---

## 4. Conclusion & Concrete Fix Recommendations

To restore 100% test suite compilation and contract alignment across all test files in `tests/`, implement the following concrete changes:

### File-by-File Fix Blueprint

#### 1. `tests/e2e_data_tests.rs`
- **Lines 19, 20, 28, 79, 83**: Replace `Connection::new()` with `Connection::default()`.
- **Line 89**: Replace `storage::save_connections(&file_path, &connections)` with `storage::save_connections_to_path(&file_path, &connections)`.
- **Line 95**: Replace `storage::load_connections(&file_path)` with `storage::load_connections_from_path(&file_path)`.
- **Line 106**: Replace `storage::load_connections(&file_path)` with `storage::load_connections_from_path(&file_path)`.
- **Line 118**: Replace `storage::save_config(&config_path, &config)` with `storage::save_config_to_path(&config_path, &config)`.
- **Line 119**: Replace `storage::load_config(&config_path)` with `storage::load_config_from_path(&config_path)`.
- **Line 129**: Replace `secrets::set_password(conn_id, password)` with `secrets::set_password_sync(conn_id, password)`.
- **Line 132**: Replace `secrets::get_password(conn_id)` with `secrets::get_password_sync(conn_id)`.
- **Line 136**: Replace `secrets::delete_password(conn_id)` with `secrets::delete_password_sync(conn_id)`.
- **Line 137**: Replace `secrets::get_password(conn_id)` with `secrets::get_password_sync(conn_id)`.

#### 2. `tests/e2e_ui_tests.rs`
- **Lines 6, 11, 31, 35, 39, 53**: Replace `Connection::new()` with `Connection::default()`.

#### 3. `tests/e2e_vnc_tests.rs`
- **No changes required** (0 errors).

#### 4. `tests/e2e_boundary_tests.rs`
- **Line 65**: Replace `secrets::get_password(...)` with `secrets::get_password_sync(...)`.
- **Line 74**: Replace `secrets::set_password(...)` with `secrets::set_password_sync(...)`.
- **Line 75**: Replace `secrets::get_password(...)` with `secrets::get_password_sync(...)`.
- **Lines 85-86**: Replace:
  ```rust
  let result = storage::load_connections(&corrupt_file);
  assert!(result.is_err());
  ```
  with:
  ```rust
  let loaded = storage::load_connections_from_path(&corrupt_file).expect("Should recover gracefully");
  assert!(loaded.is_empty());
  ```

#### 5. `tests/e2e_cross_feature_tests.rs`
- **Lines 10, 14, 18, 22, 81**: Replace `Connection::new()` with `Connection::default()`.
- **Line 51**: Replace `storage::save_config(&config_path, &window.config)` with `storage::save_config_to_path(&config_path, &window.config)`.
- **Line 53**: Replace `storage::load_config(&config_path)` with `storage::load_config_from_path(&config_path)`.
- **Line 86**: Replace `secrets::set_password(...)` with `secrets::set_password_sync(...)`.
- **Line 87**: Replace `storage::save_connections(&connections_file, ...)` with `storage::save_connections_to_path(&connections_file, ...)`.
- **Line 97**: Replace `storage::save_connections(&connections_file, ...)` with `storage::save_connections_to_path(&connections_file, ...)`.
- **Line 98**: Replace `secrets::set_password(...)` with `secrets::set_password_sync(...)`.
- **Line 100**: Replace `storage::load_connections(&connections_file)` with `storage::load_connections_from_path(&connections_file)`.
- **Line 103**: Replace `secrets::get_password(...)` with `secrets::get_password_sync(...)`.

#### 6. `tests/e2e_launcher_tests.rs`
- **Lines 7, 33, 48, 64**: Replace `Connection::new()` with `Connection::default()`.

#### 7. `tests/e2e_lifecycle_tests.rs`
- **Lines 20, 76, 104**: Replace `Connection::new()` with `Connection::default()`.
- **Lines 16, 45, 69**: Replace `storage::load_connections(&connections_file)` with `storage::load_connections_from_path(&connections_file)`.
- **Lines 37, 65**: Replace `storage::save_connections(&connections_file, ...)` with `storage::save_connections_to_path(&connections_file, ...)`.
- **Line 38**: Replace `secrets::set_password(...)` with `secrets::set_password_sync(...)`.
- **Line 49, 71**: Replace `secrets::get_password(...)` with `secrets::get_password_sync(...)`.
- **Line 66**: Replace `secrets::delete_password(...)` with `secrets::delete_password_sync(...)`.

---

## 5. Verification Method

To verify these findings and confirm the fix once applied:

1. Inspect each test file against `src/lib.rs` exports.
2. After applying the recommended fixes to `tests/`, run:
   ```bash
   cargo test --no-run
   ```
   **Expected Output**: 0 compilation errors across all unit and integration test targets.
3. Run test suite execution:
   ```bash
   cargo test
   ```
   **Expected Output**: All tests in `src/` and `tests/` pass cleanly.
