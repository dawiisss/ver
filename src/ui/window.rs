use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use libadwaita::prelude::*;
use libadwaita as adw;

use crate::launcher;
use crate::models::{AppConfig, Connection, Protocol, VncScaling};
use crate::network;
use crate::secrets;
use crate::storage;
use crate::ui::discovery::DiscoveryDialog;
use crate::ui::editor::ConnectionEditor;
use crate::ui::preferences::{apply_theme, PreferencesWindow};
use crate::vnc::{VncClient, VncCommand, VncSessionEvent, VncWidget};



pub struct AppWindowState {
    pub connections: Vec<Connection>,
    pub selected_id: Option<String>,
    pub search_query: String,
    pub config: AppConfig,
}

pub struct MainWindow {
    pub connections: Vec<Connection>,
    pub selected_id: Option<String>,
    pub search_filter: String,
    pub config: AppConfig,
    pub window: Option<adw::ApplicationWindow>,
}

impl MainWindow {
    pub fn new(connections: Vec<Connection>, config: AppConfig) -> Self {
        Self {
            connections,
            selected_id: None,
            search_filter: String::new(),
            config,
            window: None,
        }
    }

    pub fn filtered_connections(&self) -> Vec<&Connection> {
        if self.search_filter.is_empty() {
            self.connections.iter().collect()
        } else {
            let filter = self.search_filter.to_lowercase();
            self.connections
                .iter()
                .filter(|c| {
                    c.name.to_lowercase().contains(&filter)
                        || c.host.to_lowercase().contains(&filter)
                        || c.group.to_lowercase().contains(&filter)
                        || c.username.to_lowercase().contains(&filter)
                        || c.protocol.as_str().contains(&filter)
                })
                .collect()
        }
    }

    pub fn grouped_connections(&self) -> BTreeMap<String, Vec<&Connection>> {
        let mut groups = BTreeMap::new();
        for conn in self.filtered_connections() {
            groups
                .entry(conn.group.clone())
                .or_insert_with(Vec::new)
                .push(conn);
        }
        groups
    }

    pub fn set_search_filter(&mut self, query: &str) {
        self.search_filter = query.to_string();
    }

    pub fn set_theme(&mut self, theme: &str) {
        self.config.theme = theme.to_string();
        apply_theme(theme);
    }

    /// Build and present GTK4 / Libadwaita ApplicationWindow interface.
    pub fn build_ui(app: &adw::Application, connections: Vec<Connection>, config: AppConfig) -> adw::ApplicationWindow {
        let state = Rc::new(RefCell::new(AppWindowState {
            connections,
            selected_id: None,
            search_query: String::new(),
            config,
        }));

        let window_title = adw::WindowTitle::builder()
            .title("VER - Connection Manager")
            .subtitle(format!("{} connections", state.borrow().connections.len()))
            .build();

        let header_bar = adw::HeaderBar::builder()
            .title_widget(&window_title)
            .build();

        // Start pack buttons
        let add_btn = gtk::Button::builder()
            .icon_name("list-add-symbolic")
            .tooltip_text("Add New Connection")
            .build();

        let search_toggle = gtk::ToggleButton::builder()
            .icon_name("system-search-symbolic")
            .tooltip_text("Search Connections")
            .build();

        header_bar.pack_start(&add_btn);
        header_bar.pack_start(&search_toggle);

        // End pack buttons
        let discovery_btn = gtk::Button::builder()
            .icon_name("network-workgroup-symbolic")
            .tooltip_text("Discover Network Devices")
            .build();

        let prefs_btn = gtk::Button::builder()
            .icon_name("emblem-system-symbolic")
            .tooltip_text("Preferences")
            .build();

        let menu_btn = gtk::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .tooltip_text("Main Menu")
            .build();

        let menu = gio::Menu::new();
        menu.append(Some("About VER"), Some("app.about"));
        menu_btn.set_menu_model(Some(&menu));

        header_bar.pack_end(&menu_btn);
        header_bar.pack_end(&prefs_btn);
        header_bar.pack_end(&discovery_btn);

        // Search bar
        let search_entry = gtk::SearchEntry::builder()
            .placeholder_text("Quick search connections...")
            .hexpand(true)
            .build();

        let search_clamp = adw::Clamp::builder()
            .maximum_size(500)
            .child(&search_entry)
            .build();

        let search_bar = gtk::SearchBar::builder()
            .child(&search_clamp)
            .search_mode_enabled(false)
            .build();

        search_bar.connect_entry(&search_entry);
        search_toggle.bind_property("active", &search_bar, "search-mode-enabled")
            .bidirectional()
            .build();

        // Sidebar ListBox
        let list_box = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::Single)
            .css_classes(vec!["navigation-sidebar"])
            .build();

