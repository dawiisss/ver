use local_ip_address::local_ip;
use std::cell::RefCell;
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::rc::Rc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use gtk::glib;
use gtk::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;

use crate::models::{Connection, Protocol};

#[derive(Debug, Clone)]
pub struct DiscoveredService {
    pub name: String,
    pub protocol: String,
    pub host: String,
    pub port: u16,
}

#[derive(Default)]
pub struct DiscoveryDialog {
    pub discovered_services: Vec<DiscoveredService>,
}

impl DiscoveryDialog {
    pub fn new() -> Self {
        Self {
            discovered_services: Vec::new(),
        }
    }

    pub fn add_service(&mut self, service: DiscoveredService) {
        self.discovered_services.push(service);
    }

    pub fn is_service_in_connections(
        service: &DiscoveredService,
        connections: &[Connection],
    ) -> bool {
        connections
            .iter()
            .any(|conn| Self::matches_connection(service, conn))
    }

    pub fn filter_unadded(&self, connections: &[Connection]) -> Vec<DiscoveredService> {
        self.discovered_services
            .iter()
            .filter(|s| !Self::is_service_in_connections(s, connections))
            .cloned()
            .collect()
    }

    pub fn matches_connection(service: &DiscoveredService, conn: &Connection) -> bool {
        let (conn_host, conn_port) =
            crate::importers::parse_host_port(&conn.host, conn.resolve_port());
        let (serv_host, serv_port) = crate::importers::parse_host_port(&service.host, service.port);

        let network_match = Self::protocol_matches(conn.protocol, &service.protocol)
            && conn_port == serv_port
            && Self::hosts_match(&conn_host, &serv_host);

        let name_match = !conn.name.trim().is_empty()
            && !service.name.trim().is_empty()
            && conn.name.trim().eq_ignore_ascii_case(service.name.trim())
            && Self::protocol_matches(conn.protocol, &service.protocol);

        network_match || name_match
    }

    pub fn services_match(s1: &DiscoveredService, s2: &DiscoveredService) -> bool {
        let (h1, p1) = crate::importers::parse_host_port(&s1.host, s1.port);
        let (h2, p2) = crate::importers::parse_host_port(&s2.host, s2.port);

        s1.protocol.trim().eq_ignore_ascii_case(s2.protocol.trim())
            && p1 == p2
            && Self::hosts_match(&h1, &h2)
    }

    pub fn protocol_matches(conn_proto: Protocol, service_proto: &str) -> bool {
        let s = service_proto.trim().to_ascii_lowercase();
        match conn_proto {
            Protocol::Rdp | Protocol::Xrdp => s == "rdp" || s == "xrdp",
            Protocol::Vnc => s == "vnc",
            Protocol::Ssh => s == "ssh",
            Protocol::Spice => s == "spice",
        }
    }

    pub fn hosts_match(h1: &str, h2: &str) -> bool {
        let t1 = h1.trim();
        let t2 = h2.trim();
        if t1.is_empty() || t2.is_empty() {
            return false;
        }
        if t1.eq_ignore_ascii_case(t2) {
            return true;
        }
        if Self::is_loopback(t1) && Self::is_loopback(t2) {
            return true;
        }
        if let (Ok(ip1), Ok(ip2)) = (t1.parse::<IpAddr>(), t2.parse::<IpAddr>()) {
            return ip1 == ip2;
        }
        false
    }

    fn is_loopback(h: &str) -> bool {
        let t = h.trim();
        if t.eq_ignore_ascii_case("localhost") {
            return true;
        }
        if let Ok(ip) = t.parse::<IpAddr>() {
            return ip.is_loopback() || ip.is_unspecified();
        }
        false
    }

