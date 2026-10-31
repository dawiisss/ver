use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Supported remote connection protocols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    #[default]
    Rdp,
    Vnc,
    Ssh,
    Spice,
    Xrdp,
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

/// RDP color depth and graphics pipeline modes for FreeRDP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum RdpColorDepth {
    #[serde(rename = "Auto (Default)")]
    #[default]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum VncColorLevel {
    #[serde(rename = "Full Color (Default)")]
    #[default]
    Full,
    #[serde(rename = "Medium")]
    Medium,
    #[serde(rename = "Low")]
    Low,
    #[serde(rename = "Very Low")]
    VeryLow,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum VncEncodingOption {
    #[default]
    Auto,
    Tight,
    Zrle,
    Raw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RdpNetworkProfile {
    #[default]
    Auto,
    Lan,
    Wan,
    Broadband,
    Modem,
}

/// RDP TLS certificate verification policies for FreeRDP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RdpCertHandling {
    Ignore,
    #[default]
    Tofu,
    Deny,
    Ask,
}

impl RdpCertHandling {
    pub fn as_str(&self) -> &'static str {
        match self {
            RdpCertHandling::Ignore => "ignore",
            RdpCertHandling::Tofu => "tofu",
            RdpCertHandling::Deny => "deny",
            RdpCertHandling::Ask => "ask",
        }
    }
}

impl std::fmt::Display for RdpCertHandling {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// RDP Security Protocol negotiation options for FreeRDP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RdpSecurityProtocol {
    #[default]
    Auto,
    Nla,
    Tls,
    Rdp,
    Ext,
}

impl RdpSecurityProtocol {
    pub fn as_str(&self) -> &'static str {
        match self {
            RdpSecurityProtocol::Auto => "auto",
            RdpSecurityProtocol::Nla => "nla",
            RdpSecurityProtocol::Tls => "tls",
            RdpSecurityProtocol::Rdp => "rdp",
            RdpSecurityProtocol::Ext => "ext",
        }
    }
}

impl std::fmt::Display for RdpSecurityProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
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
    /// Legacy field kept for backward compatibility with older configuration schemas.
    #[serde(default)]
    pub color_depth: u8,
    #[serde(default)]
    pub rdp_color_depth: RdpColorDepth,
    #[serde(default)]
    pub rdp_cert_handling: RdpCertHandling,
    #[serde(default)]
    pub rdp_security: RdpSecurityProtocol,
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
    #[serde(default = "default_true")]
    pub rdp_glyph_cache: bool,
    #[serde(default)]
    pub rdp_microphone: bool,
    #[serde(default)]
    pub rdp_usb_redirect: bool,
    #[serde(default = "default_true")]
    pub rdp_smooth_fonts: bool,
    #[serde(default = "default_true")]
    pub rdp_desktop_composition: bool,
    #[serde(default)]
    pub rdp_hw_accel: bool,
    #[serde(default)]
    pub ssh_identity_file: String,
    /// `ssh -J` jump hosts for SSH connections: `[user@]host[:port]`, comma separated.
    #[serde(default)]
    pub ssh_jump_host: String,
    /// Port forwards opened with SSH connections.
    #[serde(default)]
    pub ssh_forwards: Vec<SshForward>,
    /// SSH server (`[user@]host[:port]`) that RDP, VNC and SPICE connections are tunnelled
    /// through. Empty connects directly.
    #[serde(default)]
    pub ssh_tunnel_host: String,
    #[serde(default)]
    pub ssh_tunnel_identity_file: String,
}
fn default_true() -> bool {
    true
}

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
            rdp_cert_handling: RdpCertHandling::Tofu,
            rdp_security: RdpSecurityProtocol::Auto,
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
            rdp_glyph_cache: true,
            rdp_microphone: false,
            rdp_usb_redirect: false,
            rdp_smooth_fonts: true,
            rdp_desktop_composition: true,
            rdp_hw_accel: false,
            ssh_identity_file: String::new(),
            ssh_jump_host: String::new(),
            ssh_forwards: Vec::new(),
            ssh_tunnel_host: String::new(),
            ssh_tunnel_identity_file: String::new(),
        }
    }
}

