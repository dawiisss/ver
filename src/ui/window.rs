use gtk::gdk;
use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::importers::{merge_imported_connections, ImportConflictStrategy};
use crate::launcher;
use crate::models::{AppConfig, Connection, Protocol};
use crate::network;
use crate::prober::{self, HostStatus};
use crate::secrets;
use crate::storage;
use crate::ui::discovery::DiscoveryDialog;
use crate::ui::editor::ConnectionEditor;
use crate::ui::export_dialog::ExportDialog;
use crate::ui::import_dialog::ImportDialog;
use crate::ui::preferences::{apply_theme, PreferencesWindow};
use crate::ui::quick_connect::QuickConnectDialog;
use crate::ui::shortcuts::ShortcutsDialog;

#[derive(Clone)]
pub struct ActiveSession {
    pub conn_id: String,
    pub conn_name: String,
    pub child: Arc<Mutex<Option<std::process::Child>>>,
    pub log_buffer: gtk::TextBuffer,
    pub is_running: Arc<AtomicBool>,
}

pub struct AppWindowState {
    pub connections: Vec<Connection>,
    pub selected_id: Option<String>,
    pub search_query: String,
    pub config: AppConfig,
    pub active_sessions: HashMap<String, ActiveSession>,
    pub host_statuses: HashMap<String, HostStatus>,
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
    pub fn build_ui(
        app: &adw::Application,
        connections: Vec<Connection>,
        config: AppConfig,
    ) -> adw::ApplicationWindow {
        // Load custom CSS for reachability status dots
        let css_provider = gtk::CssProvider::new();
        css_provider.load_from_data(
            "
            .status-dot {
                min-width: 8px;
                min-height: 8px;
                border-radius: 9999px;
                margin-right: 6px;
            }
            .status-online {
                background-color: #2ec27e;
            }
            .status-offline {
                background-color: #e01b24;
            }
            .status-probing {
                background-color: #f6d32d;
            }
            .status-unknown {
                background-color: #9a9996;
            }
            ",
        );
        if let Some(display) = gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &css_provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }

        let state = Rc::new(RefCell::new(AppWindowState {
            connections,
            selected_id: None,
            search_query: String::new(),
            config,
            active_sessions: HashMap::new(),
            host_statuses: HashMap::new(),
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
            .tooltip_text("Add New Connection (Ctrl+N)")
            .build();

        let quick_btn = gtk::Button::builder()
            .icon_name("tab-new-symbolic")
            .tooltip_text("Quick Connect (Ctrl+K)")
            .build();

        let search_toggle = gtk::ToggleButton::builder()
            .icon_name("system-search-symbolic")
            .tooltip_text("Search Connections (Ctrl+F)")
            .build();

        header_bar.pack_start(&add_btn);
        header_bar.pack_start(&quick_btn);
        header_bar.pack_start(&search_toggle);

        // End pack buttons
        let refresh_btn = gtk::Button::builder()
            .icon_name("view-refresh-symbolic")
            .tooltip_text("Refresh Reachability (F5)")
            .build();

        let discovery_btn = gtk::Button::builder()
            .icon_name("network-workgroup-symbolic")
            .tooltip_text("Discover Network Devices (Ctrl+D)")
            .build();

        let prefs_btn = gtk::Button::builder()
            .icon_name("emblem-system-symbolic")
            .tooltip_text("Preferences (Ctrl+,)")
            .build();

        let menu_btn = gtk::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .tooltip_text("Main Menu")
            .build();

        let menu = gio::Menu::new();
        menu.append(Some("Quick Connect..."), Some("app.quick_connect"));
        menu.append(Some("Import Connections..."), Some("app.import"));
        menu.append(Some("Export Connections..."), Some("app.export"));
        menu.append(Some("Refresh Reachability"), Some("app.refresh"));
        menu.append(Some("Keyboard Shortcuts"), Some("app.shortcuts"));
        menu.append(Some("Preferences"), Some("app.preferences"));
        menu.append(Some("About VER"), Some("app.about"));
        menu.append(Some("Quit"), Some("app.quit"));
        menu_btn.set_menu_model(Some(&menu));

        header_bar.pack_end(&menu_btn);
        header_bar.pack_end(&prefs_btn);
        header_bar.pack_end(&discovery_btn);
        header_bar.pack_end(&refresh_btn);

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
        search_toggle
            .bind_property("active", &search_bar, "search-mode-enabled")
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
            .description("Select a connection from the sidebar or press Ctrl+K for Quick Connect.")
            .child(&status_add_btn)
            .build();

        let editor_container = gtk::Box::new(gtk::Orientation::Vertical, 0);
        editor_container.set_vexpand(true);
        editor_container.set_hexpand(true);

        content_stack.add_named(&status_page, Some("welcome"));
        content_stack.add_named(&editor_container, Some("editor"));

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

        let toast_overlay = adw::ToastOverlay::new();
        toast_overlay.set_child(Some(&main_box));

        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("VER - Connection Manager")
            .default_width(900)
            .default_height(650)
            .content(&toast_overlay)
            .build();

        window.connect_close_request(move |win| {
            win.set_visible(false);
            glib::Propagation::Stop
        });

        // Helper to create a ListBoxRow for a connection with status dot and protocol icon
        let state_for_row = state.clone();
        let create_row = move |conn: &Connection| -> gtk::ListBoxRow {
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

            // Prefix container with Status Dot and Protocol Icon
            let prefix_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            prefix_box.set_valign(gtk::Align::Center);

            let status_dot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            status_dot.set_valign(gtk::Align::Center);
            status_dot.set_widget_name("status_dot");

            let current_status = state_for_row
                .borrow()
                .host_statuses
                .get(&conn.id)
                .cloned()
                .unwrap_or(HostStatus::Unknown);

            status_dot.set_css_classes(&["status-dot", current_status.status_css_class()]);
            status_dot.set_tooltip_text(Some(&current_status.description()));

            let icon = gtk::Image::from_icon_name(icon_name);

            prefix_box.append(&status_dot);
            prefix_box.append(&icon);
            action_row.add_prefix(&prefix_box);

            let active_badge = gtk::Label::builder()
                .label("Active")
                .css_classes(vec!["success", "pill", "caption"])
                .valign(gtk::Align::Center)
                .visible(false)
                .build();
            active_badge.set_widget_name("active_badge");
            action_row.add_suffix(&active_badge);

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

        // External Session Tracker Container
        enum ExternalSessionEvent {
            Log(String),
            Exit(bool),
        }

        let show_external_session = {
            let container = external_session_container.clone();
            let stack = content_stack.clone();
            let state_for_show = state.clone();
            let list_box_for_show = list_box.clone();
            Rc::new(move |conn_id: &str| {
                let session_opt = state_for_show
                    .borrow()
                    .active_sessions
                    .get(conn_id)
                    .cloned();
                if let Some(session) = session_opt {
                    while let Some(c) = container.first_child() {
                        container.remove(&c);
                    }

                    let title = gtk::Label::builder()
                        .label(format!("External Session Active: {}", session.conn_name))
                        .css_classes(vec!["title-2"])
                        .margin_top(48)
                        .margin_bottom(12)
                        .build();

                    let button_box = gtk::Box::builder()
                        .orientation(gtk::Orientation::Horizontal)
                        .spacing(12)
                        .halign(gtk::Align::Center)
                        .margin_bottom(24)
                        .build();

                    let btn_disconnect = gtk::Button::builder()
                        .label("Disconnect")
                        .css_classes(vec!["destructive-action", "pill"])
                        .build();

                    let btn_view_settings = gtk::Button::builder()
                        .label("View Settings")
                        .css_classes(vec!["flat", "pill"])
                        .build();

                    button_box.append(&btn_disconnect);
                    button_box.append(&btn_view_settings);

                    let log_view = gtk::TextView::builder()
                        .buffer(&session.log_buffer)
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

                    let child_arc = session.child.clone();
                    let is_running = session.is_running.clone();
                    let stack_for_btn = stack.clone();
                    let state_for_btn = state_for_show.clone();
                    let list_box_for_btn = list_box_for_show.clone();
                    let id_for_btn = conn_id.to_string();

                    btn_disconnect.connect_clicked(move |_| {
                        is_running.store(false, Ordering::SeqCst);
                        let mut opt = child_arc.lock().unwrap();
                        if let Some(mut c) = opt.take() {
                            std::thread::spawn(move || {
                                #[cfg(unix)]
                                {
                                    let pid = c.id();
                                    unsafe {
                                        libc::kill(-(pid as i32), libc::SIGTERM);
                                    }
                                }

                                let mut exited = false;
                                for _ in 0..20 {
                                    if let Ok(Some(_)) = c.try_wait() {
                                        exited = true;
                                        break;
                                    }
                                    std::thread::sleep(std::time::Duration::from_millis(100));
                                }

                                if !exited {
                                    #[cfg(unix)]
                                    {
                                        let pid = c.id();
                                        unsafe {
                                            libc::kill(-(pid as i32), libc::SIGKILL);
                                        }
                                    }
                                    let _ = c.kill();
                                    let _ = c.wait();
                                }
                            });
                        }
                        state_for_btn
                            .borrow_mut()
                            .active_sessions
                            .remove(&id_for_btn);
                        set_row_active_badge(&list_box_for_btn, &id_for_btn, false);
                        stack_for_btn.set_visible_child_name("editor");
                    });

                    let stack_for_settings = stack.clone();
                    btn_view_settings.connect_clicked(move |_| {
                        stack_for_settings.set_visible_child_name("editor");
                    });

                    container.append(&title);
                    container.append(&button_box);
                    container.append(&scroll);
                    stack.set_visible_child_name("external_session");
                }
            })
        };

        let track_external_session = {
            let state_for_track = state.clone();
            let list_box_for_track = list_box.clone();
            let show_session_for_track = show_external_session.clone();
            let stack_for_track = content_stack.clone();
            Rc::new(move |mut child: std::process::Child, conn: Connection| {
                let conn_id = conn.id.clone();
                let conn_name = conn.name.clone();
                let log_buffer = gtk::TextBuffer::new(None);
                let is_running = Arc::new(AtomicBool::new(true));

                let stdout_pipe = child.stdout.take();
                let stderr_pipe = child.stderr.take();
                let child_arc = Arc::new(Mutex::new(Some(child)));

                let active_session = ActiveSession {
                    conn_id: conn_id.clone(),
                    conn_name: conn_name.clone(),
                    child: child_arc.clone(),
                    log_buffer: log_buffer.clone(),
                    is_running: is_running.clone(),
                };

                state_for_track
                    .borrow_mut()
                    .active_sessions
                    .insert(conn_id.clone(), active_session);

                set_row_active_badge(&list_box_for_track, &conn_id, true);

                let (tx, rx) = async_channel::unbounded::<ExternalSessionEvent>();

                let tx_out = tx.clone();
                if let Some(stdout) = stdout_pipe {
                    std::thread::spawn(move || {
                        use std::io::{BufRead, BufReader};
                        let reader = BufReader::new(stdout);
                        for l in reader.lines().map_while(Result::ok) {
                            let _ = tx_out.send_blocking(ExternalSessionEvent::Log(l));
                        }
                    });
                }

                let tx_err = tx.clone();
                if let Some(stderr) = stderr_pipe {
                    std::thread::spawn(move || {
                        use std::io::{BufRead, BufReader};
                        let reader = BufReader::new(stderr);
                        for l in reader.lines().map_while(Result::ok) {
                            let _ = tx_err.send_blocking(ExternalSessionEvent::Log(l));
                        }
                    });
                }

                let child_arc_for_wait = child_arc.clone();
                let is_running_for_wait = is_running.clone();
                let tx_exit = tx.clone();

                std::thread::spawn(move || loop {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    let mut opt = child_arc_for_wait.lock().unwrap();
                    if let Some(ref mut c) = *opt {
                        if let Ok(Some(status)) = c.try_wait() {
                            let success = status.success();
                            is_running_for_wait.store(false, Ordering::SeqCst);
                            let _ = tx_exit.send_blocking(ExternalSessionEvent::Exit(success));
                            break;
                        }
                    } else {
                        is_running_for_wait.store(false, Ordering::SeqCst);
                        break;
                    }
                });

                let buffer = log_buffer.clone();
                let list_box_for_exit = list_box_for_track.clone();
                let state_for_exit = state_for_track.clone();
                let stack_for_exit = stack_for_track.clone();
                let conn_id_for_exit = conn_id.clone();

                gtk::glib::MainContext::default().spawn_local(async move {
                    while let Ok(event) = rx.recv().await {
                        match event {
                            ExternalSessionEvent::Log(msg) => {
                                let mut iter = buffer.end_iter();
                                buffer.insert(&mut iter, &format!("{}\n", msg));
                            }
                            ExternalSessionEvent::Exit(success) => {
                                state_for_exit
                                    .borrow_mut()
                                    .active_sessions
                                    .remove(&conn_id_for_exit);
                                set_row_active_badge(&list_box_for_exit, &conn_id_for_exit, false);

                                let is_currently_selected = state_for_exit
                                    .borrow()
                                    .selected_id
                                    .as_ref()
                                    .map(|id| id == &conn_id_for_exit)
                                    .unwrap_or(false);

                                if is_currently_selected && success {
                                    stack_for_exit.set_visible_child_name("editor");
                                }
                            }
                        }
                    }
                });

                show_session_for_track(&conn_id);
            })
        };

        let state_for_launch = state.clone();
        let toast_overlay_for_launch = toast_overlay.clone();
        let track_session_for_launch = track_external_session.clone();
        let launch_session = Rc::new(move |conn: Connection, pass: String| {
            state_for_launch.borrow_mut().config.last_connected_id = Some(conn.id.clone());
            let _ = storage::save_config(&state_for_launch.borrow().config);

            let launch_result = match conn.protocol {
                Protocol::Rdp | Protocol::Xrdp => {
                    let pass_opt = if pass.is_empty() {
                        None
                    } else {
                        Some(pass.as_str())
                    };
                    crate::launcher::launch_rdp(&conn, pass_opt)
                }
                Protocol::Ssh => launcher::launch_ssh(&conn),
                Protocol::Spice => {
                    let pass_opt = if pass.is_empty() {
                        None
                    } else {
                        Some(pass.as_str())
                    };
                    launcher::launch_spice(&conn, pass_opt)
                }
                Protocol::Vnc => {
                    let pass_opt = if pass.is_empty() {
                        None
                    } else {
                        Some(pass.as_str())
                    };
                    launcher::launch_vnc(&conn, pass_opt)
                }
            };

            match launch_result {
                Ok(child) => {
                    track_session_for_launch(child, conn);
                }
                Err(err) => {
                    toast_overlay_for_launch
                        .add_toast(adw::Toast::new(&format!("Connection failed: {}", err)));
                }
            }
        });

        // Background Batch Reachability Prober
        let trigger_batch_probe = {
            let state_for_probe = state.clone();
            let list_box_for_probe = list_box.clone();
            Rc::new(move || {
                let targets: Vec<(String, String, u16)> = {
                    let st = state_for_probe.borrow();
                    st.connections
                        .iter()
                        .map(|c| (c.id.clone(), c.host.clone(), c.port))
                        .collect()
                };

                if targets.is_empty() {
                    return;
                }

                // Set all to probing status
                for (id, _, _) in &targets {
                    state_for_probe
                        .borrow_mut()
                        .host_statuses
                        .insert(id.clone(), HostStatus::Probing);
                    set_row_status_dot(&list_box_for_probe, id, &HostStatus::Probing);
                }

                let state_inner = state_for_probe.clone();
                let list_box_inner = list_box_for_probe.clone();

                let (tx, rx) = async_channel::unbounded::<(String, HostStatus)>();
                prober::spawn_batch_probe(targets, Duration::from_millis(2000), 8, tx);

                glib::MainContext::default().spawn_local(async move {
                    while let Ok((id, status)) = rx.recv().await {
                        state_inner
                            .borrow_mut()
                            .host_statuses
                            .insert(id.clone(), status.clone());
                        set_row_status_dot(&list_box_inner, &id, &status);
                    }
                });
            })
        };

        // Trigger batch probe on startup
        trigger_batch_probe();

        // Refresh button action
        let trigger_probe_btn = trigger_batch_probe.clone();
        refresh_btn.connect_clicked(move |_| {
            trigger_probe_btn();
        });

        // Add Connection Action
        let create_row_for_add = create_row.clone();
        let add_conn_action = {
            let state = state.clone();
            let list_box = list_box.clone();
            let window_title = window_title.clone();
            let create_row_inner = create_row_for_add.clone();
            Rc::new(move || {
                let default_protocol = state.borrow().config.default_protocol;
                let new_conn = Connection::new_with_protocol(default_protocol);
                state.borrow_mut().connections.push(new_conn.clone());
                let _ = storage::save_connections(&state.borrow().connections);

                let row = create_row_inner(&new_conn);
                list_box.append(&row);
                list_box.invalidate_filter();
                list_box.invalidate_sort();
                list_box.invalidate_headers();

                list_box.select_row(Some(&row));
                window_title
                    .set_subtitle(&format!("{} connections", state.borrow().connections.len()));
            })
        };

        let add_action_1 = add_conn_action.clone();
        add_btn.connect_clicked(move |_| add_action_1());

        let add_action_2 = add_conn_action.clone();
        status_add_btn.connect_clicked(move |_| add_action_2());

        // Quick Connect Action
        let window_for_qc = window.clone();
        let state_for_qc = state.clone();
        let list_box_for_qc = list_box.clone();
        let window_title_for_qc = window_title.clone();
        let launch_session_for_qc = launch_session.clone();
        let create_row_for_qc = create_row.clone();

        let open_quick_connect = Rc::new(move || {
            let default_proto = state_for_qc.borrow().config.default_protocol;
            let launch_connect = launch_session_for_qc.clone();
            let state_save = state_for_qc.clone();
            let list_box_save = list_box_for_qc.clone();
            let window_title_save = window_title_for_qc.clone();
            let launch_save = launch_session_for_qc.clone();
            let create_row_save = create_row_for_qc.clone();

            QuickConnectDialog::show(
                &window_for_qc,
                default_proto,
                move |conn, pass| {
                    launch_connect(conn, pass.unwrap_or_default());
                },
                move |conn, pass| {
                    let pass_str = pass.unwrap_or_default();
                    if !pass_str.is_empty() {
                        let _ = secrets::set_password_sync(&conn.id, &pass_str);
                    }
                    state_save.borrow_mut().connections.push(conn.clone());
                    let _ = storage::save_connections(&state_save.borrow().connections);

                    let row = create_row_save(&conn);
                    list_box_save.append(&row);
                    list_box_save.invalidate_filter();
                    list_box_save.invalidate_sort();
                    list_box_save.invalidate_headers();
                    list_box_save.select_row(Some(&row));
                    window_title_save.set_subtitle(&format!(
                        "{} connections",
                        state_save.borrow().connections.len()
                    ));

                    launch_save(conn, pass_str);
                },
            );
        });

        let open_qc_btn = open_quick_connect.clone();
        quick_btn.connect_clicked(move |_| {
            open_qc_btn();
        });

        // Import Action
        let window_for_import = window.clone();
        let state_for_import = state.clone();
        let list_box_for_import = list_box.clone();
        let window_title_for_import = window_title.clone();
        let toast_overlay_import = toast_overlay.clone();
        let create_row_import = create_row.clone();
        let trigger_probe_import = trigger_batch_probe.clone();

        let open_import_dialog = Rc::new(move || {
            let state_inner = state_for_import.clone();
            let list_box_inner = list_box_for_import.clone();
            let window_title_inner = window_title_for_import.clone();
            let toast_inner = toast_overlay_import.clone();
            let create_row_inner = create_row_import.clone();
            let trigger_probe_inner = trigger_probe_import.clone();

            ImportDialog::show(
                &window_for_import,
                move |imported_conns: Vec<Connection>, strategy: ImportConflictStrategy| {
                    if imported_conns.is_empty() {
                        return;
                    }

                    let (added, updated, skipped) = {
                        let mut st = state_inner.borrow_mut();
                        merge_imported_connections(&mut st.connections, imported_conns, strategy)
                    };
                    let _ = storage::save_connections(&state_inner.borrow().connections);

                    // Rebuild sidebar rows
                    while let Some(child) = list_box_inner.first_child() {
                        list_box_inner.remove(&child);
                    }
                    for conn in &state_inner.borrow().connections {
                        let row = create_row_inner(conn);
                        list_box_inner.append(&row);
                    }
                    list_box_inner.invalidate_filter();
                    list_box_inner.invalidate_sort();
                    list_box_inner.invalidate_headers();

                    window_title_inner.set_subtitle(&format!(
                        "{} connections",
                        state_inner.borrow().connections.len()
                    ));

                    toast_inner.add_toast(adw::Toast::new(&format!(
                        "Import finished: {} added, {} updated, {} skipped",
                        added, updated, skipped
                    )));

                    trigger_probe_inner();
                },
            );
        });

        // Export Action
        let window_for_export = window.clone();
        let state_for_export = state.clone();
        let open_export_dialog = Rc::new(move || {
            let st = state_for_export.borrow();
            let selected_conn = st
                .selected_id
                .as_ref()
                .and_then(|id| st.connections.iter().find(|c| &c.id == id));
            ExportDialog::show(&window_for_export, &st.connections, selected_conn);
        });

        // Shortcuts Dialog Action
        let window_for_shortcuts = window.clone();
        let open_shortcuts = Rc::new(move || {
            ShortcutsDialog::show(&window_for_shortcuts);
        });

        // Preferences Button Action
        let window_for_prefs = window.clone();
        let state_for_prefs = state.clone();
        let open_prefs = Rc::new(move || {
            let config_rc = Rc::new(RefCell::new(state_for_prefs.borrow().config.clone()));
            let prefs_dialog =
                PreferencesWindow::build_window(Some(&window_for_prefs), config_rc.clone());

            let state_for_close = state_for_prefs.clone();
            prefs_dialog.connect_close_request(move |_| {
                state_for_close.borrow_mut().config = config_rc.borrow().clone();
                gtk::glib::Propagation::Proceed
            });

            prefs_dialog.present();
        });

        let open_prefs_btn = open_prefs.clone();
        prefs_btn.connect_clicked(move |_| open_prefs_btn());

        // Discovery Button Action
        let window_for_disc = window.clone();
        let state_for_disc = state.clone();
        let list_box_for_disc = list_box.clone();
        let window_title_for_disc = window_title.clone();
        let create_row_disc = create_row.clone();
        let trigger_probe_disc = trigger_batch_probe.clone();

        let open_discovery = Rc::new(move || {
            let state_inner = state_for_disc.clone();
            let list_box_inner = list_box_for_disc.clone();
            let window_title_inner = window_title_for_disc.clone();
            let create_row_inner = create_row_disc.clone();
            let trigger_probe_inner = trigger_probe_disc.clone();

            let disc_window = DiscoveryDialog::build_window(
                Some(&window_for_disc),
                move |new_conn: Connection| {
                    state_inner.borrow_mut().connections.push(new_conn.clone());
                    let _ = storage::save_connections(&state_inner.borrow().connections);

                    let row = create_row_inner(&new_conn);
                    list_box_inner.append(&row);
                    list_box_inner.invalidate_filter();
                    list_box_inner.invalidate_sort();
                    list_box_inner.invalidate_headers();

                    list_box_inner.select_row(Some(&row));
                    window_title_inner.set_subtitle(&format!(
                        "{} connections",
                        state_inner.borrow().connections.len()
                    ));
                    trigger_probe_inner();
                },
            );
            disc_window.present();
        });

        let open_disc_btn = open_discovery.clone();
        discovery_btn.connect_clicked(move |_| open_disc_btn());

        // Register GIO Actions for Menu Items
        let qc_action = gio::SimpleAction::new("quick_connect", None);
        let qc_trigger = open_quick_connect.clone();
        qc_action.connect_activate(move |_, _| qc_trigger());
        app.add_action(&qc_action);

        let import_action = gio::SimpleAction::new("import", None);
        let import_trigger = open_import_dialog.clone();
        import_action.connect_activate(move |_, _| import_trigger());
        app.add_action(&import_action);

        let export_action = gio::SimpleAction::new("export", None);
        let export_trigger = open_export_dialog.clone();
        export_action.connect_activate(move |_, _| export_trigger());
        app.add_action(&export_action);

        let refresh_action = gio::SimpleAction::new("refresh", None);
        let refresh_trigger = trigger_batch_probe.clone();
        refresh_action.connect_activate(move |_, _| refresh_trigger());
        app.add_action(&refresh_action);

        let shortcuts_action = gio::SimpleAction::new("shortcuts", None);
        let shortcuts_trigger = open_shortcuts.clone();
        shortcuts_action.connect_activate(move |_, _| shortcuts_trigger());
        app.add_action(&shortcuts_action);

        let prefs_action = gio::SimpleAction::new("preferences", None);
        let prefs_trigger = open_prefs.clone();
        prefs_action.connect_activate(move |_, _| prefs_trigger());
        app.add_action(&prefs_action);

        let window_for_about = window.clone();
        let about_action = gio::SimpleAction::new("about", None);
        about_action.connect_activate(move |_, _| {
            let about = adw::AboutWindow::builder()
                .application_name("VER - Very Easy Remote")
                .developer_name("dawiisss")
                .version(env!("CARGO_PKG_VERSION"))
                .comments("GTK4 / Libadwaita Remote Connection Manager in Rust")
                .website("https://github.com/dawiisss/ver")
                .issue_url("https://github.com/dawiisss/ver/issues")
                .support_url("https://github.com/dawiisss/ver/discussions")
                .license_type(gtk::License::MitX11)
                .transient_for(&window_for_about)
                .modal(true)
                .build();
            about.present();
        });
        app.add_action(&about_action);

        let app_clone_quit = app.clone();
        let quit_action = gio::SimpleAction::new("quit", None);
        quit_action.connect_activate(move |_, _| {
            app_clone_quit.quit();
        });
        app.add_action(&quit_action);

        // Row Selection Callback & Editor View construction
        let state_for_select = state.clone();
        let content_stack_for_select = content_stack.clone();
        let editor_container_for_select = editor_container.clone();
        let list_box_for_select = list_box.clone();
        let launch_session_for_select = launch_session.clone();
        let show_session_for_select = show_external_session.clone();
        let create_row_for_select = create_row.clone();
        let toast_overlay_for_select = toast_overlay.clone();

        list_box.connect_row_selected(move |_, row_opt| {
            while let Some(child) = editor_container_for_select.first_child() {
                editor_container_for_select.remove(&child);
            }

            if let Some(row) = row_opt {
                let conn_id = row.widget_name().to_string();
                state_for_select.borrow_mut().selected_id = Some(conn_id.clone());

                let is_session_active = state_for_select
                    .borrow()
                    .active_sessions
                    .get(&conn_id)
                    .map(|s| s.is_running.load(Ordering::SeqCst))
                    .unwrap_or(false);

                let conn_opt = state_for_select
                    .borrow()
                    .connections
                    .iter()
                    .find(|c| c.id == conn_id)
                    .cloned();
                if let Some(conn) = conn_opt {
                    let password = secrets::get_password_sync(&conn.id)
                        .unwrap_or(None)
                        .unwrap_or_default();

                    // Wire Callbacks for ConnectionEditor
                    let state_on_save = state_for_select.clone();
                    let list_box_on_save = list_box_for_select.clone();
                    let row_on_save = row.clone();
                    let on_save = move |updated_conn: Connection, updated_pass: String| {
                        if let Some(c) = state_on_save
                            .borrow_mut()
                            .connections
                            .iter_mut()
                            .find(|c| c.id == updated_conn.id)
                        {
                            *c = updated_conn.clone();
                        }
                        let _ = storage::save_connections(&state_on_save.borrow().connections);

                        if updated_pass.is_empty() {
                            let _ = secrets::delete_password_sync(&updated_conn.id);
                        } else {
                            let _ = secrets::set_password_sync(&updated_conn.id, &updated_pass);
                        }

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

                    let launch_session_on_connect = launch_session_for_select.clone();
                    let on_connect = move |conn: Connection, pass: String| {
                        launch_session_on_connect(conn, pass);
                    };

                    let state_on_dup = state_for_select.clone();
                    let list_box_on_dup = list_box_for_select.clone();
                    let window_title_on_dup = window_title.clone();
                    let create_row_on_dup = create_row_for_select.clone();
                    let on_duplicate = move |dup_conn: Connection, dup_pass: String| {
                        state_on_dup.borrow_mut().connections.push(dup_conn.clone());
                        let _ = storage::save_connections(&state_on_dup.borrow().connections);

                        if !dup_pass.is_empty() {
                            let _ = secrets::set_password_sync(&dup_conn.id, &dup_pass);
                        }

                        let new_row = create_row_on_dup(&dup_conn);
                        list_box_on_dup.append(&new_row);
                        list_box_on_dup.invalidate_filter();
                        list_box_on_dup.invalidate_sort();
                        list_box_on_dup.invalidate_headers();

                        list_box_on_dup.select_row(Some(&new_row));
                        window_title_on_dup.set_subtitle(&format!(
                            "{} connections",
                            state_on_dup.borrow().connections.len()
                        ));
                    };

                    let state_on_del = state_for_select.clone();
                    let list_box_on_del = list_box_for_select.clone();
                    let window_title_on_del = window_title.clone();
                    let row_on_del = row.clone();
                    let on_delete = move |del_id: String| {
                        state_on_del
                            .borrow_mut()
                            .connections
                            .retain(|c| c.id != del_id);
                        let _ = storage::save_connections(&state_on_del.borrow().connections);
                        let _ = secrets::delete_password_sync(&del_id);

                        list_box_on_del.remove(&row_on_del);
                        list_box_on_del.unselect_all();
                        window_title_on_del.set_subtitle(&format!(
                            "{} connections",
                            state_on_del.borrow().connections.len()
                        ));
                    };

                    // Context-Aware Wake-on-LAN with Automated Polling
                    let state_on_wake = state_for_select.clone();
                    let list_box_on_wake = list_box_for_select.clone();
                    let toast_on_wake = toast_overlay_for_select.clone();
                    let conn_for_wake = conn.clone();

                    let on_wake = move |mac: String| {
                        let _ = network::send_wol(&mac);
                        let target_host = conn_for_wake.host.clone();
                        let target_port = conn_for_wake.port;
                        let target_id = conn_for_wake.id.clone();
                        let target_name = conn_for_wake.name.clone();

                        let state_poll = state_on_wake.clone();
                        let list_box_poll = list_box_on_wake.clone();
                        let toast_poll = toast_on_wake.clone();

                        // Set probing status immediately
                        state_poll
                            .borrow_mut()
                            .host_statuses
                            .insert(target_id.clone(), HostStatus::Probing);
                        set_row_status_dot(&list_box_poll, &target_id, &HostStatus::Probing);

                        // Spawn polling task: probe every 2 seconds for up to 30 seconds
                        glib::MainContext::default().spawn_local(async move {
                            for _ in 0..15 {
                                glib::timeout_future(Duration::from_millis(2000)).await;
                                let status = prober::probe_host_async(
                                    target_host.clone(),
                                    target_port,
                                    Duration::from_millis(1500),
                                )
                                .await;

                                if status.is_online() {
                                    state_poll
                                        .borrow_mut()
                                        .host_statuses
                                        .insert(target_id.clone(), status.clone());
                                    set_row_status_dot(&list_box_poll, &target_id, &status);
                                    toast_poll.add_toast(adw::Toast::new(&format!(
                                        "Host '{}' is now online!",
                                        target_name
                                    )));
                                    return;
                                }
                            }

                            let final_status = HostStatus::Offline {
                                reason: "Host did not wake up within 30s".to_string(),
                            };
                            state_poll
                                .borrow_mut()
                                .host_statuses
                                .insert(target_id.clone(), final_status.clone());
                            set_row_status_dot(&list_box_poll, &target_id, &final_status);
                        });
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

                    if is_session_active {
                        let banner = adw::Banner::builder()
                            .title("An active session is currently running for this connection")
                            .button_label("View Session")
                            .revealed(true)
                            .build();
                        let show_session_for_banner = show_session_for_select.clone();
                        let id_for_banner = conn_id.clone();
                        banner.connect_button_clicked(move |_| {
                            show_session_for_banner(&id_for_banner);
                        });
                        editor_container_for_select.append(&banner);
                    }

                    editor_container_for_select.append(&editor_widget);

                    if is_session_active {
                        show_session_for_select(&conn_id);
                    } else {
                        content_stack_for_select.set_visible_child_name("editor");
                    }
                }
            } else {
                state_for_select.borrow_mut().selected_id = None;
                content_stack_for_select.set_visible_child_name("welcome");
            }
        });

        // Global Keyboard Event Controller (Accelerators)
        let key_controller = gtk::EventControllerKey::new();
        let add_conn_key = add_conn_action.clone();
        let quick_connect_key = open_quick_connect.clone();
        let import_key = open_import_dialog.clone();
        let export_key = open_export_dialog.clone();
        let shortcuts_key = open_shortcuts.clone();
        let prefs_key = open_prefs.clone();
        let disc_key = open_discovery.clone();
        let probe_key = trigger_batch_probe.clone();
        let search_toggle_key = search_toggle.clone();
        let search_entry_key = search_entry.clone();
        let state_key = state.clone();
        let list_box_key = list_box.clone();
        let launch_session_key = launch_session.clone();
        let app_key = app.clone();

        key_controller.connect_key_pressed(move |_, keyval, _keycode, state_flags| {
            let ctrl = state_flags.contains(gdk::ModifierType::CONTROL_MASK);

            if ctrl {
                match keyval {
                    gdk::Key::k | gdk::Key::K => {
                        quick_connect_key();
                        return glib::Propagation::Stop;
                    }
                    gdk::Key::n | gdk::Key::N => {
                        add_conn_key();
                        return glib::Propagation::Stop;
                    }
                    gdk::Key::f | gdk::Key::F => {
                        let active = !search_toggle_key.is_active();
                        search_toggle_key.set_active(active);
                        if active {
                            search_entry_key.grab_focus();
                        }
                        return glib::Propagation::Stop;
                    }
                    gdk::Key::i | gdk::Key::I => {
                        import_key();
                        return glib::Propagation::Stop;
                    }
                    gdk::Key::e | gdk::Key::E => {
                        export_key();
                        return glib::Propagation::Stop;
                    }
                    gdk::Key::d | gdk::Key::D => {
                        disc_key();
                        return glib::Propagation::Stop;
                    }
                    gdk::Key::comma => {
                        prefs_key();
                        return glib::Propagation::Stop;
                    }
                    gdk::Key::question | gdk::Key::slash => {
                        shortcuts_key();
                        return glib::Propagation::Stop;
                    }
                    gdk::Key::r | gdk::Key::R => {
                        probe_key();
                        return glib::Propagation::Stop;
                    }
                    gdk::Key::q | gdk::Key::Q => {
                        app_key.quit();
                        return glib::Propagation::Stop;
                    }
                    _ => {}
                }
            } else {
                match keyval {
                    gdk::Key::F5 => {
                        probe_key();
                        return glib::Propagation::Stop;
                    }
                    gdk::Key::F1 => {
                        shortcuts_key();
                        return glib::Propagation::Stop;
                    }
                    gdk::Key::Return => {
                        if let Some(sel_row) = list_box_key.selected_row() {
                            let id = sel_row.widget_name().to_string();
                            let conn_opt = state_key
                                .borrow()
                                .connections
                                .iter()
                                .find(|c| c.id == id)
                                .cloned();
                            if let Some(conn) = conn_opt {
                                let pass = secrets::get_password_sync(&conn.id)
                                    .unwrap_or(None)
                                    .unwrap_or_default();
                                launch_session_key(conn, pass);
                                return glib::Propagation::Stop;
                            }
                        }
                    }
                    _ => {}
                }
            }

            glib::Propagation::Proceed
        });
        window.add_controller(key_controller);

        // Auto-connect Last Session on startup
        let auto_connect_target = {
            let st = state.borrow();
            if st.config.auto_connect_last {
                st.config
                    .last_connected_id
                    .as_ref()
                    .and_then(|last_id| st.connections.iter().find(|c| &c.id == last_id).cloned())
            } else {
                None
            }
        };

        if let Some(conn) = auto_connect_target {
            let mut child_opt = list_box.first_child();
            while let Some(child) = child_opt {
                if let Ok(row) = child.clone().downcast::<gtk::ListBoxRow>() {
                    if row.widget_name() == conn.id.as_str() {
                        list_box.select_row(Some(&row));
                        break;
                    }
                }
                child_opt = child.next_sibling();
            }

            let password = secrets::get_password_sync(&conn.id)
                .unwrap_or(None)
                .unwrap_or_default();
            launch_session(conn, password);
        }

        window
    }
}

fn setup_group_headers(list_box: &gtk::ListBox, state: Rc<RefCell<AppWindowState>>) {
    list_box.set_header_func(move |row, before| {
        let st = state.borrow();
        let conn = st
            .connections
            .iter()
            .find(|c| c.id == row.widget_name().as_str());
        let before_conn = before.and_then(|b| {
            st.connections
                .iter()
                .find(|c| c.id == b.widget_name().as_str())
                .cloned()
        });

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
        let c1 = st
            .connections
            .iter()
            .find(|c| c.id == row1.widget_name().as_str());
        let c2 = st
            .connections
            .iter()
            .find(|c| c.id == row2.widget_name().as_str());

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

fn setup_filtering(
    list_box: &gtk::ListBox,
    search_entry: &gtk::SearchEntry,
    state: Rc<RefCell<AppWindowState>>,
) {
    let state_for_filter = state.clone();
    list_box.set_filter_func(move |row| {
        let st = state_for_filter.borrow();
        let query = st.search_query.trim().to_lowercase();
        if query.is_empty() {
            return true;
        }

        if let Some(c) = st
            .connections
            .iter()
            .find(|c| c.id == row.widget_name().as_str())
        {
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

fn find_and_set_badge(widget: &gtk::Widget, is_active: bool) -> bool {
    if widget.widget_name() == "active_badge" {
        widget.set_visible(is_active);
        return true;
    }
    let mut child = widget.first_child();
    while let Some(c) = child {
        if find_and_set_badge(&c, is_active) {
            return true;
        }
        child = c.next_sibling();
    }
    false
}

fn set_row_active_badge(list_box: &gtk::ListBox, conn_id: &str, is_active: bool) {
    let mut child_opt = list_box.first_child();
    while let Some(child) = child_opt {
        if let Ok(row) = child.clone().downcast::<gtk::ListBoxRow>() {
            if row.widget_name() == conn_id {
                let mut next_w = row.first_child();
                while let Some(w) = next_w {
                    if find_and_set_badge(&w, is_active) {
                        return;
                    }
                    next_w = w.next_sibling();
                }
                break;
            }
        }
        child_opt = child.next_sibling();
    }
}

fn find_and_set_status_dot(widget: &gtk::Widget, status: &HostStatus) -> bool {
    if widget.widget_name() == "status_dot" {
        if let Ok(b) = widget.clone().downcast::<gtk::Box>() {
            b.set_css_classes(&["status-dot", status.status_css_class()]);
            b.set_tooltip_text(Some(&status.description()));
            return true;
        }
    }
    let mut child = widget.first_child();
    while let Some(c) = child {
        if find_and_set_status_dot(&c, status) {
            return true;
        }
        child = c.next_sibling();
    }
    false
}

fn set_row_status_dot(list_box: &gtk::ListBox, conn_id: &str, status: &HostStatus) {
    let mut child_opt = list_box.first_child();
    while let Some(child) = child_opt {
        if let Ok(row) = child.clone().downcast::<gtk::ListBoxRow>() {
            if row.widget_name() == conn_id {
                let mut next_w = row.first_child();
                while let Some(w) = next_w {
                    if find_and_set_status_dot(&w, status) {
                        return;
                    }
                    next_w = w.next_sibling();
                }
                break;
            }
        }
        child_opt = child.next_sibling();
    }
}