    pub fn build_window<F, C>(
        parent: Option<&impl IsA<gtk::Window>>,
        is_already_added: C,
        on_add_callback: F,
    ) -> adw::Window
    where
        F: Fn(Connection) + 'static,
        C: Fn(&DiscoveredService) -> bool + 'static,
    {
        let window = adw::Window::builder()
            .title("Discover Network Devices")
            .modal(true)
            .default_width(480)
            .default_height(560)
            .build();

        if let Some(p) = parent {
            window.set_transient_for(Some(p));
        }

        let header_bar = adw::HeaderBar::builder()
            .title_widget(&adw::WindowTitle::new(
                "Network Discovery",
                "Scanning local subnet",
            ))
            .build();

        let refresh_btn = gtk::Button::builder()
            .icon_name("view-refresh-symbolic")
            .tooltip_text("Rescan Subnet")
            .build();
        header_bar.pack_end(&refresh_btn);

        let spinner = gtk::Spinner::builder()
            .spinning(true)
            .width_request(24)
            .height_request(24)
            .build();

        let status_label = gtk::Label::builder()
            .label("Scanning local network for VNC, RDP, SSH hosts...")
            .css_classes(vec!["dim-label"])
            .build();

        let status_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        status_box.set_margin_start(12);
        status_box.set_margin_end(12);
        status_box.set_margin_top(12);
        status_box.set_margin_bottom(12);
        status_box.append(&spinner);
        status_box.append(&status_label);

        let list_box = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .css_classes(vec!["boxed-list"])
            .build();

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .vexpand(true)
            .child(&list_box)
            .build();

        let main_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        main_box.append(&header_bar);
        main_box.append(&status_box);
        main_box.append(&scrolled);

        window.set_content(Some(&main_box));

        let on_add = Rc::new(on_add_callback);
        let is_already_added = Rc::new(is_already_added);
        let locally_added: Rc<RefCell<Vec<DiscoveredService>>> = Rc::new(RefCell::new(Vec::new()));
        let is_scanning = Arc::new(std::sync::atomic::AtomicBool::new(false));

        let start_scan = move |list_box: gtk::ListBox,
                               spinner: gtk::Spinner,
                               status_label: gtk::Label,
                               on_add: Rc<F>,
                               is_already_added: Rc<C>,
                               locally_added: Rc<RefCell<Vec<DiscoveredService>>>,
                               is_scanning: Arc<std::sync::atomic::AtomicBool>|
              -> Option<Arc<std::sync::atomic::AtomicBool>> {
            if is_scanning
                .compare_exchange(
                    false,
                    true,
                    std::sync::atomic::Ordering::SeqCst,
                    std::sync::atomic::Ordering::SeqCst,
                )
                .is_err()
            {
                return None;
            }

            // Clear existing list items
            while let Some(child) = list_box.first_child() {
                list_box.remove(&child);
            }

            spinner.set_spinning(true);
            status_label.set_text("Scanning local network for VNC, RDP, SSH hosts...");

            let (sender, receiver) = async_channel::unbounded::<Option<DiscoveredService>>();

            let is_scanning_receiver = is_scanning.clone();
            let spinner_local = spinner.clone();
            let status_label_local = status_label.clone();
            let list_box_local = list_box.clone();
            let on_add_local = on_add.clone();
            let is_already_added_local = is_already_added.clone();
            let locally_added_local = locally_added.clone();

            glib::MainContext::default().spawn_local(async move {
                let mut found_count = 0usize;
                let mut skipped_count = 0usize;

                while let Ok(msg) = receiver.recv().await {
                    match msg {
                        Some(service) => {
                            let already_in_connections = is_already_added_local(&service);
                            let already_in_local = locally_added_local
                                .borrow()
                                .iter()
                                .any(|s| DiscoveryDialog::services_match(s, &service));

                            if already_in_connections || already_in_local {
                                skipped_count += 1;
                                continue;
                            }

                            found_count += 1;
                            let row = adw::ActionRow::builder()
                                .title(&service.name)
                                .subtitle(format!(
                                    "{}:{} ({})",
                                    service.host,
                                    service.port,
                                    service.protocol.to_uppercase()
                                ))
                                .build();

                            let icon_name = match service.protocol.to_lowercase().as_str() {
                                "vnc" => "computer-symbolic",
                                "ssh" => "utilities-terminal-symbolic",
                                "rdp" => "video-display-symbolic",
                                _ => "display-symbolic",
                            };
                            let icon = gtk::Image::from_icon_name(icon_name);
                            row.add_prefix(&icon);

                            let add_btn = gtk::Button::builder()
                                .label("Add")
                                .css_classes(vec!["suggested-action"])
                                .valign(gtk::Align::Center)
                                .build();

                            let service_clone = service.clone();
                            let on_add_clone = on_add_local.clone();
                            let list_box_for_add = list_box_local.clone();
                            let row_for_add = row.clone();
                            let status_label_for_add = status_label_local.clone();
                            let locally_added_for_add = locally_added_local.clone();

                            add_btn.connect_clicked(move |_btn| {
                                let proto = match service_clone.protocol.to_lowercase().as_str() {
                                    "vnc" => Protocol::Vnc,
                                    "ssh" => Protocol::Ssh,
                                    _ => Protocol::Rdp,
                                };
                                let mut conn = Connection::new_with_protocol(proto);
                                conn.name = service_clone.name.clone();
                                conn.host = service_clone.host.clone();
                                conn.port = service_clone.port;
                                conn.group = "Discovered".to_string();

                                locally_added_for_add.borrow_mut().push(service_clone.clone());
                                on_add_clone(conn);

                                list_box_for_add.remove(&row_for_add);
                                if list_box_for_add.first_child().is_none() {
                                    status_label_for_add.set_text(
                                        "All discovered devices have been added to your connections.",
                                    );
                                } else {
                                    status_label_for_add.set_text(&format!(
                                        "Added '{}' to connections.",
                                        service_clone.name
                                    ));
                                }
                            });

                            row.add_suffix(&add_btn);
                            list_box_local.append(&row);
                        }
                        None => {
                            spinner_local.set_spinning(false);
                            if found_count == 0 {
                                if skipped_count > 0 {
                                    let msg = if skipped_count == 1 {
                                        "Scan complete. 1 discovered device is already in your connections.".to_string()
                                    } else {
                                        format!(
                                            "Scan complete. {} discovered devices are already in your connections.",
                                            skipped_count
                                        )
                                    };
                                    status_label_local.set_text(&msg);
                                } else {
                                    status_label_local.set_text(
                                        "Scan complete. No remote desktop or SSH devices found.",
                                    );
                                }
                            } else if skipped_count > 0 {
                                status_label_local.set_text(&format!(
                                    "Scan complete. Found {} new device{} ({} already in connections).",
                                    found_count,
                                    if found_count == 1 { "" } else { "s" },
                                    skipped_count
                                ));
                            } else {
                                status_label_local.set_text(&format!(
                                    "Scan complete. Found {} new device{}.",
                                    found_count,
                                    if found_count == 1 { "" } else { "s" }
                                ));
                            }
                            is_scanning_receiver.store(false, std::sync::atomic::Ordering::SeqCst);
                            break;
                        }
                    }
                }
            });

            let cancel_token = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let cancel_token_thread = cancel_token.clone();

            // Spawn scanner thread pool
            thread::spawn(move || {
                let targets = vec![("localhost", "127.0.0.1")];

                // Standard ports to probe: (Port, Protocol Name)
                let ports: &[(u16, &str)] = &[(5900, "vnc"), (3389, "rdp"), (22, "ssh")];

                // Probe local targets
                for (name, ip_str) in targets {
                    if cancel_token_thread.load(std::sync::atomic::Ordering::SeqCst) {
                        let _ = sender.send_blocking(None);
                        return;
                    }
                    if let Ok(ip) = ip_str.parse::<IpAddr>() {
                        for &(port, proto) in ports {
                            let addr = SocketAddr::new(ip, port);
                            if TcpStream::connect_timeout(&addr, Duration::from_millis(150)).is_ok()
                            {
                                let service = DiscoveredService {
                                    name: format!("{} ({})", name, proto.to_uppercase()),
                                    protocol: proto.to_string(),
                                    host: ip_str.to_string(),
                                    port,
                                };
                                let _ = sender.send_blocking(Some(service));
                            }
                        }
                    }
                }

                // Subnet sweep
                let mut subnet_prefix = "192.168.1".to_string(); // Fallback
                if let Ok(my_ip) = local_ip() {
                    match my_ip {
                        IpAddr::V4(ipv4) => {
                            let octets = ipv4.octets();
                            subnet_prefix = format!("{}.{}.{}", octets[0], octets[1], octets[2]);
                        }
                        IpAddr::V6(_) => {}
                    }
                }

                let (host_tx, host_rx) = async_channel::unbounded::<u8>();
                for host_id in 1..=254 {
                    let _ = host_tx.send_blocking(host_id);
                }
                drop(host_tx);

                let num_workers = 16;
                let mut handles = vec![];
                for _ in 0..num_workers {
                    let host_rx_clone = host_rx.clone();
                    let prefix = subnet_prefix.clone();
                    let sender_clone = sender.clone();
                    let cancel_worker = cancel_token_thread.clone();

                    let handle = thread::spawn(move || {
                        while let Ok(host_id) = host_rx_clone.recv_blocking() {
                            if cancel_worker.load(std::sync::atomic::Ordering::SeqCst) {
                                break;
                            }
                            let ip_str = format!("{}.{}", prefix, host_id);
                            if let Ok(ip) = ip_str.parse::<IpAddr>() {
                                let ports: &[(u16, &str)] =
                                    &[(5900, "vnc"), (3389, "rdp"), (22, "ssh")];
                                for &(port, proto) in ports {
                                    if cancel_worker.load(std::sync::atomic::Ordering::SeqCst) {
                                        break;
                                    }
                                    let addr = SocketAddr::new(ip, port);
                                    if TcpStream::connect_timeout(&addr, Duration::from_millis(150))
                                        .is_ok()
                                    {
                                        let service = DiscoveredService {
                                            name: format!(
                                                "Host {} ({})",
                                                ip_str,
                                                proto.to_uppercase()
                                            ),
                                            protocol: proto.to_string(),
                                            host: ip_str.clone(),
                                            port,
                                        };
                                        let _ = sender_clone.send_blocking(Some(service));
                                    }
                                }
                            }
                        }
                    });
                    handles.push(handle);
                }

                for handle in handles {
                    let _ = handle.join();
                }

                let _ = sender.send_blocking(None);
            });

            Some(cancel_token)
        };

        let current_cancel_token: Arc<
            std::sync::Mutex<Option<Arc<std::sync::atomic::AtomicBool>>>,
        > = Arc::new(std::sync::Mutex::new(None));

        let run_scan = {
            let list_box = list_box.clone();
            let spinner = spinner.clone();
            let status_label = status_label.clone();
            let on_add = on_add.clone();
            let is_already_added = is_already_added.clone();
            let locally_added = locally_added.clone();
            let is_scanning = is_scanning.clone();
            let current_cancel = current_cancel_token.clone();

            move || {
                // Cancel any ongoing scan first
                if let Some(prev_token) = current_cancel.lock().unwrap().take() {
                    prev_token.store(true, std::sync::atomic::Ordering::SeqCst);
                }
                let token = start_scan(
                    list_box.clone(),
                    spinner.clone(),
                    status_label.clone(),
                    on_add.clone(),
                    is_already_added.clone(),
                    locally_added.clone(),
                    is_scanning.clone(),
                );
                *current_cancel.lock().unwrap() = token;
            }
        };

        run_scan();

        {
            let do_scan = run_scan.clone();
            refresh_btn.connect_clicked(move |_| {
                do_scan();
            });
        }

        {
            let cancel_on_close = current_cancel_token.clone();
            window.connect_close_request(move |_| {
                if let Some(token) = cancel_on_close.lock().unwrap().take() {
                    token.store(true, std::sync::atomic::Ordering::SeqCst);
                }
                glib::Propagation::Proceed
            });
        }

        window
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Connection, Protocol};