        let scrolled_sidebar = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .child(&list_box)
            .vexpand(true)
            .build();

        let sidebar_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        sidebar_box.append(&scrolled_sidebar);

        // Content Pane (Welcome vs Editor)
        let content_stack = gtk::Stack::builder()
            .transition_type(gtk::StackTransitionType::Crossfade)
            .build();

        let status_add_btn = gtk::Button::builder()
            .label("Add Connection")
            .css_classes(vec!["suggested-action", "pill"])
            .build();

        let status_page = adw::StatusPage::builder()
            .icon_name("computer-symbolic")
            .title("No Connection Selected")
            .description("Select a connection from the sidebar or add a new one to get started.")
            .child(&status_add_btn)
            .build();

        let editor_container = gtk::Box::new(gtk::Orientation::Vertical, 0);
        editor_container.set_vexpand(true);
        editor_container.set_hexpand(true);

        let vnc_container = gtk::Box::new(gtk::Orientation::Vertical, 0);
        vnc_container.set_vexpand(true);
        vnc_container.set_hexpand(true);

        content_stack.add_named(&status_page, Some("welcome"));
        content_stack.add_named(&editor_container, Some("editor"));
        content_stack.add_named(&vnc_container, Some("vnc_session"));
        
        // External Session Tracker Container
        let external_session_container = gtk::Box::new(gtk::Orientation::Vertical, 0);
        external_session_container.set_vexpand(true);
        external_session_container.set_hexpand(true);
        content_stack.add_named(&external_session_container, Some("external_session"));
        
        content_stack.set_visible_child_name("welcome");


        let paned = gtk::Paned::builder()
            .orientation(gtk::Orientation::Horizontal)
            .position(280)
            .start_child(&sidebar_box)
            .end_child(&content_stack)
            .shrink_start_child(false)
            .shrink_end_child(false)
            .build();

        let main_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        main_box.append(&header_bar);
        main_box.append(&search_bar);
        main_box.append(&paned);

        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("VER - Connection Manager")
            .default_width(900)
            .default_height(650)
            .content(&main_box)
            .build();

        window.connect_close_request(move |win| {
            win.set_visible(false);
            glib::Propagation::Stop
        });

        // Action: About VER
        let window_clone = window.clone();
        let about_action = gio::SimpleAction::new("about", None);
        about_action.connect_activate(move |_, _| {
            let about = adw::AboutWindow::builder()
                .application_name("VER - Very Easy Remote")
                .developer_name("VER Team")
                .version("0.1.0")
                .comments("GTK4 / Libadwaita Remote Connection Manager in Rust")
                .license_type(gtk::License::Gpl30)
                .transient_for(&window_clone)
                .modal(true)
                .build();
            about.present();
        });
        app.add_action(&about_action);

        // Helper to create a ListBoxRow for a connection
        let create_row = |conn: &Connection| -> gtk::ListBoxRow {
            let row = gtk::ListBoxRow::new();
            row.set_widget_name(&conn.id);

            let icon_name = match conn.protocol {
                Protocol::Rdp | Protocol::Xrdp => "video-display-symbolic",
                Protocol::Vnc => "computer-symbolic",
                Protocol::Ssh => "utilities-terminal-symbolic",
                Protocol::Spice => "video-display-symbolic",
            };

            let subtitle = if conn.username.is_empty() {
                conn.host.clone()
            } else {
                format!("{}@{}", conn.username, conn.host)
            };

            let action_row = adw::ActionRow::builder()
                .title(&conn.name)
                .subtitle(subtitle)
                .build();

            let icon = gtk::Image::from_icon_name(icon_name);
            action_row.add_prefix(&icon);
            row.set_child(Some(&action_row));
            row
        };

