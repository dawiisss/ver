use ver::importers::{
    export_connections_json, export_rdp_file, import_connections_json, import_rdp_content,
    import_remmina_content, import_ssh_config_content, merge_imported_connections,
    ImportConflictStrategy,
};
use ver::models::{Connection, Protocol, RdpCertHandling, SshForwardKind};

#[test]
fn test_remmina_rdp_parsing() {
    let remmina_content = r#"
[remmina]
name=Windows Server 2022
server=192.168.1.100:3389
protocol=RDP
group=Production/Servers
username=Administrator
domain=CORP
colordepth=32
cert_ignore=1
multimon=1
viewmode=2
sound=local
glyph-cache=1
"#;

    let conn = import_remmina_content(remmina_content, Some("test.remmina")).unwrap();
    assert_eq!(conn.name, "Windows Server 2022");
    assert_eq!(conn.protocol, Protocol::Rdp);
    assert_eq!(conn.host, "192.168.1.100");
    assert_eq!(conn.port, 3389);
    assert_eq!(conn.username, "Administrator");
    assert_eq!(conn.group, "Production/Servers");
    assert_eq!(conn.advanced_settings.rdp_domain, "CORP");
    assert_eq!(
        conn.advanced_settings.rdp_cert_handling,
        RdpCertHandling::Ignore
    );
    assert!(conn.advanced_settings.rdp_multimon);
    assert!(conn.advanced_settings.rdp_fullscreen);
    assert!(conn.advanced_settings.rdp_audio);
}

#[test]
fn test_remmina_vnc_and_ssh_parsing() {
    let vnc_content = r#"
[remmina]
name=VNC Host
server=10.0.0.5:5901
protocol=VNC
group=Lab
disableclipboard=1
"#;
    let conn = import_remmina_content(vnc_content, None).unwrap();
    assert_eq!(conn.name, "VNC Host");
    assert_eq!(conn.protocol, Protocol::Vnc);
    assert_eq!(conn.host, "10.0.0.5");
    assert_eq!(conn.port, 5901);
    assert!(!conn.advanced_settings.clipboard_sharing);

    let windowed = import_remmina_content(&format!("{}viewmode=1\n", vnc_content), None).unwrap();
    assert!(!windowed.advanced_settings.vnc_fullscreen);

    let ssh_content = r#"
[remmina]
name=Ubuntu Bastion
server=bastion.example.com
protocol=SSH
username=ubuntu
ssh_tunnel_privatekey=/home/user/.ssh/id_rsa
"#;
    let conn_ssh = import_remmina_content(ssh_content, None).unwrap();
    assert_eq!(conn_ssh.protocol, Protocol::Ssh);
    assert_eq!(conn_ssh.port, 22);
    assert_eq!(conn_ssh.username, "ubuntu");
    assert_eq!(
        conn_ssh.advanced_settings.ssh_identity_file,
        "/home/user/.ssh/id_rsa"
    );
}

#[test]
fn test_ssh_config_parsing() {
    let ssh_config = r#"
# Global configuration
Host *
  ServerAliveInterval 60
  User defaultuser

Host web-prod web-backup
  HostName 192.168.50.10
  User deploy
  Port 2222
  IdentityFile ~/.ssh/deploy_key

Host db-master
  HostName db.internal.net
  Port 22
"#;

    let conns = import_ssh_config_content(ssh_config).unwrap();
    assert_eq!(conns.len(), 3);

    let web1 = conns.iter().find(|c| c.name == "web-prod").unwrap();
    assert_eq!(web1.host, "192.168.50.10");
    assert_eq!(web1.port, 2222);
    assert_eq!(web1.username, "deploy");
    assert!(web1
        .advanced_settings
        .ssh_identity_file
        .ends_with(".ssh/deploy_key"));

    let web2 = conns.iter().find(|c| c.name == "web-backup").unwrap();
    assert_eq!(web2.host, "192.168.50.10");
    assert_eq!(web2.port, 2222);

    let db = conns.iter().find(|c| c.name == "db-master").unwrap();
    assert_eq!(db.host, "db.internal.net");
    assert_eq!(db.username, "defaultuser");
}

