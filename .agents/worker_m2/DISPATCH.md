## 2026-08-12T17:34:23Z
<USER_REQUEST>
You are worker_m2. Your working directory is /home/dawiisss/Documents/antigravity/beautiful-goodall/.agents/worker_m2.
Mandatory requirements:
1. Read /home/dawiisss/Documents/antigravity/beautiful-goodall/.agents/ORIGINAL_REQUEST.md
2. Read /home/dawiisss/Documents/antigravity/beautiful-goodall/.agents/orchestrator/PROJECT.md
3. Read the 3 Explorer design handoffs at:
   - /home/dawiisss/Documents/antigravity/beautiful-goodall/.agents/explorer_m2_1/handoff.md
   - /home/dawiisss/Documents/antigravity/beautiful-goodall/.agents/explorer_m2_2/handoff.md
   - /home/dawiisss/Documents/antigravity/beautiful-goodall/.agents/explorer_m2_3/handoff.md

MANDATORY INTEGRITY WARNING:
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_preview_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

Your mission:
Implement Milestone 2: GTK4 / Libadwaita Connection Manager UI with full feature parity.
1. Extend `AppConfig` in `src/models.rs` with `default_protocol`, `auto_connect_last`, `default_vnc_scaling`, `last_connected_id`.
2. Implement `src/ui/window.rs`: `MainWindow` with Libadwaita `adw::HeaderBar`, sidebar connection list grouped by connection `group` strings, real-time `gtk::SearchEntry` filtering, and content pane switching between status page and `ConnectionEditor`.
3. Implement `src/ui/editor.rs`: `ConnectionEditor` widget with `adw::PreferencesGroup`, `adw::EntryRow`, `adw::ComboRow`, `adw::SwitchRow`, `adw::PasswordEntryRow`, Save/Connect/Delete actions, form validation, and `secrets::*_sync` integration.
4. Implement `src/ui/preferences.rs`: `PreferencesWindow` modal dialog for Theme selection (System, Dark, Light), Default Protocol, Auto-connect, and VNC Scaling preferences, connected to `storage::save_config`.
5. Implement `src/ui/discovery.rs`: `DiscoveryDialog` modal for network host scanning and one-click connection creation.
6. Implement `src/ui/mod.rs` exports and `src/main.rs` Libadwaita application entrypoint (`libadwaita::Application`).

Run `cargo check`, `cargo build`, and `cargo test --all-targets` in /home/dawiisss/Documents/antigravity/beautiful-goodall. Confirm 100% clean compilation and test pass.
Write your handoff report to /home/dawiisss/Documents/antigravity/beautiful-goodall/.agents/worker_m2/handoff.md and report back via send_message.
</USER_REQUEST>