        // Populate initial connection rows
        {
            let st = state.borrow();
            for conn in &st.connections {
                let row = create_row(conn);
                list_box.append(&row);
            }
        }

        // Group Header Func
        self::setup_group_headers(&list_box, state.clone());

        // Sort Func (Group ASC -> Name ASC)
        self::setup_sorting(&list_box, state.clone());

        // Filter Func
        self::setup_filtering(&list_box, &search_entry, state.clone());

        // Callback for Add Connection
        let add_conn_action = {
            let state = state.clone();
            let list_box = list_box.clone();
            let window_title = window_title.clone();
            move || {
                let default_protocol = state.borrow().config.default_protocol.clone();
                let new_conn = Connection::new_with_protocol(default_protocol);
                state.borrow_mut().connections.push(new_conn.clone());
                let _ = storage::save_connections(&state.borrow().connections);

                let row = create_row(&new_conn);
                list_box.append(&row);
                list_box.invalidate_filter();
                list_box.invalidate_sort();
                list_box.invalidate_headers();

                list_box.select_row(Some(&row));
                window_title.set_subtitle(&format!("{} connections", state.borrow().connections.len()));
            }
        };

        let add_action_1 = add_conn_action.clone();
        add_btn.connect_clicked(move |_| add_action_1());

        let add_action_2 = add_conn_action;
        status_add_btn.connect_clicked(move |_| add_action_2());

        // Preferences Button Action
        let window_for_prefs = window.clone();
        let state_for_prefs = state.clone();
        prefs_btn.connect_clicked(move |_| {
            let config_rc = Rc::new(RefCell::new(state_for_prefs.borrow().config.clone()));
            let prefs_dialog = PreferencesWindow::build_window(Some(&window_for_prefs), config_rc.clone());
            
            // Sync updated config back to window state on close
            let state_for_close = state_for_prefs.clone();
            prefs_dialog.connect_close_request(move |_| {
                state_for_close.borrow_mut().config = config_rc.borrow().clone();
                gtk::glib::Propagation::Proceed
            });

            prefs_dialog.present();
        });

        // Discovery Button Action
        let window_for_disc = window.clone();
        let state_for_disc = state.clone();
        let list_box_for_disc = list_box.clone();
        let window_title_for_disc = window_title.clone();
        discovery_btn.connect_clicked(move |_| {
            let state_inner = state_for_disc.clone();
            let list_box_inner = list_box_for_disc.clone();
            let window_title_inner = window_title_for_disc.clone();

            let disc_window = DiscoveryDialog::build_window(
                Some(&window_for_disc),
                move |new_conn: Connection| {
                    state_inner.borrow_mut().connections.push(new_conn.clone());
                    let _ = storage::save_connections(&state_inner.borrow().connections);

                    let row = create_row(&new_conn);
                    list_box_inner.append(&row);
                    list_box_inner.invalidate_filter();
                    list_box_inner.invalidate_sort();
                    list_box_inner.invalidate_headers();

                    list_box_inner.select_row(Some(&row));
                    window_title_inner.set_subtitle(&format!("{} connections", state_inner.borrow().connections.len()));
                },
            );
            disc_window.present();
        });

        // Row Selection Callback & Editor View construction
        let state_for_select = state.clone();
        let content_stack_for_select = content_stack.clone();
        let editor_container_for_select = editor_container.clone();
        let list_box_for_select = list_box.clone();
        let window_title_for_select = window_title.clone();
        
        let window_for_select = window.clone();
        let header_bar_for_select = header_bar.clone();
        let sidebar_box_for_select = sidebar_box.clone();

