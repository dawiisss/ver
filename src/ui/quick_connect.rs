use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;

use crate::models::{AdvancedSettings, Connection, Protocol};

/// Parses a quick connect URI or shorthand string into a `Connection`.
pub fn parse_quick_connect(input: &str, default_proto: Protocol) -> Result<Connection, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Connection target cannot be empty".to_string());
    }

    let mut protocol = default_proto;
    let mut target_str = trimmed;

    // Check for URI scheme (e.g. ssh://, rdp://, vnc://, spice://, xrdp://)
    if let Some((scheme, rest)) = trimmed.split_once("://") {
        protocol = match scheme.to_lowercase().as_str() {
            "ssh" => Protocol::Ssh,
            "rdp" => Protocol::Rdp,
            "xrdp" => Protocol::Xrdp,
            "vnc" => Protocol::Vnc,
            "spice" => Protocol::Spice,
            other => return Err(format!("Unsupported protocol scheme: {}", other)),
        };
        target_str = rest;
    } else if let Some((first_word, _)) = trimmed.split_once(char::is_whitespace) {
        // e.g. "ssh user@host"
        if let Some(p) = match first_word.to_lowercase().as_str() {
            "ssh" => Some(Protocol::Ssh),
            "rdp" => Some(Protocol::Rdp),
            "xrdp" => Some(Protocol::Xrdp),
            "vnc" => Some(Protocol::Vnc),
            "spice" => Some(Protocol::Spice),
            _ => None,
        } {
            protocol = p;
            target_str = trimmed[first_word.len()..].trim();
        }
    }

    let mut username = String::new();
    let mut host_part = target_str;

    // Check for user@host
    if let Some((u, h)) = target_str.split_once('@') {
        username = u.to_string();
        host_part = h;
    }

    // Parse host and port
    let (host, port) = parse_host_and_port(host_part, protocol.default_port())?;

    if host.is_empty() {
        return Err("Host cannot be empty".to_string());
    }

    let name = if !username.is_empty() {
        format!("{}@{}", username, host)
    } else {
        host.clone()
    };

    Ok(Connection {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        protocol,
        host,
        port,
        username,
        mac_address: String::new(),
        group: "Quick Connect".to_string(),
        advanced_settings: AdvancedSettings::default(),
    })
}

fn parse_host_and_port(input: &str, default_port: u16) -> Result<(String, u16), String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok((String::new(), default_port));
    }

    // IPv6 enclosed in brackets: [fe80::1]:22
    if trimmed.starts_with('[') {
        if let Some(close_idx) = trimmed.find(']') {
            let host = trimmed[1..close_idx].to_string();
            let remainder = &trimmed[close_idx + 1..];
            if let Some(port_str) = remainder.strip_prefix(':') {
                let port = port_str
                    .parse::<u16>()
                    .map_err(|_| format!("Invalid port number: {}", port_str))?;
                return Ok((host, port));
            }
            return Ok((host, default_port));
        }
    }

    // Standard host:port
    if let Some((h, p)) = trimmed.rsplit_once(':') {
        if !h.contains(':') {
            let port = p
                .parse::<u16>()
                .map_err(|_| format!("Invalid port number: {}", p))?;
            return Ok((h.to_string(), port));
        }
    }

    Ok((trimmed.to_string(), default_port))
}

pub struct QuickConnectDialog;

impl QuickConnectDialog {
    pub fn show<FConnect, FSaveConnect>(
        parent: &impl IsA<gtk::Window>,
        default_proto: Protocol,
        on_connect: FConnect,
        on_save_connect: FSaveConnect,
    ) where
        FConnect: Fn(Connection, Option<String>) + 'static,
        FSaveConnect: Fn(Connection, Option<String>) + 'static,
    {
        let window = adw::Window::builder()
            .transient_for(parent)
            .modal(true)
            .title("Quick Connect")
            .default_width(460)
            .default_height(480)
            .build();

        let header_bar = adw::HeaderBar::new();
        let title = adw::WindowTitle::new("Quick Connect", "Connect immediately or save");
        header_bar.set_title_widget(Some(&title));

        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&header_bar);

        let content_box = gtk::Box::new(gtk::Orientation::Vertical, 16);
        content_box.set_margin_top(16);
        content_box.set_margin_bottom(16);
        content_box.set_margin_start(16);
        content_box.set_margin_end(16);

        // Quick URI Entry Group
        let quick_group = adw::PreferencesGroup::builder()
            .title("Connection Target")
            .description("Enter hostname, IP, or URI (e.g. ssh://user@server:22 or 192.168.1.100)")
            .build();

        let uri_entry = adw::EntryRow::builder()
            .title("Target Address")
            .show_apply_button(false)
            .build();
        quick_group.add(&uri_entry);

        // Details Group
        let details_group = adw::PreferencesGroup::builder()
            .title("Connection Details")
            .build();

        let proto_model = gtk::StringList::new(&["RDP", "VNC", "SPICE", "SSH", "XRDP"]);
        let proto_row = adw::ComboRow::builder()
            .title("Protocol")
            .model(&proto_model)
            .build();

        let default_idx = match default_proto {
            Protocol::Rdp => 0,
            Protocol::Vnc => 1,
            Protocol::Spice => 2,
            Protocol::Ssh => 3,
            Protocol::Xrdp => 4,
        };
        proto_row.set_selected(default_idx);

