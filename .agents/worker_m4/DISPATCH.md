## 2026-08-12T17:53:01Z
You are worker_m4 (teamwork_preview_worker).
Your working directory is /home/dawiisss/Documents/antigravity/beautiful-goodall/.agents/worker_m4.

Task: Implement Milestone 4 (R4: RDP Launcher, SSH Terminal Launcher, and Wake-on-LAN Integration).

MANDATORY INTEGRITY WARNING:
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_preview_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

Context Files & Specifications to Read First:
1. /home/dawiisss/Documents/antigravity/beautiful-goodall/.agents/ORIGINAL_REQUEST.md
2. /home/dawiisss/Documents/antigravity/beautiful-goodall/.agents/orchestrator/PROJECT.md
3. /home/dawiisss/Documents/antigravity/beautiful-goodall/.agents/explorer_m4_1/handoff.md (RDP xfreerdp3 launcher design)
4. /home/dawiisss/Documents/antigravity/beautiful-goodall/.agents/explorer_m4_2/handoff.md (SSH terminal emulator launcher design)
5. /home/dawiisss/Documents/antigravity/beautiful-goodall/.agents/explorer_m4_3/handoff.md (Wake-on-LAN magic packet design)

Work Scope:
1. Implement `src/network.rs`:
   - `parse_mac_address(mac: &str) -> Result<[u8; 6], String>`: parse colon, hyphen, dot, and hex MAC strings.
   - `build_wol_packet(mac: &[u8; 6]) -> [u8; 102]`: construct 6x0xFF + 16xMAC payload.
   - `send_wol(mac_address: &str) -> Result<(), String>`: UDP broadcast on port 9 (`255.255.255.255:9`).
   - `send_wol_to(mac_address: &str, target_addr: &str) -> Result<(), String>`: UDP send to specified broadcast address/port.
   - Write comprehensive unit tests for network.rs (MAC parsing, packet structure, UDP loopback broadcast delivery).

2. Implement `src/launcher.rs`:
   - `launch_rdp(conn: &Connection, password: Option<&str>) -> Result<std::process::Child, String>`: Build xfreerdp3 Command with `/v`, `/u`, `/p`, `/d`, `/dynamic-resolution`, `+clipboard`, `/sound`, `/multimon`, etc. Detach stdin with `Stdio::null()`.
   - `detect_terminal_emulator() -> Option<(&'static str, PathBuf)>`: Search PATH in order: `ptyxis`, `kgx`, `gnome-terminal`, `konsole`, `alacritty`, `xterm`.
   - `launch_ssh(conn: &Connection) -> Result<std::process::Child, String>` and `launch_ssh_with_identity(conn: &Connection, identity_file: Option<&str>) -> Result<std::process::Child, String>`.
   - Write comprehensive unit tests for launcher.rs.

3. Module Exports & UI Integration:
   - Export `pub mod network;` and `pub mod launcher;` in `src/main.rs` / `src/lib.rs`.
   - Update UI action buttons in `src/ui/editor.rs` / `src/ui/window.rs`:
     - When user clicks Connect on an RDP connection, fetch password (from secret service if present) and call `launcher::launch_rdp`.
     - When user clicks Connect on an SSH connection, call `launcher::launch_ssh`.
     - When user clicks Wake-on-LAN action button, call `network::send_wol` and display toast notification.

4. Build & Verification:
   - Run `cargo build` to ensure error-free compilation.
   - Run `cargo test` to execute all unit tests, module tests, and E2E test targets.
   - Ensure 100% of unit tests and integration tests pass.

5. Report:
   - Write handoff report to /home/dawiisss/Documents/antigravity/beautiful-goodall/.agents/worker_m4/handoff.md.
   - Send message to parent with build/test results and summary.
