use crate::models::{Connection, RdpColorDepth, RdpNetworkProfile};
use std::env;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

/// Linux terminal emulator search order for interactive SSH sessions.
pub const TERMINAL_CANDIDATES: &[&str] = &[
    "ptyxis",
    "kgx",
    "gnome-terminal",
    "konsole",
    "alacritty",
    "xterm",
];

/// Searches for a binary by name in the directories listed in system `PATH`.
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

/// Detects the highest priority terminal emulator available in system `PATH`.
/// Returns `Option<(&'static str, PathBuf)>` matching binary name and executable path.
pub fn detect_terminal_emulator() -> Option<(&'static str, PathBuf)> {
    for &term in TERMINAL_CANDIDATES {
        if let Some(path) = find_binary_in_path(term) {
            return Some((term, path));
        }
    }
    None
}

/// Build command-line argument list for `xfreerdp3` based on connection parameters.
pub fn build_rdp_args(conn: &Connection, password: Option<&str>) -> Vec<String> {
    let port = conn.resolve_port();
    let mut args = vec![format!("/v:{}:{}", conn.host, port)];

    if !conn.username.trim().is_empty() {
        args.push(format!("/u:{}", conn.username.trim()));
    }

    if let Some(pass) = password {
        if !pass.is_empty() {
            args.push(format!("/p:{}", pass));
        }
    }

    if !conn.advanced_settings.rdp_domain.trim().is_empty() {
        args.push(format!("/d:{}", conn.advanced_settings.rdp_domain.trim()));
    }

    args.push("/cert:ignore".to_string());
    if conn.advanced_settings.rdp_dynamic_resolution {
        args.push("/dynamic-resolution".to_string());
    }

    if conn.advanced_settings.clipboard_sharing {
        args.push("+clipboard".to_string());
    }

    match conn.advanced_settings.rdp_color_depth {
        RdpColorDepth::Automatic => {} // No flags for Auto
        RdpColorDepth::GfxAvc444 => {
            args.push("/bpp:32".to_string());
            args.push("/gfx:AVC444".to_string());
        }
        RdpColorDepth::GfxAvc420 => {
            args.push("/bpp:32".to_string());
            args.push("/gfx:AVC420".to_string());
        }
        RdpColorDepth::GfxRfx => {
            args.push("/bpp:32".to_string());
            args.push("/gfx:RFX".to_string());
        }
        RdpColorDepth::GfxRfxProgressive => {
            args.push("/bpp:32".to_string());
            args.push("/gfx:progressive".to_string());
        }
        RdpColorDepth::RemoteFx => {
            args.push("/bpp:32".to_string());
            args.push("-gfx".to_string());
            args.push("/rfx".to_string());
        }
        RdpColorDepth::TrueColor32 => {
            args.push("/bpp:32".to_string());
            args.push("-gfx".to_string());
            args.push("-rfx".to_string());
        }
        RdpColorDepth::TrueColor24 => {
            args.push("/bpp:24".to_string());
            args.push("-gfx".to_string());
            args.push("-rfx".to_string());
        }
        RdpColorDepth::HighColor16 => {
            args.push("/bpp:16".to_string());
            args.push("-gfx".to_string());
            args.push("-rfx".to_string());
        }
        RdpColorDepth::HighColor15 => {
            args.push("/bpp:15".to_string());
            args.push("-gfx".to_string());
            args.push("-rfx".to_string());
        }
        RdpColorDepth::Colors256 => {
            args.push("/bpp:8".to_string());
            args.push("-gfx".to_string());
            args.push("-rfx".to_string());
        }
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

    if !conn.advanced_settings.rdp_gateway.trim().is_empty() {
        args.push(format!("/g:{}", conn.advanced_settings.rdp_gateway.trim()));
    }

    if !conn.advanced_settings.rdp_shared_folder.trim().is_empty() {
        args.push(format!(
            "/drive:shared,{}",
            conn.advanced_settings.rdp_shared_folder.trim()
        ));
    }

    if !conn
        .advanced_settings
        .rdp_custom_resolution
        .trim()
        .is_empty()
        && !conn.advanced_settings.rdp_fullscreen
    {
        args.push(format!(
            "/size:{}",
            conn.advanced_settings.rdp_custom_resolution.trim()
        ));
    }

    if conn.advanced_settings.rdp_glyph_cache {
        args.push("/cache:glyph:on".to_string());
    }

    if conn.advanced_settings.rdp_microphone {
        args.push("/microphone".to_string());
    }

    if conn.advanced_settings.rdp_usb_redirect {
        args.push("/usb:auto".to_string());
    }

    if conn.advanced_settings.rdp_smooth_fonts {
        args.push("+fonts".to_string());
    }

    if conn.advanced_settings.rdp_desktop_composition {
        args.push("+aero".to_string());
    }

    if conn.advanced_settings.rdp_hw_accel {
        args.push("/gfx".to_string());
    }

    match conn.advanced_settings.rdp_network_profile {
        RdpNetworkProfile::Auto => {} // Don't push any flag by default
        RdpNetworkProfile::Lan => args.push("/network:lan".to_string()),
        RdpNetworkProfile::Wan => args.push("/network:wan".to_string()),
        RdpNetworkProfile::Broadband => args.push("/network:broadband".to_string()),
        RdpNetworkProfile::Modem => args.push("/network:modem".to_string()),
    }

    if conn.advanced_settings.rdp_disable_wallpaper {
        args.push("-wallpaper".to_string());
    }

    if conn.advanced_settings.rdp_disable_themes {
        args.push("-themes".to_string());
    }

    if conn.advanced_settings.rdp_disable_animations {
        args.push("-window-drag".to_string());
        args.push("-menu-anims".to_string());
    }

    args
}

/// Build the `ssh` command argument vector with optional identity key file.
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

/// Construct a configured `std::process::Command` for launching a specific terminal emulator.
pub fn build_terminal_command(
    term_name: &str,
    conn: &Connection,
    identity_file: Option<&str>,
) -> Command {
    let ssh_args = build_ssh_args_with_identity(conn, identity_file);
    let mut cmd = Command::new(term_name);

    match term_name {
        "ptyxis" | "gnome-terminal" => {
            cmd.arg("--").args(&ssh_args);
        }
        "kgx" => {
            let ssh_str = ssh_args.join(" ");
            cmd.arg("-e").arg(ssh_str);
        }
        _ => {
            // konsole, alacritty, xterm and fallbacks
            cmd.arg("-e").args(&ssh_args);
        }
    }

    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    cmd
}

/// Spawns an `xfreerdp3` RDP session detached from parent process group.
pub fn launch_rdp(conn: &Connection, password: Option<&str>) -> Result<Child, String> {
    if conn.host.trim().is_empty() {
        return Err("Connection host cannot be empty".to_string());
    }

    let args = build_rdp_args(conn, password);
    let mut cmd = Command::new("xfreerdp3");

    cmd.args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    cmd.spawn()
        .map_err(|e| format!("Failed to spawn xfreerdp3 process: {}", e))
}

/// Launches an SSH session in an available terminal emulator with an optional SSH identity key file.
pub fn launch_ssh_with_identity(
    conn: &Connection,
    identity_file: Option<&str>,
) -> Result<Child, String> {
    if conn.host.trim().is_empty() {
        return Err("Connection host cannot be empty".to_string());
    }

    let (term_name, _path) = detect_terminal_emulator().ok_or_else(|| {
        format!(
            "No supported terminal emulator found on PATH (searched: {})",
            TERMINAL_CANDIDATES.join(", ")
        )
    })?;

    let mut cmd = build_terminal_command(term_name, conn, identity_file);
    cmd.spawn()
        .map_err(|e| format!("Failed to spawn terminal emulator '{}': {}", term_name, e))
}

/// Launches an SSH session in an available terminal emulator.
pub fn launch_ssh(conn: &Connection) -> Result<Child, String> {
    launch_ssh_with_identity(conn, None)
}

/// Build command-line argument list for `remote-viewer` based on connection parameters.
pub fn build_spice_args(conn: &Connection) -> Vec<String> {
    let mut args = Vec::new();
    args.push(format!("--title={}", conn.name));

    if conn.advanced_settings.spice_fullscreen {
        args.push("--fullscreen".to_string());
    }

    if conn.advanced_settings.spice_scale_to_window {
        args.push("--auto-resize=always".to_string());
    }

    if conn.advanced_settings.spice_usb_redirect {
        args.push("--spice-usbredir-auto-redirect-filter=-1,-1,-1,-1,0".to_string());
    }

    let port = conn.resolve_port();
    let uri = format!("spice://{}:{}", conn.host, port);
    args.push(uri);
    args
}

pub fn build_vnc_args(conn: &Connection) -> Vec<String> {
    let mut args = vec![];

    if conn.advanced_settings.vnc_viewonly {
        args.push("-ViewOnly".to_string());
    }

    if conn.advanced_settings.vnc_shared {
        args.push("-Shared".to_string());
    }

    use crate::models::{VncColorLevel, VncEncodingOption};
    match conn.advanced_settings.vnc_encoding {
        VncEncodingOption::Tight => args.push("-PreferredEncoding=Tight".to_string()),
        VncEncodingOption::Zrle => args.push("-PreferredEncoding=ZRLE".to_string()),
        VncEncodingOption::Raw => args.push("-PreferredEncoding=Raw".to_string()),
        VncEncodingOption::Auto => {} // Auto is TigerVNC default
    }

    if conn.advanced_settings.vnc_fullscreen {
        args.push("-FullScreen=1".to_string());
    }

    if !conn.advanced_settings.vnc_clipboard {
        args.push("-AcceptClipboard=0".to_string());
        args.push("-SendClipboard=0".to_string());
    }

    match conn.advanced_settings.vnc_color_level {
        VncColorLevel::Full => {} // AutoSelect is default. Passing -FullColor=1 disables AutoSelect.
        VncColorLevel::Medium => args.push("-LowColorLevel=2".to_string()),
        VncColorLevel::Low => args.push("-LowColorLevel=1".to_string()),
        VncColorLevel::VeryLow => args.push("-LowColorLevel=0".to_string()),
    }

    if conn.advanced_settings.vnc_compress_level > 0 {
        args.push(format!(
            "-CompressLevel={}",
            conn.advanced_settings.vnc_compress_level
        ));
    }

    if conn.advanced_settings.vnc_quality_level > 0 {
        args.push(format!(
            "-QualityLevel={}",
            conn.advanced_settings.vnc_quality_level
        ));
    }

    let resolved_port = conn.resolve_port();
    let port_to_use = if resolved_port == 0 {
        5900
    } else {
        resolved_port
    };
    args.push(format!("{}:{}", conn.host.trim(), port_to_use));

    args
}

pub fn launch_vnc(conn: &Connection, password: Option<&str>) -> Result<Child, String> {
    if conn.host.trim().is_empty() {
        return Err("Connection host cannot be empty".to_string());
    }

    let mut args = build_vnc_args(conn);

    // Check if we have a password
    let mut temp_file_path = None;

    if let Some(pass) = password.filter(|p| !p.is_empty()) {
        // Use vncpasswd -f to encrypt the password
        let mut vncpasswd = Command::new("vncpasswd")
            .arg("-f")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to run vncpasswd: {}", e))?;

        {
            use std::io::Write;
            if let Some(mut stdin) = vncpasswd.stdin.take() {
                let _ = stdin.write_all(pass.as_bytes());
                let _ = stdin.write_all(b"\n");
                let _ = stdin.write_all(pass.as_bytes());
                let _ = stdin.write_all(b"\n");
            }
        }

        let output = vncpasswd
            .wait_with_output()
            .map_err(|e| format!("vncpasswd failed: {}", e))?;
        if output.status.success() {
            use std::io::Write;
            let mut builder = tempfile::Builder::new();
            builder.prefix("ver_vnc_").suffix(".pwd");
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                builder.permissions(std::fs::Permissions::from_mode(0o600));
            }
            if let Ok(mut temp_file) = builder.tempfile() {
                if temp_file.write_all(&output.stdout).is_ok() {
                    if let Ok((_, path)) = temp_file.keep() {
                        let path_str = path.to_string_lossy().to_string();
                        args.push("-passwd".to_string());
                        args.push(path_str.clone());
                        temp_file_path = Some(path_str);
                    }
                }
            }
        }
    }

    let mut cmd = Command::new("vncviewer");

    cmd.args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    let child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn vncviewer process: {}", e))?;

    // Clean up password file after viewer has had time to read it
    if let Some(path) = temp_file_path {
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(3));
            let _ = std::fs::remove_file(path);
        });
    }

    Ok(child)
}