/// Direction of an SSH port forward, matching `ssh -L`, `-R` and `-D`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SshForwardKind {
    #[default]
    Local,
    Remote,
    Dynamic,
}

impl SshForwardKind {
    pub fn flag(self) -> &'static str {
        match self {
            SshForwardKind::Local => "-L",
            SshForwardKind::Remote => "-R",
            SshForwardKind::Dynamic => "-D",
        }
    }
}

/// An SSH port forward. Dynamic (SOCKS) forwards have no destination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SshForward {
    pub kind: SshForwardKind,
    /// Listen address; empty uses ssh's default.
    pub bind_address: String,
    pub listen_port: u16,
    pub dest_host: String,
    pub dest_port: u16,
}

/// Split on ':' outside square brackets, so IPv6 addresses stay whole.
fn split_colons(spec: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    for (i, c) in spec.char_indices() {
        match c {
            '[' => depth += 1,
            ']' => depth -= 1,
            ':' if depth == 0 => {
                parts.push(&spec[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&spec[start..]);
    parts
}

/// Bracket IPv6 literals for `ssh` forward specifications.
fn bracket_ipv6(host: &str) -> String {
    if host.contains(':') && !host.starts_with('[') {
        format!("[{}]", host)
    } else {
        host.to_string()
    }
}

fn parse_port(value: &str) -> Result<u16, String> {
    match value.trim().parse::<u16>() {
        Ok(port) if port != 0 => Ok(port),
        _ => Err(format!("'{}' is not a port between 1 and 65535", value)),
    }
}

/// Check a host name used in an ssh argument, which must not be read as an option.
fn validate_ssh_host(host: &str, what: &str) -> Result<(), String> {
    if host.is_empty() {
        return Err(format!("{} cannot be empty", what));
    }
    if host.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(format!("{} cannot contain whitespace", what));
    }
    if host.starts_with('-') {
        return Err(format!("{} cannot start with '-'", what));
    }
    Ok(())
}

impl SshForward {
    /// Parse `L|R [bind:]port:host:hostport` or `D [bind:]port`, as written for `ssh -L/-R/-D`.
    pub fn parse(input: &str) -> Result<Self, String> {
        let input = input.trim();
        let (kind, spec) = input.split_once(char::is_whitespace).ok_or(
            "Write a forward as 'L 8080:localhost:80', 'R 9000:localhost:3000' or 'D 1080'",
        )?;
        let kind = match kind.trim_start_matches('-').to_ascii_lowercase().as_str() {
            "l" | "local" => SshForwardKind::Local,
            "r" | "remote" => SshForwardKind::Remote,
            "d" | "dynamic" | "socks" => SshForwardKind::Dynamic,
            other => return Err(format!("Unknown forward type '{}'; use L, R or D", other)),
        };
        Self::parse_spec(kind, spec.trim())
    }

    /// Parse the part after the flag: `[bind:]port:host:hostport`, or `[bind:]port` for `D`.
    pub fn parse_spec(kind: SshForwardKind, spec: &str) -> Result<Self, String> {
        let parts = split_colons(spec);
        let unbracket = |s: &str| s.trim_matches(['[', ']']).to_string();
        let forward = match (kind, parts.as_slice()) {
            (SshForwardKind::Dynamic, [port]) => Self {
                kind,
                listen_port: parse_port(port)?,
                ..Default::default()
            },
            (SshForwardKind::Dynamic, [bind, port]) => Self {
                kind,
                bind_address: unbracket(bind),
                listen_port: parse_port(port)?,
                ..Default::default()
            },
            (SshForwardKind::Dynamic, _) => {
                return Err("A dynamic forward is '[bind_address:]port'".into())
            }
            (_, [port, host, host_port]) => Self {
                kind,
                listen_port: parse_port(port)?,
                dest_host: unbracket(host),
                dest_port: parse_port(host_port)?,
                ..Default::default()
            },
            (_, [bind, port, host, host_port]) => Self {
                kind,
                bind_address: unbracket(bind),
                listen_port: parse_port(port)?,
                dest_host: unbracket(host),
                dest_port: parse_port(host_port)?,
            },
            _ => return Err("A forward is '[bind_address:]port:host:hostport'".into()),
        };
        forward.validate()?;
        Ok(forward)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.listen_port == 0 {
            return Err("Forward listen port must be between 1 and 65535".into());
        }
        if !self.bind_address.is_empty() {
            validate_ssh_host(&self.bind_address, "Forward bind address")?;
        }
        if self.kind != SshForwardKind::Dynamic {
            validate_ssh_host(&self.dest_host, "Forward destination host")?;
            if self.dest_port == 0 {
                return Err("Forward destination port must be between 1 and 65535".into());
            }
        }
        Ok(())
    }

    /// The argument that follows `flag()` on the `ssh` command line.
    pub fn spec(&self) -> String {
        let mut spec = if self.bind_address.is_empty() {
            self.listen_port.to_string()
        } else {
            format!("{}:{}", bracket_ipv6(&self.bind_address), self.listen_port)
        };
        if self.kind != SshForwardKind::Dynamic {
            spec = format!(
                "{}:{}:{}",
                spec,
                bracket_ipv6(&self.dest_host),
                self.dest_port
            );
        }
        spec
    }

    /// Short description for the connection editor.
    pub fn describe(&self) -> String {
        let listen = if self.bind_address.is_empty() {
            self.listen_port.to_string()
        } else {
            format_host_port(&self.bind_address, self.listen_port)
        };
        match self.kind {
            SshForwardKind::Local => format!(
                "Local {} → {}",
                listen,
                format_host_port(&self.dest_host, self.dest_port)
            ),
            SshForwardKind::Remote => format!(
                "Remote {} → {}",
                listen,
                format_host_port(&self.dest_host, self.dest_port)
            ),
            SshForwardKind::Dynamic => format!("SOCKS proxy on {}", listen),
        }
    }
}

/// An SSH destination written as `[user@]host[:port]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshDestination {
    pub user: Option<String>,
    pub host: String,
    pub port: Option<u16>,
}

impl SshDestination {
    pub fn parse(input: &str) -> Result<Self, String> {
        let input = input.trim();
        let (user, host_port) = match input.rsplit_once('@') {
            Some((user, rest)) if !user.is_empty() => (Some(user.to_string()), rest),
            Some(_) => return Err(format!("'{}' has an empty user name", input)),
            None => (None, input),
        };
        let (host, port) = match split_colons(host_port).as_slice() {
            [host] => (host.to_string(), None),
            [host, port] => (host.to_string(), Some(parse_port(port)?)),
            // An unbracketed IPv6 address without a port.
            _ if !host_port.starts_with('[') => (host_port.to_string(), None),
            _ => return Err(format!("'{}' is not a valid host and port", host_port)),
        };
        let host = host.trim_matches(['[', ']']).to_string();
        validate_ssh_host(&host, "SSH host")?;
        if let Some(user) = &user {
            validate_ssh_host(user, "SSH user name")?;
        }
        Ok(Self { user, host, port })
    }

    /// `user@host`, as given to `ssh` after `--`.
    pub fn target(&self) -> String {
        match &self.user {
            Some(user) => format!("{}@{}", user, self.host),
            None => self.host.clone(),
        }
    }
}

/// Validate a comma-separated `ssh -J` list.
pub fn validate_jump_hosts(jumps: &str) -> Result<(), String> {
    for hop in jumps.split(',') {
        SshDestination::parse(hop).map_err(|e| format!("Jump host: {}", e))?;
    }
    Ok(())
}

impl AdvancedSettings {
    pub fn sanitize(&mut self) -> bool {
        let mut modified = false;
        if !matches!(self.color_depth, 0 | 8 | 16 | 24 | 32) {
            self.color_depth = 0;
            modified = true;
        }
        for level in [&mut self.vnc_compress_level, &mut self.vnc_quality_level] {
            if *level > 9 {
                *level = 0;
                modified = true;
            }
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
#[serde(from = "ConnectionRecord")]
pub struct Connection {
    pub id: String,
    pub name: String,
    pub protocol: Protocol,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub mac_address: String,
    pub group: String,
    /// Pinned connections are listed first in the sidebar and in the tray menu.
    pub pinned: bool,
    pub advanced_settings: AdvancedSettings,
}

/// Serialized form of `Connection`, so a missing port can default from the protocol.
#[derive(Deserialize)]
struct ConnectionRecord {
    #[serde(default = "default_id")]
    id: String,
    #[serde(default = "default_name")]
    name: String,
    #[serde(default)]
    protocol: Protocol,
    #[serde(default)]
    host: String,
    #[serde(default)]
    port: Option<u16>,
    #[serde(default)]
    username: String,
    #[serde(default)]
    mac_address: String,
    #[serde(default = "default_group")]
    group: String,
    #[serde(default)]
    pinned: bool,
    #[serde(default)]
    advanced_settings: AdvancedSettings,
}

impl From<ConnectionRecord> for Connection {
    fn from(record: ConnectionRecord) -> Self {
        Self {
            port: record
                .port
                .unwrap_or_else(|| record.protocol.default_port()),
            id: record.id,
            name: record.name,
            protocol: record.protocol,
            host: record.host,
            username: record.username,
            mac_address: record.mac_address,
            group: record.group,
            pinned: record.pinned,
            advanced_settings: record.advanced_settings,
        }
    }
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
            pinned: false,
            advanced_settings: AdvancedSettings::default(),
        }
    }
}

impl Connection {
    /// Validate a configured endpoint. Draft connections may be saved before this succeeds.
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("Connection name cannot be empty".into());
        }
        let host = self.host.trim();
        if host.is_empty() || host.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Err("Host must be a hostname or IP address without whitespace".into());
        }
        if self.port == 0 {
            return Err("Port must be a valid number between 1 and 65535".into());
        }
        self.validate_fields()
    }

    /// Validate persisted fields while permitting an unfinished connection with no host.
    pub fn validate_fields(&self) -> Result<(), String> {
        if self
            .host
            .trim()
            .chars()
            .any(|c| c.is_whitespace() || c.is_control())
        {
            return Err("Host must be a hostname or IP address without whitespace".into());
        }
        if self.host.trim().starts_with('-') {
            return Err("Host cannot start with '-'".into());
        }
        if self.advanced_settings.vnc_compress_level > 9
            || self.advanced_settings.vnc_quality_level > 9
        {
            return Err("VNC compression and quality must be between 0 and 9".into());
        }
        self.validate_mac()?;
        let adv = &self.advanced_settings;
        if !adv.ssh_jump_host.trim().is_empty() {
            validate_jump_hosts(adv.ssh_jump_host.trim())?;
        }
        if !adv.ssh_tunnel_host.trim().is_empty() {
            SshDestination::parse(&adv.ssh_tunnel_host)
                .map_err(|e| format!("SSH tunnel: {}", e))?;
        }
        for forward in &adv.ssh_forwards {
            forward.validate()?;
        }
        Ok(())
    }

    /// Whether this graphical connection is reached through an SSH tunnel.
    pub fn uses_ssh_tunnel(&self) -> bool {
        self.protocol != Protocol::Ssh && !self.advanced_settings.ssh_tunnel_host.trim().is_empty()
    }

    /// Host and port to probe for reachability: the first SSH hop when one is configured,
    /// since the target itself is often unreachable from this machine.
    pub fn probe_target(&self) -> (String, u16) {
        let adv = &self.advanced_settings;
        let first_hop = if self.uses_ssh_tunnel() {
            Some(adv.ssh_tunnel_host.as_str())
        } else if self.protocol == Protocol::Ssh && !adv.ssh_jump_host.trim().is_empty() {
            adv.ssh_jump_host.split(',').next()
        } else {
            None
        };
        match first_hop.and_then(|hop| SshDestination::parse(hop).ok()) {
            Some(dest) => (dest.host, dest.port.unwrap_or(22)),
            None => (self.host.clone(), self.resolve_port()),
        }
    }

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

        if self.id.trim().is_empty()
            || self.id.contains('/')
            || self.id.contains('\\')
            || self.id.contains("..")
        {
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
        let mac_bytes = crate::network::parse_mac_address(trimmed)?;
        let formatted: String = mac_bytes.iter().map(|b| format!("{:02X}", b)).collect();
        Ok(Some(formatted))
    }
}

