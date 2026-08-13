# Milestone 4 Technical Investigation: RDP Launcher Integration via `xfreerdp3`

**Author**: `explorer_m4_1` (teamwork_preview_explorer)  
**Date**: 2026-08-12  
**Target Module**: `src/launcher.rs`  
**Related Models**: `src/models.rs` (`Connection`, `AdvancedSettings`, `Protocol`)  

---

## 1. Observation

### System Environment & Dependencies
- **Binary Path**: `/usr/bin/xfreerdp3` verified on host system.
- **FreeRDP Version**: FreeRDP 3.x series (`xfreerdp3`).
- **Codebase Context**:
  - `src/models.rs`: Defines `Connection` struct, `AdvancedSettings` struct, and `Protocol` enum.
  - `src/launcher.rs`: Currently contains preliminary `build_rdp_args`, `build_ssh_args`, `launch_rdp`, and `launch_ssh` functions.
  - `.agents/orchestrator/PROJECT.md`: Defines interface contract `launcher::launch_rdp(conn: &Connection, password: Option<&str>) -> Result<Child, String>`.

### Model Inspection (`src/models.rs`)
The `Connection` and `AdvancedSettings` structs contain the following relevant fields for RDP connection handling:

```rust
pub struct Connection {
    pub id: String,
    pub name: String,
    pub protocol: Protocol, // Protocol::Rdp
    pub host: String,
    pub port: u16,          // Default 3389 for RDP
    pub username: String,
    pub mac_address: String,
    pub group: String,
    pub advanced_settings: AdvancedSettings,
}

pub struct AdvancedSettings {
    pub rdp_multimon: bool,       // Multi-monitor support
    pub rdp_fullscreen: bool,     // Fullscreen toggle
    pub rdp_audio: bool,          // Audio redirection
    pub vnc_viewonly: bool,
    pub vnc_shared: bool,
    pub clipboard_sharing: bool,  // Bi-directional clipboard
    pub color_depth: u8,          // Color depth (0, 8, 16, 24, 32 bpp)
    pub vnc_scaling: VncScaling,
}
```

---

## 2. Logic Chain & Technical Analysis

### 2.1 `xfreerdp3` CLI Option Mapping

Investigation of `xfreerdp3 --help` on Linux yields the following CLI flag specifications:

| Requirement / Option | `xfreerdp3` CLI Flag | Rust Mapping & Format Logic |
|---|---|---|
| **Host / Port** | `/v:<server>[:<port>]` | `format!("/v:{}:{}", conn.host, conn.resolve_port())` |
| **Username** | `/u:<username>` or `/u:[<domain>\]<user>` | `format!("/u:{}", conn.username)` if `!conn.username.is_empty()` |
| **Password** | `/p:<password>` | `format!("/p:{}", pass)` if `password` is `Some(pass)` and `!pass.is_empty()` |
| **Domain** | `/d:<domain>` | If domain is separate from username, `/d:domain`. If embedded (`DOMAIN\user`), parsed automatically by `/u:` |
| **Dynamic Resolution** | `/dynamic-resolution` | `"/dynamic-resolution".to_string()` (sends display resolution update PDUs on window resize) |
| **Clipboard Sharing** | `+clipboard` / `-clipboard` | `if conn.advanced_settings.clipboard_sharing { "+clipboard" } else { "-clipboard" }` |
| **Audio Redirection** | `/sound` | `if conn.advanced_settings.rdp_audio { "/sound" }` |
| **Multi-Monitor** | `/multimon` | `if conn.advanced_settings.rdp_multimon { "/multimon" }` |
| **Fullscreen / Size** | `/f` or `/size:<W>x<H>` | `if conn.advanced_settings.rdp_fullscreen { "/f" }` |
| **Color Depth** | `/bpp:<depth>` | `if conn.advanced_settings.color_depth > 0 { format!("/bpp:{}", conn.advanced_settings.color_depth) }` |
| **Certificates** | `/cert:ignore` | `"/cert:ignore".to_string()` (suppresses certificate prompts for self-signed RDP endpoints) |

---

### 2.2 Function Design (`src/launcher.rs`)

#### Interface Contract
As specified in `PROJECT.md` and task instructions, the function signature is:

```rust
pub fn launch_rdp(conn: &Connection, password: Option<&str>) -> Result<std::process::Child, String>
```

#### Proposed Code Structure

