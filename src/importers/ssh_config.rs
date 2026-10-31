use std::fs;
use std::path::Path;

use crate::importers::ImporterError;
use crate::models::{AdvancedSettings, Connection, Protocol, SshForward, SshForwardKind};

#[derive(Default)]
struct SshHostBlock {
    aliases: Vec<String>,
    hostname: Option<String>,
    user: Option<String>,
    port: Option<u16>,
    identity_file: Option<String>,
    proxy_jump: Option<String>,
    /// Unlike other directives, every forward that applies to a host is used.
    forwards: Vec<SshForward>,
}

/// Which hosts the directives currently being read apply to.
enum Scope {
    /// Outside any block, or under `Host *` / `Match all`: applies to every host.
    Global,
    Host(SshHostBlock),
    /// Under a wildcard pattern or `Match` criteria that only apply conditionally.
    Ignored,
}

/// Parses OpenSSH config content (`~/.ssh/config`) into a list of VER `Connection`s.
pub fn import_ssh_config_content(content: &str) -> Result<Vec<Connection>, ImporterError> {
    let mut blocks: Vec<SshHostBlock> = Vec::new();
    let mut scope = Scope::Global;
    let mut global_defaults = SshHostBlock::default();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // The key ends at the first whitespace or '='; the value may be preceded by " = ".
        let (key, val) = match trimmed.split_once(|c: char| c == '=' || c.is_whitespace()) {
            Some((k, v)) => {
                let v = v.trim();
                (k, v.strip_prefix('=').unwrap_or(v).trim())
            }
            None => continue,
        };

        let is_host = key.eq_ignore_ascii_case("Host");
        if is_host || key.eq_ignore_ascii_case("Match") {
            if let Scope::Host(block) = std::mem::replace(&mut scope, Scope::Ignored) {
                blocks.push(block);
            }

            let patterns: Vec<&str> = val.split_whitespace().collect();
            if is_host {
                let aliases: Vec<String> = patterns
                    .iter()
                    .filter(|s| !s.starts_with('!') && !s.contains('*') && !s.contains('?'))
                    .map(|s| s.to_string())
                    .collect();
                if !aliases.is_empty() {
                    scope = Scope::Host(SshHostBlock {
                        aliases,
                        ..Default::default()
                    });
                } else if patterns == ["*"] {
                    scope = Scope::Global;
                }
            } else if patterns.len() == 1 && patterns[0].eq_ignore_ascii_case("all") {
                scope = Scope::Global;
            }
            continue;
        }

        match &mut scope {
            Scope::Global => apply_ssh_directive(&mut global_defaults, key, val),
            Scope::Host(block) => apply_ssh_directive(block, key, val),
            Scope::Ignored => {}
        }
    }

    if let Scope::Host(block) = scope {
        blocks.push(block);
    }

    let mut connections = Vec::new();

    for block in blocks {
        let host = block
            .hostname
            .or(global_defaults.hostname.clone())
            .unwrap_or_else(|| block.aliases.first().cloned().unwrap_or_default());

        if host.is_empty() {
            continue;
        }

        let user = block
            .user
            .or(global_defaults.user.clone())
            .unwrap_or_default();

        let port = block.port.or(global_defaults.port).unwrap_or(22);

        let identity_file = block
            .identity_file
            .or(global_defaults.identity_file.clone())
            .unwrap_or_default();

        let expanded_identity = expand_tilde(&identity_file);

        // `ProxyJump none` disables a jump host set by a broader section.
        let jump_host = block
            .proxy_jump
            .or(global_defaults.proxy_jump.clone())
            .filter(|jump| !jump.eq_ignore_ascii_case("none"))
            .unwrap_or_default();

        let mut forwards = block.forwards;
        forwards.extend(global_defaults.forwards.iter().cloned());

        for alias in block.aliases {
            let adv = AdvancedSettings {
                ssh_identity_file: expanded_identity.clone(),
                ssh_jump_host: jump_host.clone(),
                ssh_forwards: forwards.clone(),
                ..Default::default()
            };

            connections.push(Connection {
                id: uuid::Uuid::new_v4().to_string(),
                name: alias,
                protocol: Protocol::Ssh,
                host: host.clone(),
                port,
                username: user.clone(),
                mac_address: String::new(),
                group: "SSH Config".to_string(),
                pinned: false,
                advanced_settings: adv,
            });
        }
    }

    Ok(connections)
}

fn apply_ssh_directive(block: &mut SshHostBlock, key: &str, val: &str) {
    if key.eq_ignore_ascii_case("HostName") && block.hostname.is_none() {
        block.hostname = Some(val.to_string());
    } else if key.eq_ignore_ascii_case("User") && block.user.is_none() {
        block.user = Some(val.to_string());
    } else if key.eq_ignore_ascii_case("Port") && block.port.is_none() {
        if let Ok(p) = val.parse::<u16>() {
            block.port = Some(p);
        }
    } else if key.eq_ignore_ascii_case("IdentityFile") && block.identity_file.is_none() {
        block.identity_file = Some(val.to_string());
    } else if key.eq_ignore_ascii_case("ProxyJump") && block.proxy_jump.is_none() {
        block.proxy_jump = Some(val.to_string());
    } else if let Some(kind) = forward_kind(key) {
        // `LocalForward [bind:]port host:hostport`; Unix socket forwards are not supported.
        let spec = val.split_whitespace().collect::<Vec<_>>().join(":");
        if let Ok(forward) = SshForward::parse_spec(kind, &spec) {
            block.forwards.push(forward);
        }
    }
}

fn forward_kind(key: &str) -> Option<SshForwardKind> {
    if key.eq_ignore_ascii_case("LocalForward") {
        Some(SshForwardKind::Local)
    } else if key.eq_ignore_ascii_case("RemoteForward") {
        Some(SshForwardKind::Remote)
    } else if key.eq_ignore_ascii_case("DynamicForward") {
        Some(SshForwardKind::Dynamic)
    } else {
        None
    }
}

/// Reads and imports an OpenSSH config file at `path`.
pub fn import_ssh_config_file(path: &Path) -> Result<Vec<Connection>, ImporterError> {
    let content = fs::read_to_string(path)?;
    import_ssh_config_content(&content)
}

/// Helper to expand `~/` in paths.
fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest).to_string_lossy().to_string();
        }
    }
    path.to_string()
}