        let host_row = adw::EntryRow::builder().title("Host").build();
        let port_row = adw::SpinRow::builder()
            .title("Port")
            .adjustment(&gtk::Adjustment::new(
                default_proto.default_port() as f64,
                1.0,
                65535.0,
                1.0,
                10.0,
                0.0,
            ))
            .build();
        let user_row = adw::EntryRow::builder().title("Username").build();
        let pass_row = adw::PasswordEntryRow::builder().title("Password").build();

        details_group.add(&proto_row);
        details_group.add(&host_row);
        details_group.add(&port_row);
        details_group.add(&user_row);
        details_group.add(&pass_row);

        // Button bar
        let button_box = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        button_box.set_halign(gtk::Align::End);
        button_box.set_margin_top(8);

        let cancel_btn = gtk::Button::builder().label("Cancel").build();

        let save_connect_btn = gtk::Button::builder()
            .label("Save & Connect")
            .css_classes(vec!["pill"])
            .build();

        let connect_btn = gtk::Button::builder()
            .label("Connect")
            .css_classes(vec!["suggested-action", "pill"])
            .build();

        button_box.append(&cancel_btn);
        button_box.append(&save_connect_btn);
        button_box.append(&connect_btn);

        let clamp = adw::Clamp::builder()
            .maximum_size(520)
            .child(&content_box)
            .build();

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .child(&clamp)
            .build();

        content_box.append(&quick_group);
        content_box.append(&details_group);
        content_box.append(&button_box);

        toolbar_view.set_content(Some(&scrolled));
        window.set_content(Some(&toolbar_view));

        // Connect URI live parsing
        let updating = Rc::new(RefCell::new(false));
        let updating_clone = updating.clone();
        let host_row_clone = host_row.clone();
        let port_row_clone = port_row.clone();
        let user_row_clone = user_row.clone();
        let proto_row_clone = proto_row.clone();

        uri_entry.connect_changed(move |entry| {
            if *updating_clone.borrow() {
                return;
            }
            let text = entry.text();
            let selected_proto = match proto_row_clone.selected() {
                0 => Protocol::Rdp,
                1 => Protocol::Vnc,
                2 => Protocol::Spice,
                3 => Protocol::Ssh,
                _ => Protocol::Xrdp,
            };

            if let Ok(conn) = parse_quick_connect(&text, selected_proto) {
                *updating_clone.borrow_mut() = true;
                let proto_idx = match conn.protocol {
                    Protocol::Rdp => 0,
                    Protocol::Vnc => 1,
                    Protocol::Spice => 2,
                    Protocol::Ssh => 3,
                    Protocol::Xrdp => 4,
                };
                proto_row_clone.set_selected(proto_idx);
                host_row_clone.set_text(&conn.host);
                port_row_clone.set_value(conn.port as f64);
                user_row_clone.set_text(&conn.username);
                *updating_clone.borrow_mut() = false;
            }
        });

        // Cancel
        let win_weak = window.downgrade();
        cancel_btn.connect_clicked(move |_| {
            if let Some(win) = win_weak.upgrade() {
                win.close();
            }
        });

        // Helper to construct connection from fields
        let get_connection = {
            let host_row = host_row.clone();
            let port_row = port_row.clone();
            let user_row = user_row.clone();
            let proto_row = proto_row.clone();
            let pass_row = pass_row.clone();

            move || -> Result<(Connection, Option<String>), String> {
                let host = host_row.text().trim().to_string();
                if host.is_empty() {
                    return Err("Host is required".to_string());
                }

                let protocol = match proto_row.selected() {
                    0 => Protocol::Rdp,
                    1 => Protocol::Vnc,
                    2 => Protocol::Spice,
                    3 => Protocol::Ssh,
                    _ => Protocol::Xrdp,
                };

                let port = port_row.value() as u16;
                let username = user_row.text().trim().to_string();
                let password_text = pass_row.text().trim().to_string();
                let password = if password_text.is_empty() {
                    None
                } else {
                    Some(password_text)
                };

                let name = if !username.is_empty() {
                    format!("{}@{}", username, host)
                } else {
                    host.clone()
                };

                let conn = Connection {
                    id: uuid::Uuid::new_v4().to_string(),
                    name,
                    protocol,
                    host,
                    port,
                    username,
                    mac_address: String::new(),
                    group: "Quick Connect".to_string(),
                    advanced_settings: AdvancedSettings::default(),
                };

                Ok((conn, password))
            }
        };

        // Connect Now
        let on_connect = Rc::new(on_connect);
        let on_connect_clone = on_connect.clone();
        let get_conn_connect = get_connection.clone();
        let win_weak_conn = window.downgrade();
        connect_btn.connect_clicked(move |_| match get_conn_connect() {
            Ok((conn, pass)) => {
                if let Some(win) = win_weak_conn.upgrade() {
                    win.close();
                }
                on_connect_clone(conn, pass);
            }
            Err(e) => {
                eprintln!("Quick Connect validation error: {}", e);
            }
        });

        // Save & Connect
        let on_save_connect = Rc::new(on_save_connect);
        let win_weak_save = window.downgrade();
        save_connect_btn.connect_clicked(move |_| match get_connection() {
            Ok((conn, pass)) => {
                if let Some(win) = win_weak_save.upgrade() {
                    win.close();
                }
                on_save_connect(conn, pass);
            }
            Err(e) => {
                eprintln!("Quick Connect validation error: {}", e);
            }
        });

        window.present();
    }
}
