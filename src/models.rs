use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Supported remote connection protocols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Rdp,
    Vnc,
    Ssh,
    Spice,
    Xrdp,
}

impl Default for Protocol {
    fn default() -> Self {
        Protocol::Rdp
    }
}

impl Protocol {
    pub fn default_port(&self) -> u16 {
        match self {
            Protocol::Rdp | Protocol::Xrdp => 3389,
            Protocol::Vnc => 5900,
            Protocol::Ssh => 22,
            Protocol::Spice => 5900,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Protocol::Rdp => "rdp",
            Protocol::Vnc => "vnc",
            Protocol::Ssh => "ssh",
            Protocol::Spice => "spice",
            Protocol::Xrdp => "xrdp",
        }
    }
}

impl std::fmt::Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// VNC display scaling modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RdpColorDepth {
    #[serde(rename = "Auto (Default)")]
    Automatic,
    #[serde(rename = "GFX AVC444 (32 bpp)")]
    GfxAvc444,
    #[serde(rename = "GFX AVC420 (32 bpp)")]
    GfxAvc420,
    #[serde(rename = "GFX RFX (32 bpp)")]
    GfxRfx,
    #[serde(rename = "GFX RFX Progressive (32 bpp)")]
    GfxRfxProgressive,
    #[serde(rename = "RemoteFX (32 bpp)")]
    RemoteFx,
    #[serde(rename = "True colour (32 bpp)")]
    TrueColor32,
    #[serde(rename = "True colour (24 bpp)")]
    TrueColor24,
    #[serde(rename = "High colour (16 bpp)")]
    HighColor16,
    #[serde(rename = "High colour (15 bpp)")]
    HighColor15,
    #[serde(rename = "256 colours (8 bpp)")]
    Colors256,
}

impl Default for RdpColorDepth {
    fn default() -> Self {
        RdpColorDepth::Automatic
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VncColorLevel {
    #[serde(rename = "Full Color (Default)")]
    Full,
    #[serde(rename = "Medium")]
    Medium,
    #[serde(rename = "Low")]
    Low,
    #[serde(rename = "Very Low")]
    VeryLow,
}

impl Default for VncColorLevel {
    fn default() -> Self {
        VncColorLevel::Full
    }
}

impl VncColorLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            VncColorLevel::Full => "Full Color (Default)",
            VncColorLevel::Medium => "Medium",
            VncColorLevel::Low => "Low",
            VncColorLevel::VeryLow => "Very Low",
        }
    }
}

impl std::fmt::Display for VncColorLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// VNC Encoding options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VncEncodingOption {
    Auto,
    Tight,
    Zrle,
    Raw,
}

impl Default for VncEncodingOption {
    fn default() -> Self {
        VncEncodingOption::Auto
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RdpNetworkProfile {
    Auto,
    Lan,
    Wan,
    Broadband,
    Modem,
}

impl Default for RdpNetworkProfile {
    fn default() -> Self {
        RdpNetworkProfile::Auto
    }
}

/// Advanced settings per connection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AdvancedSettings {
    pub rdp_multimon: bool,
    pub rdp_fullscreen: bool,
    pub rdp_audio: bool,
    pub vnc_viewonly: bool,
    pub vnc_shared: bool,
    #[serde(default)]
    pub vnc_fullscreen: bool,
    #[serde(default = "default_true")]
    pub vnc_clipboard: bool, // true by default in builder
    #[serde(default)]
    pub vnc_color_level: VncColorLevel,
    #[serde(default)]
    pub vnc_encoding: VncEncodingOption,
    #[serde(default)]
    pub vnc_compress_level: u8, // 0 for auto, 1-9 for force
    #[serde(default)]
    pub vnc_quality_level: u8, // 0 for auto, 1-9 for force
    pub clipboard_sharing: bool,
    pub color_depth: u8,
    #[serde(default)]
    pub rdp_color_depth: RdpColorDepth,
    pub spice_fullscreen: bool,
    pub spice_usb_redirect: bool,
    pub spice_scale_to_window: bool,
    // New RDP Settings
    pub rdp_domain: String,
    pub rdp_gateway: String,
    pub rdp_shared_folder: String,
    pub rdp_dynamic_resolution: bool,
    pub rdp_custom_resolution: String,
    pub rdp_network_profile: RdpNetworkProfile,
    pub rdp_disable_wallpaper: bool,
    pub rdp_disable_themes: bool,
    pub rdp_disable_animations: bool,
}

fn default_true() -> bool { true }

impl Default for AdvancedSettings {
    fn default() -> Self {
        Self {
            rdp_multimon: false,
            rdp_fullscreen: false,
            rdp_audio: false,
            vnc_viewonly: false,
            vnc_shared: false,
            clipboard_sharing: false,
            color_depth: 0,
            rdp_color_depth: RdpColorDepth::Automatic,
            vnc_fullscreen: false,
            vnc_clipboard: true,
            vnc_color_level: VncColorLevel::Full,
            vnc_encoding: VncEncodingOption::Auto,
            vnc_compress_level: 0,
            vnc_quality_level: 0,
            spice_fullscreen: false,
            spice_usb_redirect: false,
            spice_scale_to_window: false,
            rdp_domain: String::new(),
            rdp_gateway: String::new(),
            rdp_shared_folder: String::new(),
            rdp_dynamic_resolution: true,
            rdp_custom_resolution: String::new(),
            rdp_network_profile: RdpNetworkProfile::Auto,
            rdp_disable_wallpaper: false,
            rdp_disable_themes: false,
            rdp_disable_animations: false,
        }
    }
}

impl AdvancedSettings {
    pub fn sanitize(&mut self) -> bool {
        let mut modified = false;
        if !matches!(self.color_depth, 0 | 8 | 16 | 24 | 32) {
            self.color_depth = 0;
            modified = true;
        }
        modified
    }
}

fn default_id() -> String {
    Uuid::new_v4().to_string()
}

fn default_name() -> String {
    "New Connection".to_string()
}

fn default_port() -> u16 {
    3389
}

fn default_group() -> String {
    "Default".to_string()
}

/// Primary remote connection data model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Connection {
    #[serde(default = "default_id")]
    pub id: String,
    #[serde(default = "default_name")]
    pub name: String,
    #[serde(default)]
    pub protocol: Protocol,
    #[serde(default)]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub mac_address: String,
    #[serde(default = "default_group")]
    pub group: String,
    #[serde(default)]
    pub advanced_settings: AdvancedSettings,
}

