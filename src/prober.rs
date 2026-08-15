use std::net::{SocketAddr, ToSocketAddrs};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostStatus {
    Unknown,
    Probing,
    Online { latency_ms: u64 },
    Offline { reason: String },
}

impl HostStatus {
    pub fn is_online(&self) -> bool {
        matches!(self, HostStatus::Online { .. })
    }

    pub fn status_icon_name(&self) -> &'static str {
        match self {
            HostStatus::Unknown => "emblem-unreadable-symbolic",
            HostStatus::Probing => "view-refresh-symbolic",
            HostStatus::Online { .. } => "emblem-ok-symbolic",
            HostStatus::Offline { .. } => "emblem-important-symbolic",
        }
    }

    pub fn status_css_class(&self) -> &'static str {
        match self {
            HostStatus::Unknown => "status-unknown",
            HostStatus::Probing => "status-probing",
            HostStatus::Online { .. } => "status-online",
            HostStatus::Offline { .. } => "status-offline",
        }
    }

    pub fn description(&self) -> String {
        match self {
            HostStatus::Unknown => "Not checked".to_string(),
            HostStatus::Probing => "Checking reachability...".to_string(),
            HostStatus::Online { latency_ms } => format!("Online ({} ms)", latency_ms),
            HostStatus::Offline { reason } => format!("Offline ({})", reason),
        }
    }
}

/// Probes a single host and port synchronously with a timeout.
pub fn probe_host_sync(host: &str, port: u16, timeout: Duration) -> HostStatus {
    if host.trim().is_empty() {
        return HostStatus::Offline {
            reason: "Empty hostname".to_string(),
        };
    }

    let target = format!("{}:{}", host.trim(), port);
    let start = Instant::now();

    // Resolve socket address with timeout
    let socket_addrs: Vec<SocketAddr> = match target.to_socket_addrs() {
        Ok(addrs) => addrs.collect(),
        Err(e) => {
            return HostStatus::Offline {
                reason: format!("DNS lookup failed: {}", e),
            };
        }
    };

    if socket_addrs.is_empty() {
        return HostStatus::Offline {
            reason: "No IP address found for host".to_string(),
        };
    }

    let mut last_err = String::from("Connection timed out");
    for addr in socket_addrs {
        match std::net::TcpStream::connect_timeout(&addr, timeout) {
            Ok(_) => {
                let latency_ms = start.elapsed().as_millis().max(1) as u64;
                return HostStatus::Online { latency_ms };
            }
            Err(e) => {
                last_err = e.to_string();
            }
        }
    }

    HostStatus::Offline { reason: last_err }
}

/// Spawns a background thread pool to probe multiple connection targets concurrently
/// and stream results over an `async_channel::Sender`. Does not depend on Tokio reactor.
pub fn spawn_batch_probe(
    targets: Vec<(String, String, u16)>, // (conn_id, host, port)
    timeout: Duration,
    concurrency: usize,
    tx: async_channel::Sender<(String, HostStatus)>,
) {
    if targets.is_empty() {
        return;
    }

    std::thread::spawn(move || {
        let (work_tx, work_rx) = async_channel::unbounded::<(String, String, u16)>();

        for item in targets {
            let _ = work_tx.send_blocking(item);
        }
        drop(work_tx); // close queue

        let num_workers = concurrency.clamp(1, 16);
        let mut handles = Vec::new();

        for _ in 0..num_workers {
            let rx_worker = work_rx.clone();
            let tx_out = tx.clone();
            let handle = std::thread::spawn(move || {
                while let Ok((id, host, port)) = rx_worker.recv_blocking() {
                    let status = probe_host_sync(&host, port, timeout);
                    if tx_out.send_blocking((id, status)).is_err() {
                        break;
                    }
                }
            });
            handles.push(handle);
        }

        for h in handles {
            let _ = h.join();
        }
    });
}

/// Probes a single host asynchronously from any context (GLib, Tokio, or sync thread).
pub async fn probe_host_async(host: String, port: u16, timeout: Duration) -> HostStatus {
    let (tx, rx) = async_channel::bounded::<HostStatus>(1);
    std::thread::spawn(move || {
        let status = probe_host_sync(&host, port, timeout);
        let _ = tx.send_blocking(status);
    });

    rx.recv().await.unwrap_or_else(|_| HostStatus::Offline {
        reason: "Prober channel closed".to_string(),
    })
}

/// Probes multiple connection targets concurrently with bounded concurrency.
pub async fn probe_connections_batch(
    targets: Vec<(String, String, u16)>,
    timeout: Duration,
    concurrency: usize,
) -> Vec<(String, HostStatus)> {
    let count = targets.len();
    let (tx, rx) = async_channel::unbounded::<(String, HostStatus)>();
    spawn_batch_probe(targets, timeout, concurrency, tx);

    let mut results = Vec::with_capacity(count);
    while let Ok(res) = rx.recv().await {
        results.push(res);
    }
    results
}
