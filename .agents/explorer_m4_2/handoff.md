# Technical Investigation & Handoff Report: Milestone 4 (R4: SSH Terminal Launcher Integration)

## 1. Observation

### 1.1 Existing Architecture & Files Inspected
- **`src/models.rs`**:
  - `Connection` struct (lines 134–153):
    - `pub id: String`
    - `pub name: String`
    - `pub protocol: Protocol` (`Rdp`, `Vnc`, `Ssh`)
    - `pub host: String`
    - `pub port: u16` (default for SSH is `22`, resolved via `conn.resolve_port()`)
    - `pub username: String`
    - `pub mac_address: String`
    - `pub group: String`
    - `pub advanced_settings: AdvancedSettings`
  - `Protocol` enum (lines 5–11):
    - `Protocol::Ssh.default_port()` returns `22`.
- **`src/core/launcher.py`** (Legacy Python reference, lines 90–118):
  - SSH command assembly: `["ssh", "-p", str(port), "user@host"]`.
  - Terminal candidate search order: `ptyxis`, `kgx`, `gnome-terminal`, `konsole`, `xfce4-terminal`, `kitty`, `alacritty`, `xterm`.
- **`src/launcher.rs`** (Current Rust implementation, lines 41–55, 68–78):
  - `pub fn build_ssh_args(conn: &Connection) -> Vec<String>`: Currently builds `["ssh", "-p", "<port>", "user@host"]` when port != 22.
  - `pub fn launch_ssh(conn: &Connection) -> Result<std::process::Child>`: Hardcoded to spawn `ptyxis -- ssh ...` without dynamic terminal detection or error reporting when missing.
- **`tests/e2e_launcher_tests.rs`** (lines 47–73):
  - `test_build_ssh_args_custom_port()`: Expects `["ssh", "-p", "2222", "devops@bastion.example.com"]`.
  - `test_build_ssh_args_default_port_22()`: Expects `["ssh", "root@shell.example.com"]`.

---

## 2. Logic Chain

1. **Terminal Emulator Detection Necessity**:
   - Hardcoding `ptyxis` in `launch_ssh` causes instant runtime failures on Linux systems where `ptyxis` is not installed (e.g. Debian, Ubuntu, Arch, KDE Plasma desktop environments).
   - A robust Linux terminal detection routine must inspect system `PATH` directories in priority order:
     `ptyxis` → `kgx` → `gnome-terminal` → `konsole` → `alacritty` → `xterm`.

2. **PATH Lookup Mechanism**:
   - `std::env::var_os("PATH")` fetches system execution paths.
   - `std::env::split_paths()` parses standard OS delimiters (`:` on Unix/Linux).
   - `path.join(binary_name).is_file()` determines whether the binary exists.

3. **Argument Mapping Differences across Terminal Emulators**:
   - Different terminal emulators expect different argument structures for running child commands:
     - `ptyxis`: Uses `--` separator: `ptyxis -- ssh -p 2222 -i id_rsa user@host`
     - `kgx` (GNOME Console): Uses `-e` with single command string: `kgx -e "ssh -p 2222 -i id_rsa user@host"`
     - `gnome-terminal`: Uses `--` separator: `gnome-terminal -- ssh -p 2222 -i id_rsa user@host`
     - `konsole`: Uses `-e` with separate arguments: `konsole -e ssh -p 2222 -i id_rsa user@host`
     - `alacritty`: Uses `-e` with separate arguments: `alacritty -e ssh -p 2222 -i id_rsa user@host`
     - `xterm`: Uses `-e` with separate arguments: `xterm -e ssh -p 2222 -i id_rsa user@host`

4. **SSH Argument Generation**:
   - Base command: `"ssh"`
   - Port flag: If `conn.resolve_port() != 22 && conn.resolve_port() != 0`, append `"-p"` and `port.to_string()`.
   - Identity file flag: If identity file path `Some(key)` is provided, append `"-i"` and `<identity_file>`.
   - Target host/user: If `username` is present, `user@host`; otherwise `host`.

