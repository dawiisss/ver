use std::net::{IpAddr, SocketAddr, TcpStream};
use local_ip_address::local_ip;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use gtk::prelude::*;
use gtk::glib;
use libadwaita::prelude::*;
use libadwaita as adw;

use crate::models::{Connection, Protocol};

#[derive(Debug, Clone)]
pub struct DiscoveredService {
    pub name: String,
    pub protocol: String,
    pub host: String,
    pub port: u16,
}

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

    pub fn build_window<F>(
        parent: Option<&impl IsA<gtk::Window>>,
        on_add_callback: F,
    ) -> adw::Window
    where
        F: Fn(Connection) + 'static,
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
            .title_widget(&adw::WindowTitle::new("Network Discovery", "Scanning local subnet"))
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

        let on_add = Arc::new(on_add_callback);

        let start_scan = move |list_box: gtk::ListBox, spinner: gtk::Spinner, status_label: gtk::Label, on_add: Arc<F>| {
            // Clear existing list items
            while let Some(child) = list_box.first_child() {
                list_box.remove(&child);
            }

            spinner.set_spinning(true);
            status_label.set_text("Scanning local network for VNC, RDP, SSH hosts...");

            #[allow(deprecated)]
            let (sender, receiver) = glib::MainContext::channel::<Option<DiscoveredService>>(glib::Priority::default());

            receiver.attach(None, move |msg| {
                match msg {
                    Some(service) => {
                        let row = adw::ActionRow::builder()
                            .title(&service.name)
                            .subtitle(format!("{}:{} ({})", service.host, service.port, service.protocol.to_uppercase()))
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
                        let on_add_clone = on_add.clone();
                        add_btn.connect_clicked(move |btn| {
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

                            on_add_clone(conn);

                            btn.set_sensitive(false);
                            btn.set_label("Added");
                        });

                        row.add_suffix(&add_btn);
                        list_box.append(&row);
                    }
                    None => {
                        spinner.set_spinning(false);
                        status_label.set_text("Scan complete.");
                    }
                }
                glib::ControlFlow::Continue
            });

            // Spawn scanner thread
            thread::spawn(move || {
                let targets = vec![
                    ("localhost", "127.0.0.1"),
                ];

                // Standard ports to probe: (Port, Protocol Name)
                let ports: &[(u16, &str)] = &[
                    (5900, "vnc"),
                    (3389, "rdp"),
                    (22, "ssh"),
                ];

                // Probe local targets (synchronously since it's just 1)
                for (name, ip_str) in targets {
                    if let Ok(ip) = ip_str.parse::<IpAddr>() {
                        for &(port, proto) in ports {
                            let addr = SocketAddr::new(ip, port);
                            if TcpStream::connect_timeout(&addr, Duration::from_millis(150)).is_ok() {
                                let service = DiscoveredService {
                                    name: format!("{} ({})", name, proto.to_uppercase()),
                                    protocol: proto.to_string(),
                                    host: ip_str.to_string(),
                                    port,
                                };
                                let _ = sender.send(Some(service));
                            }
                        }
                    }
                }

                // Subnet sweep
                let mut subnet_prefix = "192.168.1".to_string(); // Fallback
                if let Ok(my_ip) = local_ip() {
                    let ip_str = my_ip.to_string();
                    let parts: Vec<&str> = ip_str.split('.').collect();
                    if parts.len() == 4 {
                        subnet_prefix = format!("{}.{}.{}", parts[0], parts[1], parts[2]);
                    }
                }

                let mut handles = vec![];
                for host_id in 1..=254 {
                    let ip_str = format!("{}.{}", subnet_prefix, host_id);
                    let sender_clone = sender.clone();
                    
                    let handle = thread::spawn(move || {
                        if let Ok(ip) = ip_str.parse::<IpAddr>() {
                            let ports: &[(u16, &str)] = &[
                                (5900, "vnc"),
                                (3389, "rdp"),
                                (22, "ssh"),
                            ];
                            for &(port, proto) in ports {
                                let addr = SocketAddr::new(ip, port);
                                if TcpStream::connect_timeout(&addr, Duration::from_millis(150)).is_ok() {
                                    let service = DiscoveredService {
                                        name: format!("Host {} ({})", ip_str, proto.to_uppercase()),
                                        protocol: proto.to_string(),
                                        host: ip_str.clone(),
                                        port,
                                    };
                                    let _ = sender_clone.send(Some(service));
                                }
                            }
                        }
                    });
                    handles.push(handle);
                }

                for handle in handles {
                    let _ = handle.join();
                }

                let _ = sender.send(None);
            });
        };

        let list_box_clone = list_box.clone();
        let spinner_clone = spinner.clone();
        let status_label_clone = status_label.clone();
        let on_add_clone = on_add.clone();

        start_scan(list_box.clone(), spinner.clone(), status_label.clone(), on_add.clone());

        refresh_btn.connect_clicked(move |_| {
            start_scan(list_box_clone.clone(), spinner_clone.clone(), status_label_clone.clone(), on_add_clone.clone());
        });

        window
    }
}