impl Default for Connection {
    fn default() -> Self {
        Self {
            id: default_id(),
            name: default_name(),
            protocol: Protocol::default(),
            host: String::new(),
            port: default_port(),
            username: String::new(),
            mac_address: String::new(),
            group: default_group(),
            advanced_settings: AdvancedSettings::default(),
        }
    }
}

impl Connection {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_with_protocol(protocol: Protocol) -> Self {
        let port = protocol.default_port();
        Self {
            protocol,
            port,
            ..Default::default()
        }
    }

    /// Resolve port defaults when port is unset (0) or default.
    pub fn resolve_port(&self) -> u16 {
        if self.port != 0 {
            return self.port;
        }
        self.protocol.default_port()
    }

    /// Sanitize fields, ensuring valid UUIDs, non-empty name/group, valid port, and clean settings.
    pub fn sanitize(&mut self) -> bool {
        let mut modified = false;

        if self.id.trim().is_empty() || self.id.contains('/') || self.id.contains('\\') || self.id.contains("..") {
            self.id = Uuid::new_v4().to_string();
            modified = true;
        }

        if self.name.trim().is_empty() {
            self.name = "New Connection".to_string();
            modified = true;
        }

        if self.group.trim().is_empty() {
            self.group = "Default".to_string();
            modified = true;
        }

        if self.port == 0 {
            self.port = self.resolve_port();
            modified = true;
        }

        if self.advanced_settings.sanitize() {
            modified = true;
        }

        modified
    }

    /// Validate MAC address format for Wake-on-LAN.
    pub fn validate_mac(&self) -> Result<Option<String>, String> {
        let trimmed = self.mac_address.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }
        let clean: String = trimmed.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        if clean.len() == 12 {
            Ok(Some(clean.to_uppercase()))
        } else {
            Err(format!("Invalid MAC address format: '{}'", self.mac_address))
        }
    }
}

fn default_theme() -> String {
    "default".to_string()
}