        list_box.connect_row_selected(move |_, row_opt| {
            while let Some(child) = editor_container_for_select.first_child() {
                editor_container_for_select.remove(&child);
            }

            if let Some(row) = row_opt {
                let conn_id = row.widget_name().to_string();
                state_for_select.borrow_mut().selected_id = Some(conn_id.clone());

                let conn_opt = state_for_select.borrow().connections.iter().find(|c| c.id == conn_id).cloned();
                if let Some(conn) = conn_opt {
                    let password = secrets::get_password_sync(&conn.id)
                        .unwrap_or(None)
                        .unwrap_or_default();

                    // Wire Callbacks for ConnectionEditor
                    let state_on_save = state_for_select.clone();
                    let list_box_on_save = list_box_for_select.clone();
                    let row_on_save = row.clone();
                    let on_save = move |updated_conn: Connection, updated_pass: String| {
                        // 1. Update connection vector
                        if let Some(c) = state_on_save.borrow_mut().connections.iter_mut().find(|c| c.id == updated_conn.id) {
                            *c = updated_conn.clone();
                        }
                        let _ = storage::save_connections(&state_on_save.borrow().connections);

                        // 2. Update password in keyring
                        if updated_pass.is_empty() {
                            let _ = secrets::delete_password_sync(&updated_conn.id);
                        } else {
                            let _ = secrets::set_password_sync(&updated_conn.id, &updated_pass);
                        }

                        // 3. Update list row subtitle/title
                        if let Some(child) = row_on_save.child() {
                            if let Ok(action_row) = child.downcast::<adw::ActionRow>() {
                                action_row.set_title(&updated_conn.name);
                                let subtitle = if updated_conn.username.is_empty() {
                                    updated_conn.host.clone()
                                } else {
                                    format!("{}@{}", updated_conn.username, updated_conn.host)
                                };
                                action_row.set_subtitle(&subtitle);
                            }
                        }

                        list_box_on_save.invalidate_sort();
                        list_box_on_save.invalidate_headers();
                    };

                    let external_session_container_for_connect = external_session_container.clone();
                    let content_stack_for_tracker = content_stack_for_select.clone();

                    enum ExternalSessionEvent {
                        Log(String),
                        Exit(bool), // true if success (0), false if error
                    }

                    let track_external_session = move |mut child: std::process::Child, name: String| {
                        let container = external_session_container_for_connect.clone();
                        let stack = content_stack_for_tracker.clone();
                        
                        while let Some(c) = container.first_child() {
                            container.remove(&c);
                        }

                        let title = gtk::Label::builder()
                            .label(format!("External Session Active: {}", name))
                            .css_classes(vec!["title-2"])
                            .margin_top(48)
                            .margin_bottom(12)
                            .build();

                        let btn_disconnect = gtk::Button::builder()
                            .label("Disconnect")
                            .css_classes(vec!["destructive-action", "pill"])
                            .halign(gtk::Align::Center)
                            .margin_bottom(24)
                            .build();

                        let log_view = gtk::TextView::builder()
                            .editable(false)
                            .cursor_visible(false)
                            .monospace(true)
                            .css_classes(vec!["card", "view"])
                            .left_margin(12)
                            .right_margin(12)
                            .top_margin(12)
                            .bottom_margin(12)
                            .wrap_mode(gtk::WrapMode::WordChar)
                            .build();
                        
                        let scroll = gtk::ScrolledWindow::builder()
                            .child(&log_view)
                            .vexpand(true)
                            .hexpand(true)
                            .min_content_height(300)
                            .margin_start(24)
                            .margin_end(24)
                            .margin_bottom(24)
                            .build();

                        let (tx, rx) = async_channel::unbounded::<ExternalSessionEvent>();

                        let stdout_pipe = child.stdout.take();
                        let stderr_pipe = child.stderr.take();

                        let tx_out = tx.clone();
                        if let Some(stdout) = stdout_pipe {
                            std::thread::spawn(move || {
                                use std::io::{BufRead, BufReader};
                                let reader = BufReader::new(stdout);
                                for line in reader.lines() {
                                    if let Ok(l) = line {
                                        let _ = tx_out.send_blocking(ExternalSessionEvent::Log(l));
                                    }
                                }
                            });
                        }

                        let tx_err = tx.clone();
                        if let Some(stderr) = stderr_pipe {
                            std::thread::spawn(move || {
                                use std::io::{BufRead, BufReader};
                                let reader = BufReader::new(stderr);
                                for line in reader.lines() {
                                    if let Ok(l) = line {
                                        let _ = tx_err.send_blocking(ExternalSessionEvent::Log(l));
                                    }
                                }
                            });
                        }

                        let child_arc = std::sync::Arc::new(std::sync::Mutex::new(Some(child)));
                        let child_arc_for_btn = child_arc.clone();
                        let stack_for_btn = stack.clone();
                        
                        btn_disconnect.connect_clicked(move |_| {
                            let mut opt = child_arc_for_btn.lock().unwrap();
                            if let Some(mut c) = opt.take() {
                                let _ = c.kill();
                                let _ = c.wait();
                            }
                            stack_for_btn.set_visible_child_name("editor");
                        });

                        container.append(&title);
                        container.append(&btn_disconnect);
                        container.append(&scroll);
                        stack.set_visible_child_name("external_session");

                        let child_arc_for_thread = child_arc.clone();
                        let tx_exit = tx.clone();
                        
                        std::thread::spawn(move || {
                            loop {
                                std::thread::sleep(std::time::Duration::from_millis(500));
                                let mut opt = child_arc_for_thread.lock().unwrap();
                                if let Some(ref mut c) = *opt {
                                    if let Ok(Some(status)) = c.try_wait() {
                                        let success = status.success();
                                        let _ = tx_exit.send_blocking(ExternalSessionEvent::Exit(success));
                                        break;
                                    }
                                } else {
                                    break;
                                }
                            }
                        });

                        let buffer = log_view.buffer();
                        let title_for_exit = title.clone();
                        let btn_for_exit = btn_disconnect.clone();

                        gtk::glib::MainContext::default().spawn_local(async move {
                            while let Ok(event) = rx.recv().await {
                                match event {
                                    ExternalSessionEvent::Log(msg) => {
                                        let mut iter = buffer.end_iter();
                                        buffer.insert(&mut iter, &format!("{}\n", msg));
                                        // Scroll to bottom
                                        let mark = buffer.create_mark(None, &buffer.end_iter(), false);
                                        log_view.scroll_to_mark(&mark, 0.0, true, 0.0, 1.0);
                                    }
                                    ExternalSessionEvent::Exit(success) => {
                                        if success {
                                            stack.set_visible_child_name("editor");
                                        } else {
                                            title_for_exit.set_label("Session Failed");
                                            title_for_exit.add_css_class("error");
                                            btn_for_exit.set_label("Close");
                                        }
                                    }
                                }
                            }
                        });
                    };

                    let vnc_container_for_connect = vnc_container.clone();
                    let content_stack_on_connect = content_stack_for_select.clone();
                    
                    let window_for_vnc = window_for_select.clone();
                    let header_bar_for_vnc = header_bar_for_select.clone();
                    let sidebar_box_for_vnc = sidebar_box_for_select.clone();

                    let on_connect = move |conn: Connection, pass: String| {
                        match conn.protocol {
                            Protocol::Rdp | Protocol::Xrdp => {
                                let pass_opt = if pass.is_empty() { None } else { Some(pass.as_str()) };
                                if let Ok(child) = crate::launcher::launch_rdp(&conn, pass_opt) {
                                    track_external_session(child, conn.name.clone());
                                }
                            }
                            Protocol::Ssh => {
                                if let Ok(child) = launcher::launch_ssh(&conn) {
                                    track_external_session(child, conn.name.clone());
                                }
                            }
                            Protocol::Spice => {
                                let pass_opt = if pass.is_empty() { None } else { Some(pass.as_str()) };
                                if let Ok(child) = launcher::launch_spice(&conn, pass_opt) {
                                    track_external_session(child, conn.name.clone());
                                }
                            }
                            Protocol::Vnc => {
                                // Clear prior VNC session widgets
                                while let Some(child) = vnc_container_for_connect.first_child() {
                                    vnc_container_for_connect.remove(&child);
                                }

                                let vnc_overlay = gtk::Overlay::builder().build();
                                vnc_container_for_connect.append(&vnc_overlay);

                                let toolbar = gtk::FlowBox::builder()
                                    .selection_mode(gtk::SelectionMode::None)
                                    .column_spacing(8)
                                    .row_spacing(8)
                                    .build();
                                toolbar.set_margin_top(8);
                                toolbar.set_margin_bottom(8);
                                toolbar.set_margin_start(12);
                                toolbar.set_margin_end(12);

                                let status_label = gtk::Label::builder()
                                    .label(format!("Connecting to {}:{}...", conn.host, conn.port))
                                    .hexpand(true)
                                    .xalign(0.0)
                                    .wrap(true)
                                    .wrap_mode(gtk::pango::WrapMode::WordChar)
                                    .css_classes(vec!["title-4"])
                                    .build();

                                let scaling_model = gtk::StringList::new(&["Original Size", "Fit to Window", "Stretch"]);
                                let scaling_idx = match conn.advanced_settings.vnc_scaling {
                                    VncScaling::OriginalSize => 0,
                                    VncScaling::FitToWindow => 1,
                                    VncScaling::Stretch => 2,
                                };
                                let combo_scaling = gtk::DropDown::builder()
                                    .model(&scaling_model)
                                    .selected(scaling_idx)
                                    .build();

                                let btn_cad = gtk::Button::builder()
                                    .label("Ctrl+Alt+Del")
                                    .tooltip_text("Send Ctrl+Alt+Del key sequence")
                                    .build();

                                let btn_fullscreen = gtk::Button::builder()
                                    .label("Fullscreen")
                                    .tooltip_text("Toggle Fullscreen")
                                    .css_classes(vec!["suggested-action"])
                                    .build();

                                let btn_disconnect = gtk::Button::builder()
                                    .label("Disconnect VNC")
                                    .css_classes(vec!["destructive-action"])
                                    .build();

                                toolbar.insert(&status_label, -1);
                                toolbar.insert(&combo_scaling, -1);
                                toolbar.insert(&btn_cad, -1);
                                toolbar.insert(&btn_fullscreen, -1);
                                toolbar.insert(&btn_disconnect, -1);

                                let toolbar_revealer = gtk::Revealer::builder()
                                    .child(&toolbar)
                                    .reveal_child(true)
                                    .transition_type(gtk::RevealerTransitionType::SlideDown)
                                    .halign(gtk::Align::Fill)
                                    .valign(gtk::Align::Start)
                                    .build();

                                vnc_overlay.add_overlay(&toolbar_revealer);

                                let scaling = conn.advanced_settings.vnc_scaling.clone();
                                let encoding = conn.advanced_settings.vnc_encoding.clone();
                                let client = VncClient::new(conn.host.clone(), conn.port, scaling.clone(), encoding);
                                let vnc_widget = VncWidget::new(scaling);
                                let vnc_widget_rc = Rc::new(RefCell::new(vnc_widget));

                                if let Some(scrolled) = vnc_widget_rc.borrow().widget() {
                                    vnc_overlay.set_child(Some(scrolled));
                                    vnc_widget_rc.borrow().setup_event_controllers(vnc_widget_rc.clone());
                                }

                                // Fullscreen toggle logic
                                let is_fs = Rc::new(RefCell::new(false));
                                let win_fs = window_for_vnc.clone();
                                let header_fs = header_bar_for_vnc.clone();
                                let sidebar_fs = sidebar_box_for_vnc.clone();
                                let rev_fs = toolbar_revealer.clone();

                                let btn_fs_clone = btn_fullscreen.clone();
                                let is_fs_clone = is_fs.clone();
                                btn_fullscreen.connect_clicked(move |_| {
                                    let current = *is_fs_clone.borrow();
                                    if !current {
                                        win_fs.fullscreen();
                                        header_fs.set_visible(false);
                                        sidebar_fs.set_visible(false);
                                        btn_fs_clone.set_label("Restore");
                                        rev_fs.set_reveal_child(false);
                                    } else {
                                        win_fs.unfullscreen();
                                        header_fs.set_visible(true);
                                        sidebar_fs.set_visible(true);
                                        btn_fs_clone.set_label("Fullscreen");
                                        rev_fs.set_reveal_child(true);
                                    }
                                    *is_fs_clone.borrow_mut() = !current;
                                });

                                // Hover detection
                                let motion = gtk::EventControllerMotion::new();
                                let rev_motion = toolbar_revealer.clone();
                                let is_fs_motion = is_fs.clone();
                                motion.connect_motion(move |_, _x, y| {
                                    if *is_fs_motion.borrow() {
                                        if y < 60.0 {
                                            rev_motion.set_reveal_child(true);
                                        } else {
                                            rev_motion.set_reveal_child(false);
                                        }
                                    }
                                });
                                vnc_overlay.add_controller(motion);

                                #[allow(deprecated)]
                                let (glib_tx, glib_rx) = glib::MainContext::channel::<VncSessionEvent>(glib::Priority::default());
                                let pass_opt = if pass.is_empty() { None } else { Some(pass.clone()) };
                                let cmd_tx = client.start_session(pass_opt, glib_tx);
                                vnc_widget_rc.borrow_mut().set_cmd_tx(cmd_tx.clone());

                                let widget_for_scaling = vnc_widget_rc.clone();
                                combo_scaling.connect_selected_notify(move |dropdown| {
                                    let mode = match dropdown.selected() {
                                        1 => VncScaling::FitToWindow,
                                        2 => VncScaling::Stretch,
                                        _ => VncScaling::OriginalSize,
                                    };
                                    widget_for_scaling.borrow_mut().set_scaling(mode);
                                });

                                let cmd_tx_cad = cmd_tx.clone();
                                btn_cad.connect_clicked(move |_| {
                                    let _ = cmd_tx_cad.send(VncCommand::KeyEvent { keysym: 0xFFE3, down: true });
                                    let _ = cmd_tx_cad.send(VncCommand::KeyEvent { keysym: 0xFFE9, down: true });
                                    let _ = cmd_tx_cad.send(VncCommand::KeyEvent { keysym: 0xFFFF, down: true });
                                    let _ = cmd_tx_cad.send(VncCommand::KeyEvent { keysym: 0xFFFF, down: false });
                                    let _ = cmd_tx_cad.send(VncCommand::KeyEvent { keysym: 0xFFE9, down: false });
                                    let _ = cmd_tx_cad.send(VncCommand::KeyEvent { keysym: 0xFFE3, down: false });
                                });

                                let cmd_tx_disc = cmd_tx.clone();
                                let stack_disc = content_stack_on_connect.clone();
                                btn_disconnect.connect_clicked(move |_| {
                                    let _ = cmd_tx_disc.send(VncCommand::Disconnect);
                                    stack_disc.set_visible_child_name("editor");
                                });

                                let widget_rx = vnc_widget_rc.clone();
                                let label_rx = status_label.clone();
                                glib_rx.attach(None, move |event| {
                                    match event {
                                        VncSessionEvent::Connected { width, height, name } => {
                                            label_rx.set_label(&format!("Connected: {} ({}x{})", name, width, height));
                                        }
                                        VncSessionEvent::FrameUpdate(frame) => {
                                            widget_rx.borrow_mut().render_frame(frame);
                                        }
                                        VncSessionEvent::Disconnected(msg) => {
                                            label_rx.set_label(&format!("Disconnected: {}", msg));
                                        }
                                        VncSessionEvent::Error(err) => {
                                            label_rx.set_label(&format!("Error: {}", err));
                                        }
                                    }
                                    glib::ControlFlow::Continue
                                });

                                content_stack_on_connect.set_visible_child_name("vnc_session");
                            }
                        }
                    };

                    let state_on_dup = state_for_select.clone();
                    let list_box_on_dup = list_box_for_select.clone();
                    let window_title_on_dup = window_title_for_select.clone();
                    let on_duplicate = move |dup_conn: Connection, dup_pass: String| {
                        state_on_dup.borrow_mut().connections.push(dup_conn.clone());
                        let _ = storage::save_connections(&state_on_dup.borrow().connections);

                        if !dup_pass.is_empty() {
                            let _ = secrets::set_password_sync(&dup_conn.id, &dup_pass);
                        }

                        let new_row = create_row(&dup_conn);
                        list_box_on_dup.append(&new_row);
                        list_box_on_dup.invalidate_filter();
                        list_box_on_dup.invalidate_sort();
                        list_box_on_dup.invalidate_headers();

                        list_box_on_dup.select_row(Some(&new_row));
                        window_title_on_dup.set_subtitle(&format!("{} connections", state_on_dup.borrow().connections.len()));
                    };

                    let state_on_del = state_for_select.clone();
                    let list_box_on_del = list_box_for_select.clone();
                    let window_title_on_del = window_title_for_select.clone();
                    let row_on_del = row.clone();
                    let on_delete = move |del_id: String| {
                        state_on_del.borrow_mut().connections.retain(|c| c.id != del_id);
                        let _ = storage::save_connections(&state_on_del.borrow().connections);
                        let _ = secrets::delete_password_sync(&del_id);

                        list_box_on_del.remove(&row_on_del);
                        list_box_on_del.unselect_all();
                        window_title_on_del.set_subtitle(&format!("{} connections", state_on_del.borrow().connections.len()));
                    };

                    let on_wake = move |mac: String| {
                        let _ = network::send_wol(&mac);
                    };

                    let editor_widget = ConnectionEditor::build_widget(
                        conn,
                        password,
                        on_save,
                        on_connect,
                        on_duplicate,
                        on_delete,
                        on_wake,
                    );

                    editor_container_for_select.append(&editor_widget);
                    content_stack_for_select.set_visible_child_name("editor");
                }
            } else {
                state_for_select.borrow_mut().selected_id = None;
                content_stack_for_select.set_visible_child_name("welcome");
            }
        });

