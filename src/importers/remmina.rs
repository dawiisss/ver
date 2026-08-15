use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::importers::ImporterError;
use crate::models::{
    AdvancedSettings, Connection, Protocol, RdpCertHandling, RdpColorDepth, RdpSecurityProtocol,
};

/// Parses a Remmina profile string into a VER `Connection`.
pub fn import_remmina_content(
    content: &str,
    file_name: Option<&str>,
) -> Result<Connection, ImporterError> {
    let mut props = HashMap::new();
    let mut in_remmina_section = false;
    let mut found_any_section = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            found_any_section = true;
            let section = &trimmed[1..trimmed.len() - 1];
            in_remmina_section = section.eq_ignore_ascii_case("remmina");
            continue;
        }

        // If no sections are found, assume top-level keys belong to remmina profile
        if !found_any_section || in_remmina_section {
            if let Some((key, val)) = trimmed.split_once('=') {
                props.insert(key.trim().to_lowercase(), val.trim().to_string());
            }
        }
    }

    if props.is_empty() {
        return Err(ImporterError::InvalidFormat(
            "Remmina profile is empty or contains no valid properties".to_string(),
        ));
    }

    // Protocol
    let proto_str = props.get("protocol").map(|s| s.as_str()).unwrap_or("RDP");
    let protocol = match proto_str.to_uppercase().as_str() {
        "RDP" => Protocol::Rdp,
        "VNC" => Protocol::Vnc,
        "SPICE" => Protocol::Spice,
        "SSH" => Protocol::Ssh,
        other => {
            return Err(ImporterError::InvalidFormat(format!(
                "Unsupported Remmina protocol: {}",
                other
            )));
        }
    };

    // Server (Host & Port)
    let raw_server = props.get("server").cloned().unwrap_or_default();
    let (host, port) = parse_host_port(&raw_server, protocol.default_port());

    // Name
    let fallback_name = file_name
        .and_then(|f| Path::new(f).file_stem())
        .and_then(|s| s.to_str())
        .unwrap_or(if host.is_empty() {
            "Imported Remmina"
        } else {
            &host
        });

    let name = props
        .get("name")
        .filter(|s| !s.is_empty())
        .cloned()
        .unwrap_or_else(|| fallback_name.to_string());

    let group = props.get("group").cloned().unwrap_or_default();
    let username = props.get("username").cloned().unwrap_or_default();
    let domain = props.get("domain").cloned().unwrap_or_default();
    let gateway = props.get("gateway_server").cloned().unwrap_or_default();
    let sharefolder = props.get("sharefolder").cloned().unwrap_or_default();
    let ssh_key = props
        .get("ssh_tunnel_privatekey")
        .or_else(|| props.get("ssh_tunnel_certfile"))
        .cloned()
        .unwrap_or_default();

    // Color depth
    let color_depth_val = props
        .get("colordepth")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(32);
    let rdp_color_depth = match color_depth_val {
        8 => RdpColorDepth::Colors256,
        15 => RdpColorDepth::HighColor15,
        16 => RdpColorDepth::HighColor16,
        24 => RdpColorDepth::TrueColor24,
        _ => RdpColorDepth::TrueColor32,
    };

    // Cert Handling
    let cert_ignore = props.get("cert_ignore").map(|v| v == "1").unwrap_or(false)
        || props
            .get("ignore-tls-errors")
            .map(|v| v == "1")
            .unwrap_or(false);
    let rdp_cert_handling = if cert_ignore {
        RdpCertHandling::Ignore
    } else {
        RdpCertHandling::Tofu
    };

    // Security Protocol
    let sec_str = props
        .get("security")
        .map(|s| s.to_lowercase())
        .unwrap_or_default();
    let rdp_security = match sec_str.as_str() {
        "nla" => RdpSecurityProtocol::Nla,
        "tls" => RdpSecurityProtocol::Tls,
        "rdp" => RdpSecurityProtocol::Rdp,
        _ => RdpSecurityProtocol::Auto,
    };

    // Booleans
    let multimon = props.get("multimon").map(|v| v == "1").unwrap_or(false)
        || props
            .get("force_multimon")
            .map(|v| v == "1")
            .unwrap_or(false);
    let fullscreen = props.get("viewmode").map(|v| v == "1").unwrap_or(false)
        || props
            .get("window_maximize")
            .map(|v| v == "1")
            .unwrap_or(false);
    let disable_clipboard = props
        .get("disableclipboard")
        .map(|v| v == "1")
        .unwrap_or(false);
    let sound_val = props.get("sound").map(|s| s.as_str()).unwrap_or("off");
    let audio = sound_val != "off" && sound_val != "none";
    let microphone = props.get("microphone").map(|v| v == "1").unwrap_or(false);
    let glyph_cache = props.get("glyph-cache").map(|v| v == "1").unwrap_or(true);

    let advanced_settings = AdvancedSettings {
        rdp_multimon: multimon,
        rdp_fullscreen: fullscreen,
        rdp_audio: audio,
        vnc_viewonly: false,
        vnc_shared: false,
        vnc_fullscreen: fullscreen,
        vnc_clipboard: !disable_clipboard,
        vnc_color_level: crate::models::VncColorLevel::Full,
        vnc_encoding: crate::models::VncEncodingOption::Auto,
        vnc_compress_level: 0,
        vnc_quality_level: 0,
        clipboard_sharing: !disable_clipboard,
        color_depth: 32,
        rdp_color_depth,
        rdp_cert_handling,
        rdp_security,
        spice_fullscreen: fullscreen,
        spice_usb_redirect: false,
        spice_scale_to_window: true,
        rdp_domain: domain,
        rdp_gateway: gateway,
        rdp_shared_folder: sharefolder,
        rdp_dynamic_resolution: true,
        rdp_custom_resolution: String::new(),
        rdp_network_profile: crate::models::RdpNetworkProfile::Auto,
        rdp_disable_wallpaper: false,
        rdp_disable_themes: false,
        rdp_disable_animations: false,
        rdp_glyph_cache: glyph_cache,
        rdp_microphone: microphone,
        rdp_usb_redirect: false,
        rdp_smooth_fonts: true,
        rdp_desktop_composition: true,
        rdp_hw_accel: false,
        ssh_identity_file: ssh_key,
    };

    Ok(Connection {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        protocol,
        host,
        port,
        username,
        mac_address: String::new(),
        group,
        advanced_settings,
    })
}

