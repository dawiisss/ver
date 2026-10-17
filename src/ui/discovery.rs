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
use crate::network::{local_ipv4_subnets, Ipv4Subnet};

enum ScanEvent {
    Service(DiscoveredService),
    Progress(usize, usize),
    Complete,
}

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

    // Keep compatibility with Rust 1.75, before integer is_multiple_of was stabilized.
    #[allow(clippy::manual_is_multiple_of)]
    pub fn build_window<F, C>(
        parent: Option<&impl IsA<gtk::Window>>,
        is_already_added: C,
        on_add_callback: F,
    ) -> adw::Window
    where
        F: Fn(Connection) -> bool + 'static,
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
                "Choose a network or enter a CIDR range",
            ))
            .build();

        let refresh_btn = gtk::Button::builder()
            .icon_name("view-refresh-symbolic")
            .tooltip_text("Rescan Subnet")
            .build();
        header_bar.pack_end(&refresh_btn);
        let cancel_btn = gtk::Button::builder().label("Cancel Scan").build();
        header_bar.pack_end(&cancel_btn);

        let spinner = gtk::Spinner::builder()
            .spinning(true)
            .width_request(24)
            .height_request(24)
            .build();

        let status_label = gtk::Label::builder()
            .label("Scanning local network for VNC, RDP, SSH hosts...")
            .wrap(true)
            .hexpand(true)
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

        let (choices, description) = match local_ipv4_subnets() {
            Ok(choices) => (
                choices,
                "Scan up to 4096 hosts per range. Choose a smaller CIDR for larger networks."
                    .to_string(),
            ),
            Err(error) => (
                Vec::new(),
                format!("{}. Enter a CIDR range manually.", error),
            ),
        };
        let labels: Vec<String> = choices
            .iter()
            .map(|(name, subnet)| format!("{} — {}", name, subnet))
            .collect();
        let labels_ref: Vec<&str> = labels.iter().map(String::as_str).collect();
        let interface_row = adw::ComboRow::builder()
            .title("Network Interface")
            .model(&gtk::StringList::new(&labels_ref))
            .build();
        interface_row.set_sensitive(!choices.is_empty());
        let subnet_entry = adw::EntryRow::builder()
            .title("Subnet (IPv4 CIDR)")
            .text(
                choices
                    .first()
                    .map(|(_, subnet)| subnet.to_string())
                    .unwrap_or_default(),
            )
            .build();
        let subnet_for_selection = subnet_entry.clone();
        interface_row.connect_selected_notify(move |row| {
            if let Some((_, subnet)) = choices.get(row.selected() as usize) {
                subnet_for_selection.set_text(&subnet.to_string());
            }
        });
        let network_group = adw::PreferencesGroup::builder()
            .description(&description)
            .margin_start(12)
            .margin_end(12)
            .build();
        network_group.add(&interface_row);
        network_group.add(&subnet_entry);
        let main_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        main_box.append(&header_bar);
        main_box.append(&network_group);
        main_box.append(&status_box);
        main_box.append(&scrolled);

        window.set_content(Some(&main_box));

        let on_add = Rc::new(on_add_callback);
        let is_already_added = Rc::new(is_already_added);
        let locally_added: Rc<RefCell<Vec<DiscoveredService>>> = Rc::new(RefCell::new(Vec::new()));
        let cancel_for_scan = cancel_btn.clone();

        let start_scan = move |list_box: gtk::ListBox,
                               spinner: gtk::Spinner,
                               status_label: gtk::Label,
                               on_add: Rc<F>,
                               is_already_added: Rc<C>,
                               locally_added: Rc<RefCell<Vec<DiscoveredService>>>,
                               hosts: Vec<std::net::Ipv4Addr>|
              -> Arc<std::sync::atomic::AtomicBool> {
            let cancel_token = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let cancel_token_rx = cancel_token.clone();
            let cancel_token_thread = cancel_token.clone();

            // Clear existing list items
            while let Some(child) = list_box.first_child() {
                list_box.remove(&child);
            }

            spinner.set_spinning(true);
            cancel_for_scan.set_sensitive(true);
            status_label.set_text("Scanning local network for VNC, RDP, SSH hosts...");

            let (sender, receiver) = async_channel::bounded::<ScanEvent>(64);

            let spinner_local = spinner.clone();
            let status_label_local = status_label.clone();
            let list_box_local = list_box.clone();
            let on_add_local = on_add.clone();
            let is_already_added_local = is_already_added.clone();
            let locally_added_local = locally_added.clone();
            let cancel_local = cancel_for_scan.clone();

            glib::MainContext::default().spawn_local(async move {
                let mut found_count = 0usize;
                let mut skipped_count = 0usize;
                let mut completed = 0usize;

                while let Ok(msg) = receiver.recv().await {
                    if cancel_token_rx.load(std::sync::atomic::Ordering::SeqCst) {
                        break;
                    }

                    match msg {
                        ScanEvent::Service(service) => {
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
                                .use_markup(false)
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

                                if !on_add_clone(conn) {
                                    status_label_for_add.set_text("Could not save this device. Retry after resolving the save error.");
                                    return;
                                }
                                locally_added_for_add.borrow_mut().push(service_clone.clone());

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
                        ScanEvent::Progress(done, total) => {
                            completed = completed.max(done);
                            status_label_local.set_text(&format!("Scanning hosts: {}/{} · {} new services", completed, total, found_count));
                        }
                        ScanEvent::Complete => {
                            spinner_local.set_spinning(false);
                            cancel_local.set_sensitive(false);
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
                            break;
                        }
                    }
                }
            });

            // Spawn scanner thread pool
            thread::spawn(move || {
                let mut targets = hosts;
                targets.push(std::net::Ipv4Addr::LOCALHOST);
                targets.sort_unstable();
                targets.dedup();
                let total = targets.len();
                let completed = Arc::new(std::sync::atomic::AtomicUsize::new(0));
                let (host_tx, host_rx) = async_channel::unbounded::<std::net::Ipv4Addr>();
                for host in targets {
                    let _ = host_tx.send_blocking(host);
                }
                drop(host_tx);
                let mut handles = vec![];
                for _ in 0..16 {
                    let host_rx = host_rx.clone();
                    let sender = sender.clone();
                    let cancel = cancel_token_thread.clone();
                    let completed = completed.clone();
                    handles.push(thread::spawn(move || {
                        while let Ok(ip) = host_rx.recv_blocking() {
                            for (port, protocol) in [(5900, "vnc"), (3389, "rdp"), (22, "ssh")] {
                                if cancel.load(std::sync::atomic::Ordering::SeqCst) {
                                    return;
                                }
                                let addr = SocketAddr::new(IpAddr::V4(ip), port);
                                if TcpStream::connect_timeout(&addr, Duration::from_millis(150))
                                    .is_ok()
                                {
                                    let service = DiscoveredService {
                                        name: format!("Host {} ({})", ip, protocol.to_uppercase()),
                                        protocol: protocol.into(),
                                        host: ip.to_string(),
                                        port,
                                    };
                                    if sender.send_blocking(ScanEvent::Service(service)).is_err() {
                                        return;
                                    }
                                }
                            }
                            let done =
                                completed.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                            if (done % 16 == 0 || done == total)
                                && sender
                                    .send_blocking(ScanEvent::Progress(done, total))
                                    .is_err()
                            {
                                return;
                            }
                        }
                    }));
                }
                for handle in handles {
                    let _ = handle.join();
                }
                let _ = sender.send_blocking(ScanEvent::Complete);
            });

            cancel_token
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
            let current_cancel = current_cancel_token.clone();
            let subnet_entry = subnet_entry.clone();
            let cancel = cancel_btn.clone();

            move || {
                // Cancel any ongoing scan first
                if let Some(prev_token) = current_cancel.lock().unwrap().take() {
                    prev_token.store(true, std::sync::atomic::Ordering::SeqCst);
                }
                let hosts = match subnet_entry
                    .text()
                    .parse::<Ipv4Subnet>()
                    .and_then(Ipv4Subnet::hosts)
                {
                    Ok(hosts) => hosts,
                    Err(error) => {
                        cancel.set_sensitive(false);
                        spinner.set_spinning(false);
                        status_label.set_text(&error);
                        return;
                    }
                };
                let token = start_scan(
                    list_box.clone(),
                    spinner.clone(),
                    status_label.clone(),
                    on_add.clone(),
                    is_already_added.clone(),
                    locally_added.clone(),
                    hosts,
                );
                *current_cancel.lock().unwrap() = Some(token);
            }
        };

        {
            let token = current_cancel_token.clone();
            let spinner = spinner.clone();
            let status = status_label.clone();
            cancel_btn.connect_clicked(move |button| {
                button.set_sensitive(false);
                if let Some(token) = token.lock().unwrap().take() {
                    token.store(true, std::sync::atomic::Ordering::SeqCst);
                }
                spinner.set_spinning(false);
                status.set_text("Scan cancelled. Discovered services are still available.");
            });
        }
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