        window
    }
}

fn setup_group_headers(list_box: &gtk::ListBox, state: Rc<RefCell<AppWindowState>>) {
    list_box.set_header_func(move |row, before| {
        let st = state.borrow();
        let conn = st.connections.iter().find(|c| c.id == row.widget_name().as_str());
        let before_conn = before.and_then(|b| st.connections.iter().find(|c| c.id == b.widget_name().as_str()).cloned());

        let current_group = conn.map(|c| c.group.as_str()).unwrap_or("Default");
        let prev_group = before_conn.as_ref().map(|c| c.group.as_str());

        if prev_group != Some(current_group) {
            let label = gtk::Label::builder()
                .label(current_group)
                .xalign(0.0)
                .css_classes(vec!["heading", "dim-label"])
                .margin_top(12)
                .margin_bottom(4)
                .margin_start(12)
                .margin_end(12)
                .build();
            row.set_header(Some(&label));
        } else {
            row.set_header(None::<&gtk::Widget>);
        }
    });
}

fn setup_sorting(list_box: &gtk::ListBox, state: Rc<RefCell<AppWindowState>>) {
    list_box.set_sort_func(move |row1, row2| {
        let st = state.borrow();
        let c1 = st.connections.iter().find(|c| c.id == row1.widget_name().as_str());
        let c2 = st.connections.iter().find(|c| c.id == row2.widget_name().as_str());

        match (c1, c2) {
            (Some(a), Some(b)) => {
                let group_cmp = a.group.to_lowercase().cmp(&b.group.to_lowercase());
                if group_cmp != std::cmp::Ordering::Equal {
                    group_cmp.into()
                } else {
                    a.name.to_lowercase().cmp(&b.name.to_lowercase()).into()
                }
            }
            _ => std::cmp::Ordering::Equal.into(),
        }
    });
}

fn setup_filtering(list_box: &gtk::ListBox, search_entry: &gtk::SearchEntry, state: Rc<RefCell<AppWindowState>>) {
    let state_for_filter = state.clone();
    list_box.set_filter_func(move |row| {
        let st = state_for_filter.borrow();
        let query = st.search_query.trim().to_lowercase();
        if query.is_empty() {
            return true;
        }

        if let Some(c) = st.connections.iter().find(|c| c.id == row.widget_name().as_str()) {
            c.name.to_lowercase().contains(&query)
                || c.host.to_lowercase().contains(&query)
                || c.group.to_lowercase().contains(&query)
                || c.username.to_lowercase().contains(&query)
                || c.protocol.as_str().contains(&query)
        } else {
            false
        }
    });

    let list_box_clone = list_box.clone();
    search_entry.connect_search_changed(move |entry| {
        state.borrow_mut().search_query = entry.text().to_string();
        list_box_clone.invalidate_filter();
        list_box_clone.invalidate_headers();
    });
}