/// Spawns a `remote-viewer` SPICE session detached from parent process group.
pub fn launch_spice(conn: &Connection, password: Option<&str>) -> Result<Child, String> {
    if conn.host.trim().is_empty() {
        return Err("Connection host cannot be empty".to_string());
    }

    let args = build_spice_args(conn);
    let mut cmd = Command::new("remote-viewer");

    if let Some(pass) = password {
        if !pass.is_empty() {
            cmd.env("SPICE_PASSWORD", pass);
        }
    }

    cmd.args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    cmd.spawn()
        .map_err(|e| format!("Failed to spawn remote-viewer process: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Protocol;

    #[test]
    fn test_build_rdp_args_standard() {
        let conn = Connection {
            host: "rdp.example.com".to_string(),
            port: 3389,
            username: "administrator".to_string(),
            advanced_settings: crate::models::AdvancedSettings {
                clipboard_sharing: true,
                rdp_color_depth: RdpColorDepth::TrueColor32,
                rdp_multimon: true,
                rdp_fullscreen: true,
                rdp_audio: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let args = build_rdp_args(&conn, Some("MySecretPass"));

        assert!(args.contains(&"/v:rdp.example.com:3389".to_string()));
        assert!(args.contains(&"/u:administrator".to_string()));
        assert!(args.contains(&"/p:MySecretPass".to_string()));
        assert!(args.contains(&"/cert:ignore".to_string()));
        assert!(args.contains(&"/dynamic-resolution".to_string()));
        assert!(args.contains(&"+clipboard".to_string()));
        assert!(args.contains(&"/bpp:32".to_string()));
        assert!(args.contains(&"/multimon".to_string()));
        assert!(args.contains(&"/f".to_string()));
        assert!(args.contains(&"/sound".to_string()));
    }

    #[test]
    fn test_build_rdp_args_default_port_resolution() {
        let conn = Connection {
            protocol: Protocol::Rdp,
            host: "10.0.0.1".to_string(),
            port: 0, // Default port should resolve to 3389
            ..Default::default()
        };

        let args = build_rdp_args(&conn, None);
        assert!(args.contains(&"/v:10.0.0.1:3389".to_string()));
    }

    #[test]
    fn test_build_ssh_args_custom_port() {
        let conn = Connection {
            protocol: Protocol::Ssh,
            host: "bastion.example.com".to_string(),
            port: 2222,
            username: "devops".to_string(),
            ..Default::default()
        };

        let args = build_ssh_args(&conn);

        assert_eq!(args[0], "ssh");
        assert_eq!(args[1], "-p");
        assert_eq!(args[2], "2222");
        assert_eq!(args[3], "devops@bastion.example.com");
    }

    #[test]
    fn test_build_ssh_args_default_port_22() {
        let conn = Connection {
            protocol: Protocol::Ssh,
            host: "shell.example.com".to_string(),
            port: 22,
            username: "root".to_string(),
            ..Default::default()
        };

        let args = build_ssh_args(&conn);

        assert_eq!(args, vec!["ssh", "root@shell.example.com"]);
    }

    #[test]
    fn test_build_ssh_args_with_identity_file() {
        let conn = Connection {
            protocol: Protocol::Ssh,
            host: "secure.example.com".to_string(),
            port: 2222,
            username: "admin".to_string(),
            ..Default::default()
        };

        let args = build_ssh_args_with_identity(&conn, Some("/home/user/.ssh/id_ed25519"));

        assert_eq!(
            args,
            vec![
                "ssh",
                "-p",
                "2222",
                "-i",
                "/home/user/.ssh/id_ed25519",
                "admin@secure.example.com"
            ]
        );
    }

    #[test]
    fn test_detect_terminal_emulator_candidates_list() {
        assert_eq!(
            TERMINAL_CANDIDATES,
            &[
                "ptyxis",
                "kgx",
                "gnome-terminal",
                "konsole",
                "alacritty",
                "xterm"
            ]
        );
    }

    #[test]
    fn test_launch_rdp_empty_host_validation() {
        let conn = Connection::default();
        assert!(launch_rdp(&conn, None).is_err());
    }

    #[test]
    fn test_launch_ssh_empty_host_validation() {
        let conn = Connection::default();
        assert!(launch_ssh(&conn).is_err());
    }
}