5. **Error Handling & Stdio Isolation**:
   - Stdio must be redirected to `Stdio::null()` so closing the connection manager GUI app does not kill or hang the detached terminal window.
   - If no terminal emulator exists in `PATH`, `launch_ssh` must return an informative `Err(String)` detailing searched terminals.

---

## 3. Caveats

- **System Compatibility**: Non-GUI environments or systems without any installed X11/Wayland terminal emulator will fail gracefully with `Err` listing searched candidates.
- **Identity File Option**: Identity file support is provided via `build_ssh_args_with_identity(conn, identity_file)` and `launch_ssh_with_identity(conn, identity_file)`. `build_ssh_args(conn)` remains 100% backward-compatible with existing tests.
- **`kgx` Single-String Argument Requirement**: GNOME Console (`kgx`) `-e` requires a single command string (`kgx -e "ssh ..."`), whereas `konsole`, `alacritty`, `xterm` take positional arguments after `-e`. The proposed `build_terminal_command` function explicitly accounts for this distinction.

---

## 4. Conclusion & Proposed Implementation Specification

### 4.1 Terminal Emulator Detection & Invocation Specs

```
+----------------+--------------+------------------+-------------------------------------------------------+
| Terminal       | Binary Name  | Flag / Separator | Invocation Argument Structure                         |
+----------------+--------------+------------------+-------------------------------------------------------+
| Ptyxis         | ptyxis       | --               | ["ptyxis", "--", "ssh", ...]                          |
| GNOME Console  | kgx          | -e               | ["kgx", "-e", "ssh -p 2222 -i id_rsa user@host"]      |
| GNOME Terminal | gnome-terminal| --               | ["gnome-terminal", "--", "ssh", ...]                  |
| KDE Konsole    | konsole      | -e               | ["konsole", "-e", "ssh", ...]                         |
| Alacritty      | alacritty    | -e               | ["alacritty", "-e", "ssh", ...]                       |
| XTerm          | xterm        | -e               | ["xterm", "-e", "ssh", ...]                           |
+----------------+--------------+------------------+-------------------------------------------------------+
```

### 4.2 Code Blueprint for `src/launcher.rs`

