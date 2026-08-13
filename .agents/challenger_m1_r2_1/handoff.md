# Handoff Report — Empirical Stress Testing (Milestone 1)

**Verdict**: `REQUEST_CHANGES`

---

## 1. Observation

During empirical stress testing of Milestone 1 (`src/models.rs`, `src/storage.rs`, `src/secrets.rs`) and test execution via `cargo test --test m1_stress_harness`, three distinct test cases failed reproducibly:

### Test Command Executed:
`cargo test --test m1_stress_harness`

### Observed Failures (Verbatim Output):

1. **Path Traversal / Non-UUID `Connection.id` Sanitization Failure**:
   ```
   ---- test_path_traversal_in_connection_ids stdout ----
   thread 'test_path_traversal_in_connection_ids' panicked at tests/m1_stress_harness.rs:97:9:
   Connection ID '../../../../etc/passwd' should have been sanitized to a valid UUID!
   ```

2. **Synchronous Keyring Wrapper Runtime Panic**:
   ```
   ---- test_sync_wrapper_in_current_thread_tokio_runtime_panic stdout ----
   thread 'test_sync_wrapper_in_current_thread_tokio_runtime_panic' panicked at src/secrets.rs:105:9:
   can call blocking only when running on the multi-threaded runtime
   ```

3. **Non-UTF-8 Binary Corruption Unhandled Error**:
   ```
   ---- test_non_utf8_file_resilience stdout ----
   thread 'test_non_utf8_file_resilience' panicked at tests/m1_stress_harness.rs:177:61:
   Should recover gracefully from non-UTF-8 files: Failed to read connections file at "/tmp/.tmpeNOOht/non_utf8.json"

   Caused by:
       stream did not contain valid UTF-8
   ```

---

## 2. Logic Chain

1. **Unsanitized `Connection.id` (`src/models.rs:197-200`)**:
   - *Observation*: `Connection::sanitize(&mut self)` checks `if self.id.trim().is_empty()`. It does NOT validate whether `self.id` is a valid UUID (e.g. `Uuid::parse_str(&self.id)`).
   - *Logic*: If `connections.json` contains malformed IDs, non-UUID strings, or path traversal vectors (e.g. `"../../../../etc/passwd"`), `sanitize()` returns `false` without regenerating a valid UUID. Downstream components that rely on `Connection.id` (such as keyring labels or future file paths) inherit unsafe strings.
   - *Conclusion*: `Connection::sanitize` must validate `Uuid::parse_str(&self.id)` and generate a new `Uuid::new_v4().to_string()` if parsing fails or if `self.id` is empty.

2. **Tokio Current-Thread Panic in Keyring Sync Wrappers (`src/secrets.rs:105, 118, 131`)**:
   - *Observation*: `get_password_sync`, `set_password_sync`, and `delete_password_sync` invoke `tokio::task::block_in_place(...)` whenever `tokio::runtime::Handle::try_current()` returns `Ok`.
   - *Logic*: `tokio::task::block_in_place` is only supported on Tokio multi-threaded runtimes (`RuntimeFlavor::MultiThread`). If `get_password_sync` is called from a GTK callback or single-threaded Tokio context (`RuntimeFlavor::CurrentThread`), Tokio panics unconditionally.
   - *Conclusion*: Keyring sync wrappers cannot rely on `block_in_place` when running inside single-threaded Tokio executors. They should fallback to `std::thread::spawn` + `block_on` or run the async task on a dedicated runtime thread when `block_in_place` is unavailable.

3. **Unhandled Non-UTF8 Error in `load_connections_from_path` & `load_config_from_path` (`src/storage.rs:64, 116`)**:
   - *Observation*: `load_connections_from_path` and `load_config_from_path` use `fs::read_to_string(path)?` directly.
   - *Logic*: While JSON syntax errors in `serde_json::from_str` are caught, backed up via `backup_corrupt_file`, and gracefully handled by returning empty/default structures, UTF-8 reading errors from `fs::read_to_string` propagate as an `Err` via `?`. Binary-corrupted or non-UTF-8 files crash application startup instead of triggering corrupt file backup and loading empty/default state.
   - *Conclusion*: File read / UTF-8 decoding errors must be handled as corrupt file events alongside JSON deserialization errors, invoking `backup_corrupt_file(path)` and returning default/empty state.

