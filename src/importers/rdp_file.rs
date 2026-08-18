use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::importers::{parse_host_port, ImporterError};
use crate::models::{
    AdvancedSettings, Connection, Protocol, RdpCertHandling, RdpColorDepth, RdpSecurityProtocol,
};

/// Parses an `.rdp` file content into a VER `Connection`.
pub fn import_rdp_content(
    content: &str,
    file_name: Option<&str>,
) -> Result<Connection, ImporterError> {
    let mut string_props: HashMap<String, String> = HashMap::new();
    let mut int_props: HashMap<String, i64> = HashMap::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Format: key:type:value
        let parts: Vec<&str> = trimmed.splitn(3, ':').collect();
        if parts.len() < 3 {
            continue;
        }

        let key = parts[0].trim().to_lowercase();
        let type_tag = parts[1].trim().to_lowercase();
        let val = parts[2].trim();

        match type_tag.as_str() {
            "s" => {
                string_props.insert(key, val.to_string());
            }
            "i" => {
                if let Ok(num) = val.parse::<i64>() {
                    int_props.insert(key, num);
                }
            }
            _ => {
                string_props.insert(key, val.to_string());
            }
        }
    }

    if string_props.is_empty() && int_props.is_empty() {
        return Err(ImporterError::InvalidFormat(
            "RDP file contains no recognizable properties".to_string(),
        ));
    }

    let raw_address = string_props
        .get("full address")
        .cloned()
        .unwrap_or_default();

    let (host, port) = parse_host_port(&raw_address, 3389);

    let fallback_name = file_name
        .and_then(|f| Path::new(f).file_stem())
        .and_then(|s| s.to_str())
        .unwrap_or(if host.is_empty() {
            "Imported RDP"
        } else {
            &host
        });

    let username = string_props.get("username").cloned().unwrap_or_default();
    let domain = string_props.get("domain").cloned().unwrap_or_default();
    let gateway = string_props
        .get("gatewayhostname")
        .cloned()
        .unwrap_or_default();
    let shared_folder = string_props
        .get("drivestoredirect")
        .cloned()
        .unwrap_or_default();

    let screen_mode = int_props.get("screen mode id").copied().unwrap_or(1);
    let fullscreen = screen_mode == 2;
    let multimon = int_props.get("use multimon").copied().unwrap_or(0) == 1;
    let clipboard = int_props.get("redirectclipboard").copied().unwrap_or(1) == 1;
    let audiomode = int_props.get("audiomode").copied().unwrap_or(0);
    let audio = audiomode == 0;

    let bpp = int_props.get("session bpp").copied().unwrap_or(32);
    let rdp_color_depth = match bpp {
        8 => RdpColorDepth::Colors256,
        15 => RdpColorDepth::HighColor15,
        16 => RdpColorDepth::HighColor16,
        24 => RdpColorDepth::TrueColor24,
        _ => RdpColorDepth::TrueColor32,
    };

    let auth_level = int_props.get("authentication level").copied().unwrap_or(2);
    let rdp_cert_handling = match auth_level {
        0 => RdpCertHandling::Ignore,
        1 => RdpCertHandling::Deny,
        _ => RdpCertHandling::Tofu,
    };

    let width = int_props.get("desktopwidth").copied().unwrap_or(0);
    let height = int_props.get("desktopheight").copied().unwrap_or(0);
    let custom_res = if width > 0 && height > 0 {
        format!("{}x{}", width, height)
    } else {
        String::new()
    };

    let advanced_settings = AdvancedSettings {
        rdp_multimon: multimon,
        rdp_fullscreen: fullscreen,
        rdp_audio: audio,
        vnc_viewonly: false,
        vnc_shared: false,
        vnc_fullscreen: fullscreen,
        vnc_clipboard: clipboard,
        vnc_color_level: crate::models::VncColorLevel::Full,
        vnc_encoding: crate::models::VncEncodingOption::Auto,
        vnc_compress_level: 0,
        vnc_quality_level: 0,
        clipboard_sharing: clipboard,
        color_depth: 32,
        rdp_color_depth,
        rdp_cert_handling,
        rdp_security: RdpSecurityProtocol::Auto,
        spice_fullscreen: fullscreen,
        spice_usb_redirect: false,
        spice_scale_to_window: true,
        rdp_domain: domain,
        rdp_gateway: gateway,
        rdp_shared_folder: shared_folder,
        rdp_dynamic_resolution: custom_res.is_empty(),
        rdp_custom_resolution: custom_res,
        rdp_network_profile: crate::models::RdpNetworkProfile::Auto,
        rdp_disable_wallpaper: false,
        rdp_disable_themes: false,
        rdp_disable_animations: false,
        rdp_glyph_cache: true,
        rdp_microphone: false,
        rdp_usb_redirect: false,
        rdp_smooth_fonts: true,
        rdp_desktop_composition: true,
        rdp_hw_accel: false,
        ssh_identity_file: String::new(),
    };

    Ok(Connection {
        id: uuid::Uuid::new_v4().to_string(),
        name: fallback_name.to_string(),
        protocol: Protocol::Rdp,
        host,
        port,
        username,
        mac_address: String::new(),
        group: "RDP Files".to_string(),
        advanced_settings,
    })
}