```rust
use std::env;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use crate::models::Connection;

/// Terminal emulator variants supported for launching interactive SSH sessions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalEmulator {
    Ptyxis,
    Kgx,
    GnomeTerminal,
    Konsole,
    Alacritty,
    Xterm,
}

impl TerminalEmulator {
    pub fn binary_name(&self) -> &'static str {
        match self {
            TerminalEmulator::Ptyxis => "ptyxis",
            TerminalEmulator::Kgx => "kgx",
            TerminalEmulator::GnomeTerminal => "gnome-terminal",
            TerminalEmulator::Konsole => "konsole",
            TerminalEmulator::Alacritty => "alacritty",
            TerminalEmulator::Xterm => "xterm",
        }
    }
}

/// Linux terminal emulator priority search order.
pub const TERMINAL_SEARCH_ORDER: &[TerminalEmulator] = &[
    TerminalEmulator::Ptyxis,
    TerminalEmulator::Kgx,
    TerminalEmulator::GnomeTerminal,
    TerminalEmulator::Konsole,
    TerminalEmulator::Alacritty,
    TerminalEmulator::Xterm,
];

/// Checks if a binary name exists in the system `PATH`.
pub fn find_binary_in_path(binary_name: &str) -> Option<PathBuf> {
    let path_os = env::var_os("PATH")?;
    for dir in env::split_paths(&path_os) {
        let candidate = dir.join(binary_name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Detects the highest priority terminal emulator available on the host system.
pub fn detect_terminal_emulator() -> Option<TerminalEmulator> {
    for &term in TERMINAL_SEARCH_ORDER {
        if find_binary_in_path(term.binary_name()).is_some() {
            return Some(term);
        }
    }
    None
}

/// Builds the `ssh` command argument vector with optional identity file.
pub fn build_ssh_args_with_identity(conn: &Connection, identity_file: Option<&str>) -> Vec<String> {
    let mut ssh_args = vec!["ssh".to_string()];

    let resolved_port = conn.resolve_port();
    if resolved_port != 0 && resolved_port != 22 {
        ssh_args.push("-p".to_string());
        ssh_args.push(resolved_port.to_string());
    }

    if let Some(key_path) = identity_file {
        if !key_path.trim().is_empty() {
            ssh_args.push("-i".to_string());
            ssh_args.push(key_path.trim().to_string());
        }
    }

    let target = if !conn.username.trim().is_empty() {
        format!("{}@{}", conn.username.trim(), conn.host.trim())
    } else {
        conn.host.trim().to_string()
    };
    ssh_args.push(target);

    ssh_args
}

/// Standard `build_ssh_args` wrapper for backward compatibility with existing tests.
pub fn build_ssh_args(conn: &Connection) -> Vec<String> {
    build_ssh_args_with_identity(conn, None)
}

/// Constructs a configured `std::process::Command` for spawning the terminal emulator.
pub fn build_terminal_command(
    term: TerminalEmulator,
    conn: &Connection,
    identity_file: Option<&str>,
) -> Command {
    let ssh_args = build_ssh_args_with_identity(conn, identity_file);
    let mut cmd = Command::new(term.binary_name());

    match term {
        TerminalEmulator::Ptyxis | TerminalEmulator::GnomeTerminal => {
            cmd.arg("--").args(&ssh_args);
        }
        TerminalEmulator::Kgx => {
            let ssh_str = ssh_args.join(" ");
            cmd.arg("-e").arg(ssh_str);
        }
        TerminalEmulator::Konsole | TerminalEmulator::Alacritty | TerminalEmulator::Xterm => {
            cmd.arg("-e").args(&ssh_args);
        }
    }

    cmd.stdin(Stdio::null())
       .stdout(Stdio::null())
       .stderr(Stdio::null());

    cmd
}

/// Launches an SSH session in an available terminal emulator.
pub fn launch_ssh(conn: &Connection) -> Result<Child, String> {
    launch_ssh_with_identity(conn, None)
}

/// Launches an SSH session in an available terminal emulator with an optional SSH identity key file.
pub fn launch_ssh_with_identity(
    conn: &Connection,
    identity_file: Option<&str>,
) -> Result<Child, String> {
    let term = detect_terminal_emulator().ok_or_else(|| {
        format!(
            "No supported terminal emulator found on PATH (searched: {})",
            TERMINAL_SEARCH_ORDER
                .iter()
                .map(|t| t.binary_name())
                .collect::<Vec<_>>()
                .join(", ")
        )
    })?;

    let mut cmd = build_terminal_command(term, conn, identity_file);
    cmd.spawn()
       .map_err(|e| format!("Failed to spawn terminal emulator '{}': {}", term.binary_name(), e))
}
```

---

## 5. Verification Method

### 5.1 Verification Commands
To independently verify the SSH launcher integration implementation:

1. **Run existing E2E launcher test suite**:
   ```bash
   cargo test --test e2e_launcher_tests
   ```
2. **Run comprehensive unit tests in `src/launcher.rs`**:
   ```bash
   cargo test launcher::tests
   ```

### 5.2 Unit Tests to Add in `src/launcher.rs`
- `test_detect_terminal_emulator_order()`: Mock or verify PATH lookup order.
- `test_build_ssh_args_with_identity_file()`: Verify `-i /path/to/key.pem` parameter is inserted prior to `<user>@<host>`.
- `test_build_terminal_command_ptyxis()`: Verify `ptyxis -- ssh ...` argument layout.
- `test_build_terminal_command_kgx()`: Verify `kgx -e "ssh ..."` single string argument layout.
- `test_build_terminal_command_konsole()`: Verify `konsole -e ssh ...` multi-arg argument layout.
- `test_launch_ssh_missing_terminal_emulator_returns_error()`: Verify error message when no terminal binary is found in PATH.

### 5.3 Invalidation Conditions
- Any changes that break existing `build_ssh_args(conn)` behavior tested in `tests/e2e_launcher_tests.rs`.
- Failure to handle `-e` vs `--` flags properly across different terminal emulators.
- Failure to return an `Err` when no terminal emulator is present in `PATH`.
