# Handoff Report: Connection Data Models & JSON Storage Analysis

**Agent:** explorer_survey_2  
**Directory:** `/home/dawiisss/Documents/antigravity/beautiful-goodall/.agents/explorer_survey_2`  
**Date:** 2026-08-12  

---

## 1. Observation

Directly observed files and code references:

- **Dataclass Model (`src/models.py:1-32`):**
  ```python
  @dataclass
  class Connection:
      id: str = field(default_factory=lambda: str(uuid.uuid4()))
      name: str = "New Connection"
      protocol: str = "rdp" # rdp or vnc
      host: str = ""
      port: int = 3389
      username: str = ""
      mac_address: str = ""
      group: str = "Default"
      advanced_settings: dict = field(default_factory=dict)
  ```
- **Storage Logic (`src/core/storage.py:6-28`):**
  `CONNECTIONS_FILE = os.path.join(CONFIG_DIR, "connections.json")` (where `CONFIG_DIR = ~/.config/ver`).
  Loads list of connections with `json.load(f)` and `Connection.from_dict(c)`. Saves list with `json.dump(data, f, indent=4)`.
- **App Configuration (`src/core/config.py:5-29`):**
  `APP_CONFIG_FILE = os.path.join(CONFIG_DIR, "config.json")`. Default config `{"theme": "default"}`. Saves with `json.dump(config, f, indent=4)`.
- **Launcher Protocol Logic (`src/core/launcher.py:11-105`):**
  Protocols supported: `"rdp"`, `"vnc"`, `"ssh"`. Advanced settings options used:
  - `clipboard_sharing` (bool)
  - `color_depth` (int: 0, 8, 16, 24, 32)
  - `rdp_multimon` (bool)
  - `rdp_fullscreen` (bool)
  - `rdp_audio` (bool)
  - `vnc_viewonly` (bool)
  - `vnc_shared` (bool)
  - `vnc_scaling` (str: "Original Size", "Fit to Window", "Stretch")
- **Editor Form (`src/ui/editor.py:222-231`):**
  Constructs `advanced_settings` dictionary with keys: `rdp_multimon`, `rdp_fullscreen`, `rdp_audio`, `vnc_viewonly`, `vnc_shared`, `clipboard_sharing`, `color_depth`, `vnc_scaling`.
- **Keyring / Password Management (`src/core/secrets.py:3-22`):**
  Service name: `"ver_remote_connection_manager"`. Secret key: `connection.id`. Passwords are NEVER saved in JSON files.
- **Existing User Connections File (`~/.config/ver/connections.json`):**
  Contains 7 real entries, formatted with 4 spaces. Entries feature missing fields in `advanced_settings` (e.g. `{}` or missing `vnc_scaling`/`color_depth`), proving that missing fields must be optional with default fallback values in deserialization.

---

## 2. Logic Chain

1. **Storage Location & Format:**
   `src/core/storage.py` and `src/core/config.py` define target files in `~/.config/ver/` (`connections.json` and `config.json`). Files are formatted as pretty-printed JSON with 4-space indentation using `json.dump(..., indent=4)`.
2. **Schema Resilience:**
   `Connection.from_dict` in `src/models.py` filters unknown dictionary keys and uses Python dataclass default parameters. In addition, existing entries in `~/.config/ver/connections.json` demonstrate that `advanced_settings` can be `{}` or omit specific keys. Therefore, Rust `serde` structs must use `#[serde(default)]` on every field in `Connection` and `AdvancedSettings` to guarantee backward/forward compatibility.
3. **Protocol Mapping:**
   `src/core/launcher.py` and `src/ui/editor.py` establish three distinct protocol values: `"rdp"`, `"vnc"`, and `"ssh"`. A Serde enum `Protocol` with `#[serde(rename_all = "lowercase")]` maps directly to JSON string values.
4. **Secret Isolation:**
   `src/core/secrets.py` uses the system keyring under service name `"ver_remote_connection_manager"` with `connection_id` as the key. Passwords must remain strictly isolated from `Connection` serialization.

---

## 3. Caveats

- **No Caveats:** All Python source files, user config files, JSON data structures, and secrets handling code were fully inspected and verified.

---

## 4. Conclusion

The connection data model and JSON storage format have been completely mapped for Rust `serde` compatibility.
- Data structures: `Connection` struct, `AdvancedSettings` struct, `AppConfig` struct, `Protocol` enum, `VncScaling` enum.
- Storage rules: `connections.json` is a JSON array at `~/.config/ver/connections.json`; `config.json` is a JSON object at `~/.config/ver/config.json`.
- Formatting requirement: Indented with 4 spaces (`serde_json::to_string_pretty` / 4-space formatter).
- Password handling: Stored in system keyring (`keyring` crate in Rust), not in JSON.
- Serde design details and copy-pasteable Rust structs are recorded in `/home/dawiisss/Documents/antigravity/beautiful-goodall/.agents/explorer_survey_2/analysis.md`.

---

## 5. Verification Method

To verify these findings independently:

1. Inspect `analysis.md` in this directory: `/home/dawiisss/Documents/antigravity/beautiful-goodall/.agents/explorer_survey_2/analysis.md`.
2. Inspect Python model definitions: `src/models.py`, `src/core/storage.py`, `src/core/config.py`, `src/core/secrets.py`, `src/core/launcher.py`, `src/ui/editor.py`.
3. Read existing configuration: `cat ~/.config/ver/connections.json` and `cat ~/.config/ver/config.json`.
4. Validate Serde compatibility by compiling and testing the proposed Rust structs against existing `connections.json` using `serde_json::from_str`.