#[test]
fn test_ssh_config_conditional_blocks_do_not_leak() {
    let ssh_config = r#"
Host *.internal !legacy.internal
  HostName jump.example.com
  User ops

Host prod
  HostName 10.0.0.5

Match exec "test -f /tmp/ci"
  User ci

Host web
  Port = 2200

Match all
  User fallback
"#;

    let conns = import_ssh_config_content(ssh_config).unwrap();
    assert_eq!(conns.len(), 2);

    let prod = conns.iter().find(|c| c.name == "prod").unwrap();
    assert_eq!(prod.host, "10.0.0.5");
    assert_eq!(prod.username, "fallback");

    let web = conns.iter().find(|c| c.name == "web").unwrap();
    assert_eq!(web.host, "web");
    assert_eq!(web.port, 2200);
    assert_eq!(web.username, "fallback");
}

#[test]
fn test_rdp_file_import_and_export_roundtrip() {
    let rdp_content = "full address:s:rdp.workplace.com:3390\r\nusername:s:john.doe\r\ndomain:s:WORK\r\nscreen mode id:i:2\r\nuse multimon:i:1\r\nredirectclipboard:i:1\r\naudiomode:i:0\r\nsession bpp:i:32\r\nauthentication level:i:0\r\ndesktopwidth:i:2560\r\ndesktopheight:i:1440\r\n";

    let conn = import_rdp_content(rdp_content, Some("office.rdp")).unwrap();
    assert_eq!(conn.name, "office");
    assert_eq!(conn.protocol, Protocol::Rdp);
    assert_eq!(conn.host, "rdp.workplace.com");
    assert_eq!(conn.port, 3390);
    assert_eq!(conn.username, "john.doe");
    assert_eq!(conn.advanced_settings.rdp_domain, "WORK");
    assert!(conn.advanced_settings.rdp_fullscreen);
    assert!(conn.advanced_settings.rdp_multimon);
    assert_eq!(conn.advanced_settings.rdp_custom_resolution, "2560x1440");
    assert_eq!(
        conn.advanced_settings.rdp_cert_handling,
        RdpCertHandling::Ignore
    );

    let exported = export_rdp_file(&conn);
    assert!(exported.contains("full address:s:rdp.workplace.com:3390"));
    assert!(exported.contains("username:s:john.doe"));
    assert!(exported.contains("domain:s:WORK"));
    assert!(exported.contains("screen mode id:i:2"));
    assert!(exported.contains("use multimon:i:1"));
    assert!(exported.contains("desktopwidth:i:2560"));
    assert!(exported.contains("desktopheight:i:1440"));
}

#[test]
fn test_json_backup_export_and_import() {
    let mut conn1 = Connection::new_with_protocol(Protocol::Rdp);
    conn1.name = "Primary RDP".to_string();
    conn1.host = "1.2.3.4".to_string();

    let mut conn2 = Connection::new_with_protocol(Protocol::Ssh);
    conn2.name = "Primary SSH".to_string();
    conn2.host = "5.6.7.8".to_string();

    let conns = vec![conn1.clone(), conn2.clone()];
    let json = export_connections_json(&conns).unwrap();

    let imported = import_connections_json(&json).unwrap();
    assert_eq!(imported.len(), 2);
    assert_eq!(imported[0].name, "Primary RDP");
    assert_eq!(imported[1].name, "Primary SSH");
}