    #[test]
    fn test_service_matching_ssh() {
        let mut conn = Connection::new_with_protocol(Protocol::Ssh);
        conn.host = "192.168.1.100".to_string();
        conn.port = 22;

        let service = DiscoveredService {
            name: "Host 192.168.1.100 (SSH)".to_string(),
            protocol: "ssh".to_string(),
            host: "192.168.1.100".to_string(),
            port: 22,
        };

        assert!(DiscoveryDialog::is_service_in_connections(
            &service,
            &[conn]
        ));
    }

    #[test]
    fn test_service_matching_rdp_and_xrdp() {
        let mut conn = Connection::new_with_protocol(Protocol::Xrdp);
        conn.host = "192.168.1.105".to_string();
        conn.port = 3389;

        let service = DiscoveredService {
            name: "Host 192.168.1.105 (RDP)".to_string(),
            protocol: "rdp".to_string(),
            host: "192.168.1.105".to_string(),
            port: 3389,
        };

        assert!(DiscoveryDialog::is_service_in_connections(
            &service,
            &[conn]
        ));
    }

    #[test]
    fn test_service_matching_localhost_equivalence() {
        let mut conn = Connection::new_with_protocol(Protocol::Vnc);
        conn.host = "localhost".to_string();
        conn.port = 5900;

        let service = DiscoveredService {
            name: "localhost (VNC)".to_string(),
            protocol: "vnc".to_string(),
            host: "127.0.0.1".to_string(),
            port: 5900,
        };

        assert!(DiscoveryDialog::is_service_in_connections(
            &service,
            &[conn]
        ));
    }

