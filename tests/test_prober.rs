use std::net::TcpListener;
use std::time::Duration;
use ver::prober::{probe_connections_batch, probe_host_async, probe_host_sync, HostStatus};

#[test]
fn test_probe_sync_local_open_and_closed_ports() {
    // Bind a temporary local TCP listener
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    // Probe open port
    let status_open = probe_host_sync("127.0.0.1", port, Duration::from_millis(500));
    assert!(status_open.is_online());

    // Drop listener to close port
    drop(listener);

    // Probe closed port
    let status_closed = probe_host_sync("127.0.0.1", port, Duration::from_millis(200));
    assert!(!status_closed.is_online());
}

#[tokio::test]
async fn test_probe_async_and_batch() {
    let listener1 = TcpListener::bind("127.0.0.1:0").unwrap();
    let port1 = listener1.local_addr().unwrap().port();

    let listener2 = TcpListener::bind("127.0.0.1:0").unwrap();
    let port2 = listener2.local_addr().unwrap().port();

    // Test async single probe
    let status = probe_host_async("127.0.0.1".to_string(), port1, Duration::from_millis(500)).await;
    assert!(status.is_online());

    // Test batch probe
    let targets = vec![
        ("conn-1".to_string(), "127.0.0.1".to_string(), port1),
        ("conn-2".to_string(), "127.0.0.1".to_string(), port2),
        ("conn-3".to_string(), "127.0.0.1".to_string(), 64999), // closed port
    ];

    let results = probe_connections_batch(targets, Duration::from_millis(500), 4).await;
    assert_eq!(results.len(), 3);

    let res1 = results.iter().find(|(id, _)| id == "conn-1").unwrap();
    assert!(res1.1.is_online());

    let res2 = results.iter().find(|(id, _)| id == "conn-2").unwrap();
    assert!(res2.1.is_online());

    let res3 = results.iter().find(|(id, _)| id == "conn-3").unwrap();
    assert!(!res3.1.is_online());
}

#[test]
fn test_host_status_helpers() {
    let online = HostStatus::Online { latency_ms: 12 };
    assert_eq!(online.status_css_class(), "status-online");
    assert_eq!(online.description(), "Online (12 ms)");

    let offline = HostStatus::Offline {
        reason: "Connection refused".to_string(),
    };
    assert_eq!(offline.status_css_class(), "status-offline");
    assert_eq!(offline.description(), "Offline (Connection refused)");

    let probing = HostStatus::Probing;
    assert_eq!(probing.status_css_class(), "status-probing");
}