/// Global application configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    #[serde(default = "default_theme")]
    pub theme: String,
    pub default_protocol: Protocol,
    pub auto_connect_last: bool,
    pub default_vnc_color_level: VncColorLevel,
    pub last_connected_id: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            default_protocol: Protocol::default(),
            auto_connect_last: false,
            default_vnc_color_level: VncColorLevel::default(),
            last_connected_id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_defaults() {
        let conn = Connection::default();
        assert!(!conn.id.is_empty());
        assert!(Uuid::parse_str(&conn.id).is_ok());
        assert_eq!(conn.name, "New Connection");
        assert_eq!(conn.protocol, Protocol::Rdp);
        assert_eq!(conn.port, 3389);
        assert_eq!(conn.group, "Default");
        assert_eq!(conn.advanced_settings.color_depth, 0);
        assert_eq!(conn.advanced_settings.clipboard_sharing, false);
        assert_eq!(conn.advanced_settings.vnc_scaling, VncScaling::OriginalSize);
    }

    #[test]
    fn test_deserialize_empty_json_object() {
        let json_data = "{}";
        let conn: Connection = serde_json::from_str(json_data).expect("Should deserialize empty JSON object into defaults");
        assert!(!conn.id.is_empty());
        assert_eq!(conn.name, "New Connection");
        assert_eq!(conn.protocol, Protocol::Rdp);
        assert_eq!(conn.port, 3389);
        assert_eq!(conn.group, "Default");
    }

    #[test]
    fn test_deserialize_minimal_partial_json() {
        let json_data = r#"{
            "id": "11111111-2222-3333-4444-555555555555",
            "name": "Production Server",
            "protocol": "vnc",
            "host": "10.0.0.50"
        }"#;
        let conn: Connection = serde_json::from_str(json_data).expect("Should deserialize partial JSON");
        assert_eq!(conn.id, "11111111-2222-3333-4444-555555555555");
        assert_eq!(conn.name, "Production Server");
        assert_eq!(conn.protocol, Protocol::Vnc);
        assert_eq!(conn.host, "10.0.0.50");
        assert_eq!(conn.port, 3389);
        assert_eq!(conn.group, "Default");
    }

    #[test]
    fn test_deserialize_unknown_json_fields() {
        let json_data = r#"{
            "id": "11111111-2222-3333-4444-555555555555",
            "name": "Legacy Conn",
            "unknown_legacy_field_1": 12345,
            "deprecated_flag": true
        }"#;
        let conn: Connection = serde_json::from_str(json_data).expect("Should ignore unknown JSON fields cleanly");
        assert_eq!(conn.id, "11111111-2222-3333-4444-555555555555");
        assert_eq!(conn.name, "Legacy Conn");
    }

    #[test]
    fn test_protocol_enum_serde_representations() {
        assert_eq!(serde_json::to_string(&Protocol::Rdp).unwrap(), r#""rdp""#);
        assert_eq!(serde_json::to_string(&Protocol::Vnc).unwrap(), r#""vnc""#);
        assert_eq!(serde_json::to_string(&Protocol::Ssh).unwrap(), r#""ssh""#);

        let p_rdp: Protocol = serde_json::from_str(r#""rdp""#).unwrap();
        let p_vnc: Protocol = serde_json::from_str(r#""vnc""#).unwrap();
        let p_ssh: Protocol = serde_json::from_str(r#""ssh""#).unwrap();

        assert_eq!(p_rdp, Protocol::Rdp);
        assert_eq!(p_vnc, Protocol::Vnc);
        assert_eq!(p_ssh, Protocol::Ssh);
    }

    #[test]
    fn test_vnc_scaling_enum_serde_representations() {
        assert_eq!(serde_json::to_string(&VncScaling::OriginalSize).unwrap(), r#""Original Size""#);
        assert_eq!(serde_json::to_string(&VncScaling::FitToWindow).unwrap(), r#""Fit to Window""#);
        assert_eq!(serde_json::to_string(&VncScaling::Stretch).unwrap(), r#""Stretch""#);

        let s_orig: VncScaling = serde_json::from_str(r#""Original Size""#).unwrap();
        let s_fit: VncScaling = serde_json::from_str(r#""Fit to Window""#).unwrap();
        let s_stretch: VncScaling = serde_json::from_str(r#""Stretch""#).unwrap();

        assert_eq!(s_orig, VncScaling::OriginalSize);
        assert_eq!(s_fit, VncScaling::FitToWindow);
        assert_eq!(s_stretch, VncScaling::Stretch);
    }

    #[test]
    fn test_password_isolation_in_json_schema() {
        let conn = Connection::default();
        let serialized = serde_json::to_string(&conn).expect("Serialization must succeed");
        assert!(!serialized.contains("password"));
        assert!(!serialized.contains("secret"));
    }

    #[test]
    fn test_connection_sanitize() {
        let mut conn = Connection {
            id: "".to_string(),
            name: "   ".to_string(),
            group: "".to_string(),
            port: 0,
            protocol: Protocol::Vnc,
            advanced_settings: AdvancedSettings {
                color_depth: 99,
                ..Default::default()
            },
            ..Default::default()
        };

        assert!(conn.sanitize());
        assert!(Uuid::parse_str(&conn.id).is_ok());
        assert_eq!(conn.name, "New Connection");
        assert_eq!(conn.group, "Default");
        assert_eq!(conn.port, 5900);
        assert_eq!(conn.advanced_settings.color_depth, 0);

        let mut conn_traversal = Connection {
            id: "../etc/passwd".to_string(),
            ..Default::default()
        };
        assert!(conn_traversal.sanitize());
        assert!(Uuid::parse_str(&conn_traversal.id).is_ok());
    }

    #[test]
    fn test_connection_resolve_port() {
        let mut conn = Connection::default();
        conn.port = 0;

        conn.protocol = Protocol::Rdp;
        assert_eq!(conn.resolve_port(), 3389);

        conn.protocol = Protocol::Vnc;
        assert_eq!(conn.resolve_port(), 5900);

        conn.protocol = Protocol::Ssh;
        assert_eq!(conn.resolve_port(), 22);

        conn.port = 8080;
        assert_eq!(conn.resolve_port(), 8080);
    }

    #[test]
    fn test_validate_mac() {
        let mut conn = Connection::default();
        conn.mac_address = "".to_string();
        assert_eq!(conn.validate_mac(), Ok(None));

        conn.mac_address = "00:11:22:33:44:55".to_string();
        assert_eq!(conn.validate_mac(), Ok(Some("001122334455".to_string())));

        conn.mac_address = "00-11-22-33-44-55".to_string();
        assert_eq!(conn.validate_mac(), Ok(Some("001122334455".to_string())));

        conn.mac_address = "invalid-mac".to_string();
        assert!(conn.validate_mac().is_err());
    }
}
