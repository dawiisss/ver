use std::collections::HashSet;
use std::fs;
use ver::importers::{
    export_connections_json, import_connections_json, merge_imported_connections,
    ImportConflictStrategy,
};
use ver::models::{Connection, Protocol};
use ver::network::{parse_mac_address, Ipv4Subnet};
use ver::storage::{
    load_connections_from_path, save_connections_to_path, update_connections_at_path,
};

fn endpoint(port: u16) -> Connection {
    Connection {
        name: "Bastion".into(),
        host: "example.test".into(),
        port,
        protocol: Protocol::Ssh,
        ..Default::default()
    }
}

#[test]
fn failed_commits_preserve_memory_and_the_previous_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("connections.json");
    let mut connections = vec![endpoint(22)];
    save_connections_to_path(&path, &connections).unwrap();
    let original = connections.clone();
    let original_file = fs::read(&path).unwrap();
    // A destination directory cannot be replaced by the atomic file rename.
    let blocked = dir.path().join("blocked");
    fs::create_dir(&blocked).unwrap();
    for operation in 0..3 {
        let result =
            update_connections_at_path(&blocked, &mut connections, |candidate| match operation {
                0 => candidate.push(endpoint(2222)),
                1 => candidate[0].name = "Edited".into(),
                _ => candidate.clear(),
            });
        assert!(result.is_err());
        assert_eq!(connections, original);
        assert_eq!(fs::read(&path).unwrap(), original_file);
    }
    update_connections_at_path(&path, &mut connections, |candidate| {
        candidate.push(endpoint(2222))
    })
    .unwrap();
    assert_eq!(load_connections_from_path(&path).unwrap(), connections);
}

#[test]
fn duplicate_ids_are_repaired_without_changing_legacy_keyring_ids() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("connections.json");
    let mut first = endpoint(22);
    first.id = "legacy-connection-id".into();
    let mut duplicate = first.clone();
    duplicate.name = "Second".into();
    duplicate.port = 2222;
    let third = endpoint(2200);
    let original = vec![first.clone(), duplicate, third.clone()];
    fs::write(&path, serde_json::to_vec(&original).unwrap()).unwrap();
    let repaired = load_connections_from_path(&path).unwrap();
    assert_eq!(repaired[0].id, first.id);
    assert_eq!(repaired[2].id, third.id);
    assert_eq!(
        repaired.iter().map(|c| &c.id).collect::<HashSet<_>>().len(),
        3
    );
    assert!(uuid::Uuid::parse_str(&repaired[1].id).is_ok());
    save_connections_to_path(&path, &repaired).unwrap();
    assert_eq!(load_connections_from_path(&path).unwrap(), repaired);
    assert!(save_connections_to_path(&path, &original).is_err());
    assert_eq!(load_connections_from_path(&path).unwrap(), repaired);
}

#[test]
fn imports_preserve_distinct_ports_for_every_conflict_strategy() {
    for strategy in [
        ImportConflictStrategy::SkipDuplicates,
        ImportConflictStrategy::Overwrite,
        ImportConflictStrategy::RenameWithSuffix,
    ] {
        let mut existing = vec![endpoint(22)];
        assert_eq!(
            merge_imported_connections(&mut existing, vec![endpoint(2222)], strategy),
            (1, 0, 0)
        );
        assert_eq!(existing.len(), 2);
        assert_eq!(existing[0].port, 22);
        assert_eq!(existing[1].port, 2222);
    }
}