---

## 3. Caveats

- Keyring tests were run against Secret Service (`oo7` crate). Behavior when Secret Service D-Bus daemon is completely absent returns `Ok(None)` gracefully.
- GTK4 UI integration was tested via unit/stress harnesses; full GTK main loop thread interaction will be stress-tested in Milestone 2.
- No changes were made to implementation files (`src/`); test harness `tests/m1_stress_harness.rs` was updated to include empirical failure cases.

---

## 4. Conclusion

Milestone 1 implementation fails empirical stress testing due to 3 critical bugs in `src/models.rs`, `src/secrets.rs`, and `src/storage.rs`.

**Final Verdict**: `REQUEST_CHANGES`

---

## 5. Verification Method

To independently verify these findings:

```bash
cd /home/dawiisss/Documents/antigravity/beautiful-goodall
cargo test --test m1_stress_harness
```

Expected behavior after fixing implementation:
All 7 tests in `m1_stress_harness` must pass (`test result: ok. 7 passed; 0 failed`).

---

## Challenge Summary

**Overall risk assessment**: **HIGH**

## Challenges

### [High] Challenge 1: Unsantized Connection ID
- **Assumption challenged**: Assumed incoming or deserialized Connection IDs are valid UUIDs.
- **Attack scenario**: Malicious or corrupted `connections.json` containing path traversal strings (`../../../../etc/passwd`) or arbitrary text.
- **Blast radius**: Data corruption, invalid keyring queries, potential file path injection.
- **Mitigation**: Update `Connection::sanitize` to check `Uuid::parse_str(&self.id).is_err()` and replace with a fresh UUID.

### [High] Challenge 2: Keyring Sync Wrapper Runtime Panic
- **Assumption challenged**: Assumed `tokio::task::block_in_place` can be called from any active Tokio runtime handle.
- **Attack scenario**: Synchronous password lookup called from a GTK event handler or single-threaded Tokio executor (`flavor = "current_thread"`).
- **Blast radius**: Complete process crash (unhandled panic).
- **Mitigation**: Spawn a dedicated thread with a new Tokio runtime or safely detect runtime flavor before using `block_in_place`.

### [Medium] Challenge 3: Unhandled Non-UTF8 Storage File Corruption
- **Assumption challenged**: Assumed storage files on disk are always valid UTF-8 text files.
- **Attack scenario**: `connections.json` or `config.json` corrupted with binary data or non-UTF8 bytes.
- **Blast radius**: Unhandled `Err` returned from `load_connections` / `load_config`, failing application startup.
- **Mitigation**: Catch file read/UTF-8 errors in `storage.rs`, back up corrupt files using `backup_corrupt_file`, and return default structures.

## Stress Test Results

- `test_path_traversal_in_connection_ids` → Connection ID sanitized to valid UUID → Retained `"../../../../etc/passwd"` → **FAIL**
- `test_sync_wrapper_in_current_thread_tokio_runtime_panic` → Keyring sync lookup succeeds/fails gracefully → Panicked on `block_in_place` → **FAIL**
- `test_non_utf8_file_resilience` → Non-UTF8 file backed up & empty vec returned → Propagated read error & crashed → **FAIL**
- `test_malformed_json_strings_resilience` → Malformed JSON backed up & empty vec returned → Corrupt file backed up & empty vec returned → **PASS**
- `test_invalid_json_types_resilience` → Wrong JSON types backed up & empty vec returned → Corrupt file backed up & empty vec returned → **PASS**
- `test_large_json_input_stress` → 10,000 connections serialized & deserialized → 10,000 connections verified → **PASS**
- `test_special_characters_passwords_and_group_names` → Unicode/SQL/HTML strings roundtripped → Roundtrip verified → **PASS**

## Unchallenged Areas

- GTK4 UI event loops and widget rendering — out of scope for Milestone 1 (Milestone 2 scope).
- `vnc-rs` RFB network handshake — out of scope for Milestone 1 (Milestone 3 scope).