/// Reads and imports an `.rdp` file at `path`, decoding UTF-8 or UTF-16 LE/BE as necessary.
pub fn import_rdp_file(path: &Path) -> Result<Connection, ImporterError> {
    let bytes = fs::read(path)?;
    let content = decode_rdp_bytes(&bytes)?;
    let filename = path.file_name().and_then(|s| s.to_str());
    import_rdp_content(&content, filename)
}

fn decode_rdp_bytes(bytes: &[u8]) -> Result<String, ImporterError> {
    if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
        // UTF-16LE with BOM
        let u16_slice: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();
        return char::decode_utf16(u16_slice)
            .collect::<Result<String, _>>()
            .map_err(|e| {
                ImporterError::InvalidFormat(format!("Invalid UTF-16LE encoding: {}", e))
            });
    } else if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        // UTF-16BE with BOM
        let u16_slice: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
            .collect();
        return char::decode_utf16(u16_slice)
            .collect::<Result<String, _>>()
            .map_err(|e| {
                ImporterError::InvalidFormat(format!("Invalid UTF-16BE encoding: {}", e))
            });
    } else if bytes.len() >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF {
        // UTF-8 with BOM
        return String::from_utf8(bytes[3..].to_vec())
            .map_err(|e| ImporterError::InvalidFormat(format!("Invalid UTF-8 encoding: {}", e)));
    }

    // Try standard UTF-8, fall back to lossy conversion if needed
    match String::from_utf8(bytes.to_vec()) {
        Ok(s) => Ok(s),
        Err(_) => {
            // Check for UTF-16LE without BOM (common when alternating null bytes exist)
            if bytes.len() >= 4 && bytes.len().is_multiple_of(2) && (bytes[1] == 0 || bytes[3] == 0)
            {
                let u16_slice: Vec<u16> = bytes
                    .chunks_exact(2)
                    .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                    .collect();
                if let Ok(s) = char::decode_utf16(u16_slice).collect::<Result<String, _>>() {
                    return Ok(s);
                }
            }
            Ok(String::from_utf8_lossy(bytes).into_owned())
        }
    }
}

/// Exports a VER `Connection` as a standard `.rdp` file string.
pub fn export_rdp_file(conn: &Connection) -> String {
    let screen_mode = if conn.advanced_settings.rdp_fullscreen {
        2
    } else {
        1
    };
    let multimon = if conn.advanced_settings.rdp_multimon {
        1
    } else {
        0
    };
    let clipboard = if conn.advanced_settings.clipboard_sharing {
        1
    } else {
        0
    };
    let audio = if conn.advanced_settings.rdp_audio {
        0
    } else {
        2
    };

    let bpp = match conn.advanced_settings.rdp_color_depth {
        RdpColorDepth::Colors256 => 8,
        RdpColorDepth::HighColor15 => 15,
        RdpColorDepth::HighColor16 => 16,
        RdpColorDepth::TrueColor24 => 24,
        _ => 32,
    };

    let auth_level = match conn.advanced_settings.rdp_cert_handling {
        RdpCertHandling::Ignore => 0,
        RdpCertHandling::Deny => 1,
        _ => 2,
    };

    let mut lines = Vec::new();
    lines.push(format!("full address:s:{}:{}", conn.host, conn.port));
    if !conn.username.is_empty() {
        lines.push(format!("username:s:{}", conn.username));
    }
    if !conn.advanced_settings.rdp_domain.is_empty() {
        lines.push(format!("domain:s:{}", conn.advanced_settings.rdp_domain));
    }
    if !conn.advanced_settings.rdp_gateway.is_empty() {
        lines.push(format!(
            "gatewayhostname:s:{}",
            conn.advanced_settings.rdp_gateway
        ));
    }
    if !conn.advanced_settings.rdp_shared_folder.is_empty() {
        lines.push(format!(
            "drivestoredirect:s:{}",
            conn.advanced_settings.rdp_shared_folder
        ));
    }

    lines.push(format!("screen mode id:i:{}", screen_mode));
    lines.push(format!("use multimon:i:{}", multimon));
    lines.push(format!("redirectclipboard:i:{}", clipboard));
    lines.push(format!("audiomode:i:{}", audio));
    lines.push(format!("session bpp:i:{}", bpp));
    lines.push(format!("authentication level:i:{}", auth_level));
    lines.push("prompt for credentials:i:0".to_string());
    lines.push("negotiate security layer:i:1".to_string());

    if !conn.advanced_settings.rdp_custom_resolution.is_empty() {
        if let Some((w, h)) = conn.advanced_settings.rdp_custom_resolution.split_once('x') {
            if let (Ok(width), Ok(height)) = (w.trim().parse::<u32>(), h.trim().parse::<u32>()) {
                lines.push(format!("desktopwidth:i:{}", width));
                lines.push(format!("desktopheight:i:{}", height));
            }
        }
    }

    lines.join("\r\n")
}