#[test]
fn duplicate_import_ids_and_repeated_renames_remain_distinct() {
    let first = endpoint(22);
    let mut second = first.clone();
    second.port = 2222;
    let imported =
        import_connections_json(&export_connections_json(&[first.clone(), second]).unwrap())
            .unwrap();
    assert_ne!(imported[0].id, imported[1].id);
    let mut existing = vec![];
    assert_eq!(
        merge_imported_connections(&mut existing, imported, ImportConflictStrategy::Overwrite),
        (2, 0, 0)
    );
    for _ in 0..3 {
        merge_imported_connections(
            &mut existing,
            vec![first.clone()],
            ImportConflictStrategy::RenameWithSuffix,
        );
    }
    assert_eq!(
        existing.iter().map(|c| &c.id).collect::<HashSet<_>>().len(),
        5
    );
    assert!(existing.iter().any(|c| c.name == "Bastion (Imported 3)"));
}

#[test]
fn backup_schema_rejects_invalid_envelopes_and_accepts_older_backups() {
    let backup = export_connections_json(&[endpoint(22)]).unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&backup).unwrap();
    assert_eq!(value["schema_version"], 1);
    value["schema_version"] = 2.into();
    assert!(import_connections_json(&value.to_string()).is_err());
    value.as_object_mut().unwrap().remove("schema_version");
    assert_eq!(
        import_connections_json(&value.to_string()).unwrap().len(),
        1
    );
    assert!(import_connections_json(r#"{"version":"1.5.1","connections":"invalid"}"#).is_err());
    assert!(import_connections_json(r#"{"host":"host with spaces"}"#).is_err());
}

#[test]
fn discovery_ranges_respect_netmask_and_point_to_point_endpoints() {
    let subnet: Ipv4Subnet = "192.168.3.100/23".parse().unwrap();
    assert_eq!(subnet.to_string(), "192.168.2.0/23");
    let hosts = subnet.hosts().unwrap();
    assert_eq!(hosts.len(), 510);
    assert_eq!(hosts.first().unwrap().to_string(), "192.168.2.1");
    assert_eq!(hosts.last().unwrap().to_string(), "192.168.3.254");
    for (cidr, expected) in [("10.0.0.0/30", 2), ("10.0.0.0/31", 2), ("10.0.0.7/32", 1)] {
        assert_eq!(
            cidr.parse::<Ipv4Subnet>().unwrap().hosts().unwrap().len(),
            expected
        );
    }
    for invalid in ["10.0.0.0/33", "192.168.1.1", "[::1]/64"] {
        assert!(invalid.parse::<Ipv4Subnet>().is_err());
    }
    assert!("10.0.0.0/8".parse::<Ipv4Subnet>().unwrap().hosts().is_err());
    assert!("0.0.0.0/0".parse::<Ipv4Subnet>().unwrap().hosts().is_err());
}

#[test]
fn malformed_unicode_mac_addresses_return_errors_instead_of_panicking() {
    assert!(parse_mac_address("€€€€").is_err());
}

#[test]
fn backups_preserve_unfinished_connections() {
    let draft = Connection::default();
    let restored =
        import_connections_json(&export_connections_json(std::slice::from_ref(&draft)).unwrap())
            .unwrap();
    assert_eq!(restored, vec![draft]);
    assert!(import_connections_json(r#"{"unrelated":"object"}"#).is_err());
}

#[test]
fn missing_ports_default_from_the_protocol() {
    let imported =
        import_connections_json(r#"[{"protocol":"ssh","host":"a"},{"protocol":"vnc","host":"b"}]"#)
            .unwrap();
    assert_eq!(imported[0].port, 22);
    assert_eq!(imported[1].port, 5900);
    let explicit: Connection =
        serde_json::from_str(r#"{"protocol":"ssh","host":"a","port":2222}"#).unwrap();
    assert_eq!(explicit.port, 2222);
}

#[test]
fn hosts_that_look_like_options_are_rejected() {
    let conn = Connection {
        host: "-SecurityTypes=None".into(),
        protocol: Protocol::Vnc,
        ..endpoint(5900)
    };
    assert!(conn.validate().is_err());
    assert!(import_connections_json(r#"[{"protocol":"vnc","host":"-via=x"}]"#).is_err());
    assert!(ver::launcher::launch_vnc(&conn, None).is_err());
}