    #[test]
    fn test_service_not_matching_different_port_or_proto() {
        let mut conn = Connection::new_with_protocol(Protocol::Ssh);
        conn.host = "192.168.1.100".to_string();
        conn.port = 22;

        let service_diff_port = DiscoveredService {
            name: "Host 192.168.1.100 (SSH)".to_string(),
            protocol: "ssh".to_string(),
            host: "192.168.1.100".to_string(),
            port: 2222,
        };

        let service_diff_proto = DiscoveredService {
            name: "Host 192.168.1.100 (VNC)".to_string(),
            protocol: "vnc".to_string(),
            host: "192.168.1.100".to_string(),
            port: 5900,
        };

        let service_diff_host = DiscoveredService {
            name: "Host 192.168.1.101 (SSH)".to_string(),
            protocol: "ssh".to_string(),
            host: "192.168.1.101".to_string(),
            port: 22,
        };

        assert!(!DiscoveryDialog::is_service_in_connections(
            &service_diff_port,
            &[conn.clone()]
        ));
        assert!(!DiscoveryDialog::is_service_in_connections(
            &service_diff_proto,
            &[conn.clone()]
        ));
        assert!(!DiscoveryDialog::is_service_in_connections(
            &service_diff_host,
            &[conn]
        ));
    }

    #[test]
    fn test_filter_unadded_services() {
        let mut conn = Connection::new_with_protocol(Protocol::Ssh);
        conn.host = "192.168.1.10".to_string();
        conn.port = 22;

        let mut dialog = DiscoveryDialog::new();
        dialog.add_service(DiscoveredService {
            name: "Host 192.168.1.10 (SSH)".to_string(),
            protocol: "ssh".to_string(),
            host: "192.168.1.10".to_string(),
            port: 22,
        });
        dialog.add_service(DiscoveredService {
            name: "Host 192.168.1.20 (VNC)".to_string(),
            protocol: "vnc".to_string(),
            host: "192.168.1.20".to_string(),
            port: 5900,
        });

        let unadded = dialog.filter_unadded(&[conn]);
        assert_eq!(unadded.len(), 1);
        assert_eq!(unadded[0].host, "192.168.1.20");
        assert_eq!(unadded[0].port, 5900);
    }
}
