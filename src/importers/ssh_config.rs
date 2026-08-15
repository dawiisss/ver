use std::fs;
use std::path::Path;

use crate::importers::ImporterError;
use crate::models::{AdvancedSettings, Connection, Protocol};

#[derive(Default)]
struct SshHostBlock {
    aliases: Vec<String>,
    hostname: Option<String>,
    user: Option<String>,
    port: Option<u16>,
    identity_file: Option<String>,
}

/// Parses OpenSSH config content (`~/.ssh/config`) into a list of VER `Connection`s.
pub fn import_ssh_config_content(content: &str) -> Result<Vec<Connection>, ImporterError> {
    let mut blocks: Vec<SshHostBlock> = Vec::new();
    let mut current_block: Option<SshHostBlock> = None;
    let mut global_defaults = SshHostBlock::default();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Split key and value (separated by whitespace or '=')
        let (key, val) = match trimmed.split_once('=') {
            Some((k, v)) => (k.trim(), v.trim()),
            None => match trimmed.split_once(char::is_whitespace) {
                Some((k, v)) => (k.trim(), v.trim()),
                None => continue,
            },
        };

        if key.eq_ignore_ascii_case("Host") {
            if let Some(block) = current_block.take() {
                blocks.push(block);
            }

            let aliases: Vec<String> = val
                .split_whitespace()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            if aliases.is_empty() || aliases.iter().any(|a| a == "*") {
                current_block = None;
            } else {
                current_block = Some(SshHostBlock {
                    aliases,
                    ..Default::default()
                });
            }
            continue;
        }

        if let Some(ref mut block) = current_block {
            apply_ssh_directive(block, key, val);
        } else {
            apply_ssh_directive(&mut global_defaults, key, val);
        }
    }

    if let Some(block) = current_block {
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

        let port = block
            .port
            .or(global_defaults.port)
            .unwrap_or(22);

        let identity_file = block
            .identity_file
            .or(global_defaults.identity_file.clone())
            .unwrap_or_default();

        let expanded_identity = expand_tilde(&identity_file);

        for alias in block.aliases {
            let adv = AdvancedSettings {
                ssh_identity_file: expanded_identity.clone(),
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
                advanced_settings: adv,
            });
        }
    }

    Ok(connections)
}

fn apply_ssh_directive(block: &mut SshHostBlock, key: &str, val: &str) {
    if key.eq_ignore_ascii_case("HostName") {
        block.hostname = Some(val.to_string());
    } else if key.eq_ignore_ascii_case("User") {
        block.user = Some(val.to_string());
    } else if key.eq_ignore_ascii_case("Port") {
        if let Ok(p) = val.parse::<u16>() {
            block.port = Some(p);
        }
    } else if key.eq_ignore_ascii_case("IdentityFile") {
        block.identity_file = Some(val.to_string());
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