/// Preserve legacy IDs (and their keyring links), replacing only unsafe or duplicate IDs.
pub fn sanitize_connections(connections: &mut [Connection]) {
    use std::collections::HashSet;
    let mut reserved: HashSet<String> = connections.iter().map(|c| c.id.clone()).collect();
    let mut seen = HashSet::new();
    for conn in connections {
        conn.sanitize();
        if seen.contains(&conn.id) {
            loop {
                conn.id = Uuid::new_v4().to_string();
                if !reserved.contains(&conn.id) {
                    break;
                }
            }
        }
        reserved.insert(conn.id.clone());
        seen.insert(conn.id.clone());
    }
}

/// Format a host and port string, enclosing IPv6 literals in brackets `[...]`.
pub fn format_host_port(host: &str, port: u16) -> String {
    let trimmed = host.trim();
    if trimmed.contains(':') && !trimmed.starts_with('[') {
        format!("[{}]:{}", trimmed, port)
    } else {
        format!("{}:{}", trimmed, port)
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
    /// Most recently launched connection IDs, newest first.
    pub recent_connection_ids: Vec<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            default_protocol: Protocol::default(),
            auto_connect_last: false,
            default_vnc_color_level: VncColorLevel::default(),
            last_connected_id: None,
            recent_connection_ids: Vec::new(),
        }
    }
}