/// Reads and imports a Remmina profile file at `path`.
pub fn import_remmina_file(path: &Path) -> Result<Connection, ImporterError> {
    let content = fs::read_to_string(path)?;
    let filename = path.file_name().and_then(|s| s.to_str());
    import_remmina_content(&content, filename)
}

/// Scans standard `~/.local/share/remmina` directory for `.remmina` profile files.
pub fn scan_remmina_profiles() -> Vec<(PathBuf, Result<Connection, ImporterError>)> {
    let mut results = Vec::new();
    let remmina_dir = match dirs::data_dir() {
        Some(d) => d.join("remmina"),
        None => return results,
    };

    if !remmina_dir.exists() || !remmina_dir.is_dir() {
        return results;
    }

    if let Ok(entries) = fs::read_dir(remmina_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && path
                    .extension()
                    .map(|ext| ext.eq_ignore_ascii_case("remmina"))
                    .unwrap_or(false)
            {
                let res = import_remmina_file(&path);
                results.push((path, res));
            }
        }
    }

    results
}

fn parse_host_port(server: &str, default_port: u16) -> (String, u16) {
    let trimmed = server.trim();
    if trimmed.is_empty() {
        return (String::new(), default_port);
    }

    if trimmed.starts_with('[') {
        if let Some(close_idx) = trimmed.find(']') {
            let host = trimmed[1..close_idx].to_string();
            let remainder = &trimmed[close_idx + 1..];
            if let Some(port_str) = remainder.strip_prefix(':') {
                if let Ok(port) = port_str.parse::<u16>() {
                    return (host, port);
                }
            }
            return (host, default_port);
        }
    }

    if let Some((h, p)) = trimmed.rsplit_once(':') {
        if !h.contains(':') {
            if let Ok(port) = p.parse::<u16>() {
                return (h.to_string(), port);
            }
        }
    }

    (trimmed.to_string(), default_port)
}