#[test]
fn test_conflict_merge_strategies() {
    let mut base_conn = Connection::new_with_protocol(Protocol::Rdp);
    base_conn.id = "id-123".to_string();
    base_conn.name = "Server A".to_string();
    base_conn.host = "10.0.0.1".to_string();

    let mut incoming = base_conn.clone();
    incoming.username = "new_admin".to_string();

    // 1. Skip Strategy
    let mut list = vec![base_conn.clone()];
    let (added, updated, skipped) = merge_imported_connections(
        &mut list,
        vec![incoming.clone()],
        ImportConflictStrategy::SkipDuplicates,
    );
    assert_eq!((added, updated, skipped), (0, 0, 1));
    assert_eq!(list[0].username, "");

    // 2. Overwrite Strategy
    let mut list = vec![base_conn.clone()];
    let (added, updated, skipped) = merge_imported_connections(
        &mut list,
        vec![incoming.clone()],
        ImportConflictStrategy::Overwrite,
    );
    assert_eq!((added, updated, skipped), (0, 1, 0));
    assert_eq!(list[0].username, "new_admin");

    // 3. Rename Strategy
    let mut list = vec![base_conn.clone()];
    let (added, updated, skipped) = merge_imported_connections(
        &mut list,
        vec![incoming.clone()],
        ImportConflictStrategy::RenameWithSuffix,
    );
    assert_eq!((added, updated, skipped), (1, 0, 0));
    assert_eq!(list.len(), 2);
    assert_eq!(list[1].name, "Server A (Imported)");
}

#[test]
fn test_ssh_config_jump_hosts_and_forwards() {
    let content = r#"
Host *
    DynamicForward 1080

Host internal
    HostName 10.0.0.9
    ProxyJump admin@bastion:2222
    LocalForward 8080 localhost:80
    LocalForward 127.0.0.1:5433 db:5432
    RemoteForward 9000 localhost:3000
    LocalForward /tmp/socket /var/run/socket

Host bastion
    ProxyJump none
"#;
    let conns = import_ssh_config_content(content).unwrap();
    let internal = conns.iter().find(|c| c.name == "internal").unwrap();
    let adv = &internal.advanced_settings;
    assert_eq!(adv.ssh_jump_host, "admin@bastion:2222");
    let specs: Vec<(SshForwardKind, String)> = adv
        .ssh_forwards
        .iter()
        .map(|f| (f.kind, f.spec()))
        .collect();
    assert_eq!(
        specs,
        [
            (SshForwardKind::Local, "8080:localhost:80".to_string()),
            (SshForwardKind::Local, "127.0.0.1:5433:db:5432".to_string()),
            (SshForwardKind::Remote, "9000:localhost:3000".to_string()),
            (SshForwardKind::Dynamic, "1080".to_string()),
        ]
    );
    let bastion = conns.iter().find(|c| c.name == "bastion").unwrap();
    assert!(bastion.advanced_settings.ssh_jump_host.is_empty());
}

#[test]
fn test_remmina_ssh_tunnel_import() {
    let content = r#"
[remmina]
name=Behind Gateway
protocol=VNC
server=10.0.0.20:5901
ssh_tunnel_enabled=1
ssh_tunnel_server=gateway.example:2222
ssh_tunnel_username=ops
ssh_tunnel_privatekey=/home/ops/.ssh/id_ed25519
"#;
    let conn = import_remmina_content(content, None).unwrap();
    assert_eq!(conn.host, "10.0.0.20");
    assert_eq!(
        conn.advanced_settings.ssh_tunnel_host,
        "ops@gateway.example:2222"
    );
    assert_eq!(
        conn.advanced_settings.ssh_tunnel_identity_file,
        "/home/ops/.ssh/id_ed25519"
    );
    assert!(conn.uses_ssh_tunnel());

    let loopback = import_remmina_content(
        "[remmina]\nprotocol=RDP\nserver=winbox\nssh_tunnel_enabled=1\nssh_tunnel_loopback=1\n",
        None,
    )
    .unwrap();
    assert_eq!(loopback.host, "127.0.0.1");
    assert_eq!(loopback.advanced_settings.ssh_tunnel_host, "winbox");

    let disabled = import_remmina_content(
        "[remmina]\nprotocol=RDP\nserver=winbox\nssh_tunnel_server=gw\n",
        None,
    )
    .unwrap();
    assert!(!disabled.uses_ssh_tunnel());
}