/// Number of connections kept in the recent list.
pub const MAX_RECENT_CONNECTIONS: usize = 5;

impl AppConfig {
    /// Record a launch as the last connection and move it to the front of the recent list.
    pub fn record_recent(&mut self, id: &str) {
        self.last_connected_id = Some(id.to_string());
        self.recent_connection_ids.retain(|recent| recent != id);
        self.recent_connection_ids.insert(0, id.to_string());
        self.recent_connection_ids.truncate(MAX_RECENT_CONNECTIONS);
    }
}

/// Pinned connections ordered by name.
pub fn pinned_connections(connections: &[Connection]) -> Vec<&Connection> {
    let mut pinned: Vec<&Connection> = connections.iter().filter(|c| c.pinned).collect();
    pinned.sort_by_cached_key(|c| c.name.to_lowercase());
    pinned
}

/// Recently launched connections, newest first, skipping IDs that no longer exist.
pub fn recent_connections<'a>(
    config: &AppConfig,
    connections: &'a [Connection],
) -> Vec<&'a Connection> {
    config
        .recent_connection_ids
        .iter()
        .filter_map(|id| connections.iter().find(|c| &c.id == id))
        .collect()
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
        assert!(!conn.advanced_settings.clipboard_sharing);
        assert!(conn.advanced_settings.vnc_clipboard);
        assert_eq!(conn.advanced_settings.vnc_color_level, VncColorLevel::Full);
    }

    #[test]
    fn test_deserialize_empty_json_object() {
        let json_data = "{}";
        let conn: Connection = serde_json::from_str(json_data)
            .expect("Should deserialize empty JSON object into defaults");
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
        let conn: Connection =
            serde_json::from_str(json_data).expect("Should deserialize partial JSON");
        assert_eq!(conn.id, "11111111-2222-3333-4444-555555555555");
        assert_eq!(conn.name, "Production Server");
        assert_eq!(conn.protocol, Protocol::Vnc);
        assert_eq!(conn.host, "10.0.0.50");
        assert_eq!(conn.port, 5900);
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
        let conn: Connection =
            serde_json::from_str(json_data).expect("Should ignore unknown JSON fields cleanly");
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
    fn test_vnc_color_level_enum_serde_representations() {
        assert_eq!(
            serde_json::to_string(&VncColorLevel::Full).unwrap(),
            r#""Full Color (Default)""#
        );
        assert_eq!(
            serde_json::to_string(&VncColorLevel::Medium).unwrap(),
            r#""Medium""#
        );
        assert_eq!(
            serde_json::to_string(&VncColorLevel::Low).unwrap(),
            r#""Low""#
        );
        assert_eq!(
            serde_json::to_string(&VncColorLevel::VeryLow).unwrap(),
            r#""Very Low""#
        );

        let c_full: VncColorLevel = serde_json::from_str(r#""Full Color (Default)""#).unwrap();
        let c_med: VncColorLevel = serde_json::from_str(r#""Medium""#).unwrap();
        assert_eq!(c_full, VncColorLevel::Full);
        assert_eq!(c_med, VncColorLevel::Medium);
    }

    #[test]
    fn test_rdp_cert_handling_serde_representations() {
        assert_eq!(
            serde_json::to_string(&RdpCertHandling::Ignore).unwrap(),
            r#""ignore""#
        );
        assert_eq!(
            serde_json::to_string(&RdpCertHandling::Tofu).unwrap(),
            r#""tofu""#
        );
        assert_eq!(
            serde_json::to_string(&RdpCertHandling::Deny).unwrap(),
            r#""deny""#
        );
        assert_eq!(
            serde_json::to_string(&RdpCertHandling::Ask).unwrap(),
            r#""ask""#
        );

        let h_ignore: RdpCertHandling = serde_json::from_str(r#""ignore""#).unwrap();
        let h_tofu: RdpCertHandling = serde_json::from_str(r#""tofu""#).unwrap();
        let h_deny: RdpCertHandling = serde_json::from_str(r#""deny""#).unwrap();
        let h_ask: RdpCertHandling = serde_json::from_str(r#""ask""#).unwrap();

        assert_eq!(h_ignore, RdpCertHandling::Ignore);
        assert_eq!(h_tofu, RdpCertHandling::Tofu);
        assert_eq!(h_deny, RdpCertHandling::Deny);
        assert_eq!(h_ask, RdpCertHandling::Ask);
    }

    #[test]
    fn test_rdp_security_protocol_serde_representations() {
        assert_eq!(
            serde_json::to_string(&RdpSecurityProtocol::Auto).unwrap(),
            r#""auto""#
        );
        assert_eq!(
            serde_json::to_string(&RdpSecurityProtocol::Nla).unwrap(),
            r#""nla""#
        );
        assert_eq!(
            serde_json::to_string(&RdpSecurityProtocol::Tls).unwrap(),
            r#""tls""#
        );
        assert_eq!(
            serde_json::to_string(&RdpSecurityProtocol::Rdp).unwrap(),
            r#""rdp""#
        );
        assert_eq!(
            serde_json::to_string(&RdpSecurityProtocol::Ext).unwrap(),
            r#""ext""#
        );

        let s_auto: RdpSecurityProtocol = serde_json::from_str(r#""auto""#).unwrap();
        let s_nla: RdpSecurityProtocol = serde_json::from_str(r#""nla""#).unwrap();
        let s_tls: RdpSecurityProtocol = serde_json::from_str(r#""tls""#).unwrap();
        let s_rdp: RdpSecurityProtocol = serde_json::from_str(r#""rdp""#).unwrap();
        let s_ext: RdpSecurityProtocol = serde_json::from_str(r#""ext""#).unwrap();

        assert_eq!(s_auto, RdpSecurityProtocol::Auto);
        assert_eq!(s_nla, RdpSecurityProtocol::Nla);
        assert_eq!(s_tls, RdpSecurityProtocol::Tls);
        assert_eq!(s_rdp, RdpSecurityProtocol::Rdp);
        assert_eq!(s_ext, RdpSecurityProtocol::Ext);
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
        let mut conn = Connection {
            port: 0,
            protocol: Protocol::Rdp,
            ..Default::default()
        };
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
        let mut conn = Connection {
            mac_address: "".to_string(),
            ..Default::default()
        };
        assert_eq!(conn.validate_mac(), Ok(None));

        conn.mac_address = "00:11:22:33:44:55".to_string();
        assert_eq!(conn.validate_mac(), Ok(Some("001122334455".to_string())));

        conn.mac_address = "00-11-22-33-44-55".to_string();
        assert_eq!(conn.validate_mac(), Ok(Some("001122334455".to_string())));

        conn.mac_address = "invalid-mac".to_string();
        assert!(conn.validate_mac().is_err());
    }

    #[test]
    fn test_ssh_identity_file_serde() {
        let conn = Connection {
            protocol: Protocol::Ssh,
            advanced_settings: AdvancedSettings {
                ssh_identity_file: "/home/user/.ssh/id_ed25519".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };

        let json = serde_json::to_string(&conn).expect("Serialization failed");
        assert!(json.contains("/home/user/.ssh/id_ed25519"));

        let deserialized: Connection = serde_json::from_str(&json).expect("Deserialization failed");
        assert_eq!(
            deserialized.advanced_settings.ssh_identity_file,
            "/home/user/.ssh/id_ed25519"
        );
    }

    #[test]
    fn test_serde_advanced_settings_defaults() {
        let json = r#"{
            "vnc_viewonly": false,
            "vnc_clipboard": true
        }"#;
        let settings: AdvancedSettings = serde_json::from_str(json).unwrap();
        assert!(!settings.vnc_viewonly);
        assert!(settings.vnc_clipboard);
    }

    #[test]
    fn test_legacy_files_default_to_unpinned_without_history() {
        let conn: Connection = serde_json::from_str(r#"{"name": "Old", "host": "h"}"#).unwrap();
        assert!(!conn.pinned);
        let config: AppConfig = serde_json::from_str(r#"{"last_connected_id": "a"}"#).unwrap();
        assert!(config.recent_connection_ids.is_empty());
        assert_eq!(config.last_connected_id.as_deref(), Some("a"));
    }

    #[test]
    fn test_pinned_round_trips() {
        let conn = Connection {
            pinned: true,
            ..Default::default()
        };
        let json = serde_json::to_string(&conn).unwrap();
        let back: Connection = serde_json::from_str(&json).unwrap();
        assert!(back.pinned);
    }

    #[test]
    fn test_record_recent_moves_to_front_and_caps_length() {
        let mut config = AppConfig::default();
        for id in ["a", "b", "c", "d", "e", "f"] {
            config.record_recent(id);
        }
        assert_eq!(config.recent_connection_ids, ["f", "e", "d", "c", "b"]);
        config.record_recent("c");
        assert_eq!(config.recent_connection_ids, ["c", "f", "e", "d", "b"]);
        assert_eq!(config.last_connected_id.as_deref(), Some("c"));
    }

    #[test]
    fn test_pinned_and_recent_connection_lookups() {
        let named = |id: &str, name: &str, pinned: bool| Connection {
            id: id.to_string(),
            name: name.to_string(),
            pinned,
            ..Default::default()
        };
        let connections = vec![
            named("1", "zeta", true),
            named("2", "Alpha", true),
            named("3", "beta", false),
        ];
        let pinned: Vec<&str> = pinned_connections(&connections)
            .iter()
            .map(|c| c.id.as_str())
            .collect();
        assert_eq!(pinned, ["2", "1"]);

        let config = AppConfig {
            recent_connection_ids: vec!["3".into(), "deleted".into(), "1".into()],
            ..Default::default()
        };
        let recent: Vec<&str> = recent_connections(&config, &connections)
            .iter()
            .map(|c| c.id.as_str())
            .collect();
        assert_eq!(recent, ["3", "1"]);
    }

    #[test]
    fn test_parse_ssh_forwards() {
        let local = SshForward::parse("L 8080:localhost:80").unwrap();
        assert_eq!(local.kind, SshForwardKind::Local);
        assert_eq!((local.listen_port, local.dest_port), (8080, 80));
        assert_eq!(local.spec(), "8080:localhost:80");

        let remote = SshForward::parse("-R 0.0.0.0:9000:[::1]:3000").unwrap();
        assert_eq!(remote.kind, SshForwardKind::Remote);
        assert_eq!(remote.bind_address, "0.0.0.0");
        assert_eq!(remote.dest_host, "::1");
        assert_eq!(remote.spec(), "0.0.0.0:9000:[::1]:3000");

        let socks = SshForward::parse("D 1080").unwrap();
        assert_eq!(socks.spec(), "1080");
        assert_eq!(socks.describe(), "SOCKS proxy on 1080");

        for bad in [
            "8080:localhost:80",
            "X 1:a:2",
            "L 0:localhost:80",
            "L 8080:-oProxyCommand=x:80",
            "L 8080:localhost",
            "D 1:2:3",
        ] {
            assert!(
                SshForward::parse(bad).is_err(),
                "{} should be rejected",
                bad
            );
        }
    }

    #[test]
    fn test_parse_ssh_destinations() {
        let full = SshDestination::parse("admin@bastion.example:2222").unwrap();
        assert_eq!(full.user.as_deref(), Some("admin"));
        assert_eq!(
            (full.host.as_str(), full.port),
            ("bastion.example", Some(2222))
        );
        assert_eq!(full.target(), "admin@bastion.example");

        let ipv6 = SshDestination::parse("[2001:db8::1]:22").unwrap();
        assert_eq!((ipv6.host.as_str(), ipv6.port), ("2001:db8::1", Some(22)));
        assert_eq!(SshDestination::parse("2001:db8::1").unwrap().port, None);

        for bad in ["", "-oProxyCommand=x", "@host", "host:0", "user@bad host"] {
            assert!(
                SshDestination::parse(bad).is_err(),
                "{} should be rejected",
                bad
            );
        }
        assert!(validate_jump_hosts("a@one:22,two").is_ok());
        assert!(validate_jump_hosts("one,,two").is_err());
    }

    #[test]
    fn test_ssh_settings_are_validated() {
        let mut conn = Connection {
            host: "server".into(),
            ..Default::default()
        };
        conn.advanced_settings.ssh_jump_host = "-oProxyCommand=evil".into();
        assert!(conn.validate().is_err());
        conn.advanced_settings.ssh_jump_host.clear();
        conn.advanced_settings.ssh_tunnel_host = "gateway:0".into();
        assert!(conn.validate().is_err());
        conn.advanced_settings.ssh_tunnel_host = "me@gateway".into();
        assert!(conn.validate().is_ok());
    }

    #[test]
    fn test_probe_target_uses_first_ssh_hop() {
        let mut rdp = Connection {
            protocol: Protocol::Rdp,
            host: "10.0.0.5".into(),
            port: 3389,
            ..Default::default()
        };
        assert_eq!(rdp.probe_target(), ("10.0.0.5".into(), 3389));
        rdp.advanced_settings.ssh_tunnel_host = "me@gateway:2222".into();
        assert!(rdp.uses_ssh_tunnel());
        assert_eq!(rdp.probe_target(), ("gateway".into(), 2222));

        let mut ssh = Connection::new_with_protocol(Protocol::Ssh);
        ssh.host = "internal".into();
        ssh.advanced_settings.ssh_tunnel_host = "ignored".into();
        assert!(!ssh.uses_ssh_tunnel());
        ssh.advanced_settings.ssh_jump_host = "bastion,second".into();
        assert_eq!(ssh.probe_target(), ("bastion".into(), 22));
    }

    #[test]
    fn test_old_settings_have_no_ssh_routing() {
        let settings: AdvancedSettings =
            serde_json::from_str(r#"{"ssh_identity_file": "~/.ssh/id"}"#).unwrap();
        assert!(settings.ssh_jump_host.is_empty());
        assert!(settings.ssh_forwards.is_empty());
        assert!(settings.ssh_tunnel_host.is_empty());
    }
}