```rust
use std::process::{Command, Stdio};
use crate::models::Connection;

/// Build the command-line argument list for xfreerdp3 based on connection parameters.
pub fn build_rdp_args(conn: &Connection, password: Option<&str>) -> Vec<String> {
    let port = conn.resolve_port();
    let mut args = vec![format!("/v:{}:{}", conn.host, port)];

    if !conn.username.is_empty() {
        args.push(format!("/u:{}", conn.username));
    }

    if let Some(pass) = password {
        if !pass.is_empty() {
            args.push(format!("/p:{}", pass));
        }
    }

    args.push("/cert:ignore".to_string());
    args.push("/dynamic-resolution".to_string());

    if conn.advanced_settings.clipboard_sharing {
        args.push("+clipboard".to_string());
    } else {
        args.push("-clipboard".to_string());
    }

    if conn.advanced_settings.color_depth > 0 {
        args.push(format!("/bpp:{}", conn.advanced_settings.color_depth));
    }

    if conn.advanced_settings.rdp_multimon {
        args.push("/multimon".to_string());
    }

    if conn.advanced_settings.rdp_fullscreen {
        args.push("/f".to_string());
    }

    if conn.advanced_settings.rdp_audio {
        args.push("/sound".to_string());
    }

    args
}

/// Spawns an xfreerdp3 process detached from the parent process group.
pub fn launch_rdp(conn: &Connection, password: Option<&str>) -> Result<std::process::Child, String> {
    if conn.host.trim().is_empty() {
        return Err("Connection host cannot be empty".to_string());
    }

    let args = build_rdp_args(conn, password);
    let mut cmd = Command::new("xfreerdp3");

    cmd.args(&args)
       .stdin(Stdio::null())
       .stdout(Stdio::null())
       .stderr(Stdio::null());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            cmd.pre_exec(|| {
                // Detach process into a new POSIX session and process group
                libc::setsid();
                Ok(())
            });
        }
    }

    cmd.spawn()
       .map_err(|e| format!("Failed to spawn xfreerdp3 process: {}", e))
}
```

---

### 2.3 Process Spawning & Detachment Mechanics

When launching external RDP sessions from a GUI connection manager, active sessions must remain running even if the connection manager application is closed or restarted.

#### 1. Standard I/O Redirection (`Stdio::null()`)
- By default, `std::process::Command::spawn()` inherits the standard input, output, and error file descriptors from the parent GTK process.
- Redirecting streams via `.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())` ensures that standard input/output file descriptors are closed and point to `/dev/null`.
- This prevents `xfreerdp3` from blocking on stdin or keeping parent pipe descriptors open.

#### 2. Process Group & POSIX Session Separation (`setsid`)
- On Unix/Linux systems, child processes created via `fork()` inherit the Process Group ID (PGID) and Session ID (SID) of the parent process.
- If the parent GTK connection manager is launched from a terminal or terminal session, terminating the parent or closing the window may broadcast SIGHUP to the entire process group.
- Calling `libc::setsid()` inside `pre_exec` before `execvp` creates a new POSIX session with the child process as the session leader and process group leader. This completely detaches `xfreerdp3` from the controlling terminal and parent process group.

#### 3. Rust `std::process::Child` Drop Behavior
- In Rust, `std::process::Child` is a RAII handle to the process.
- **Crucial Behavior**: Dropping a `Child` struct in Rust does **NOT** send `SIGKILL` or `SIGTERM` to the process. It simply drops the handle.
- When the parent process terminates, the detached `xfreerdp3` process is orphaned and automatically adopted by `PID 1` (`init` or `systemd`), remaining active and fully functional.

---

## 3. Caveats

1. **Password Security in Process Table**:
   - Passing passwords via command-line arguments (`/p:password`) exposes the password in plain text in `/proc/<PID>/cmdline` on Linux systems.
   - Any local user running `ps aux` or inspecting `/proc` during session startup can view the argument string.
   - *Recommendation*: Document this limitation or consider passing arguments via `/args-from:stdin` or environment variables in future security updates if requested.

2. **Missing Binary Error Handling**:
   - If `xfreerdp3` is not installed on the target system, `Command::spawn()` fails with `std::io::ErrorKind::NotFound`.
   - The UI layer should handle the `Err(String)` result and present a friendly GTK dialog informing the user to install `freerdp3`.

3. **Domain Formatting**:
   - FreeRDP 3 handles domain prefixes within username (e.g., `DOMAIN\user` or `user@domain`). If separate domain fields are added in future model revisions, `/d:<domain>` can be appended to `build_rdp_args`.

---

## 4. Conclusion

- The CLI argument mapping for `xfreerdp3` in FreeRDP 3 is fully compatible with the fields of `Connection` and `AdvancedSettings`.
- The proposed `launch_rdp` function signature (`Result<std::process::Child, String>`) cleanly aligns with the contract in `PROJECT.md`.
- Combining `Stdio::null()` with `libc::setsid()` via `CommandExt::pre_exec` ensures complete process group detachment, allowing RDP sessions to survive parent connection manager termination.

---

## 5. Verification Method

To verify the implementation once written to `src/launcher.rs`:

1. **Compilation Check**:
   ```bash
   cargo check
   ```

2. **Unit Testing Argument Generation**:
   Add unit tests in `src/launcher.rs` to verify that `build_rdp_args` generates the expected CLI flags for various `Connection` configurations:
   ```bash
   cargo test --lib launcher::tests
   ```

3. **Full Integration & E2E Test Verification**:
   ```bash
   cargo test
   ```
