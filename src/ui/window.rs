use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use crate::launcher;
use crate::models::{AppConfig, Connection, Protocol};
use crate::network;
use crate::secrets;
use crate::storage;
use crate::ui::discovery::DiscoveryDialog;
use crate::ui::editor::ConnectionEditor;
use crate::ui::preferences::{apply_theme, PreferencesWindow};

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
    pub fn build_ui(
        app: &adw::Application,
        connections: Vec<Connection>,
        config: AppConfig,
    ) -> adw::ApplicationWindow {
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
        menu.append(Some("Preferences"), Some("app.preferences"));
        menu.append(Some("About VER"), Some("app.about"));
        menu.append(Some("Quit"), Some("app.quit"));
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
            .description("Select a connection from the sidebar or add a new one to get started.")
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

        // Action: About VER
        let window_clone = window.clone();
        let about_action = gio::SimpleAction::new("about", None);
        about_action.connect_activate(move |_, _| {
            let about = adw::AboutWindow::builder()
                .application_name("VER - Very Easy Remote")
                .developer_name("VER Team")
                .version("1.0.0")
                .comments("GTK4 / Libadwaita Remote Connection Manager in Rust")
                .license_type(gtk::License::Gpl30)
                .transient_for(&window_clone)
                .modal(true)
                .build();
            about.present();
        });
        app.add_action(&about_action);

        // Action: Preferences
        let window_for_menu_prefs = window.clone();
        let state_for_menu_prefs = state.clone();
        let prefs_action = gio::SimpleAction::new("preferences", None);
        prefs_action.connect_activate(move |_, _| {
            let config_rc = Rc::new(RefCell::new(state_for_menu_prefs.borrow().config.clone()));
            let prefs_dialog =
                PreferencesWindow::build_window(Some(&window_for_menu_prefs), config_rc.clone());
            let state_for_close = state_for_menu_prefs.clone();
            prefs_dialog.connect_close_request(move |_| {
                state_for_close.borrow_mut().config = config_rc.borrow().clone();
                gtk::glib::Propagation::Proceed
            });
            prefs_dialog.present();
        });
        app.add_action(&prefs_action);

        // Action: Quit
        let app_clone_quit = app.clone();
        let quit_action = gio::SimpleAction::new("quit", None);
        quit_action.connect_activate(move |_, _| {
            app_clone_quit.quit();
        });
        app.add_action(&quit_action);

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
                let default_protocol = state.borrow().config.default_protocol;
                let new_conn = Connection::new_with_protocol(default_protocol);
                state.borrow_mut().connections.push(new_conn.clone());
                let _ = storage::save_connections(&state.borrow().connections);

                let row = create_row(&new_conn);
                list_box.append(&row);
                list_box.invalidate_filter();
                list_box.invalidate_sort();
                list_box.invalidate_headers();

                list_box.select_row(Some(&row));
                window_title
                    .set_subtitle(&format!("{} connections", state.borrow().connections.len()));
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
            let prefs_dialog =
                PreferencesWindow::build_window(Some(&window_for_prefs), config_rc.clone());

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
                    window_title_inner.set_subtitle(&format!(
                        "{} connections",
                        state_inner.borrow().connections.len()
                    ));
                },
            );
            disc_window.present();
        });

        // External Session Tracking and Launching
        enum ExternalSessionEvent {
            Log(String),
            Exit(bool), // true if success (0), false if error
        }

        let external_session_container_clone = external_session_container.clone();
        let content_stack_clone = content_stack.clone();

        let track_external_session =
            Rc::new(move |mut child: std::process::Child, name: String| {
                let container = external_session_container_clone.clone();
                let stack = content_stack_clone.clone();

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

                let child_arc = std::sync::Arc::new(std::sync::Mutex::new(Some(child)));
                let child_arc_for_btn = child_arc.clone();
                let stack_for_btn = stack.clone();

                btn_disconnect.connect_clicked(move |_| {
                    let mut opt = child_arc_for_btn.lock().unwrap();
                    if let Some(mut c) = opt.take() {
                        std::thread::spawn(move || {
                            #[cfg(unix)]
                            {
                                let pid = c.id();
                                unsafe {
                                    libc::kill(pid as i32, libc::SIGTERM);
                                }
                            }

                            let mut exited = false;
                            for _ in 0..20 {
                                // wait up to 2 seconds
                                if let Ok(Some(_)) = c.try_wait() {
                                    exited = true;
                                    break;
                                }
                                std::thread::sleep(std::time::Duration::from_millis(100));
                            }

                            if !exited {
                                let _ = c.kill();
                                let _ = c.wait();
                            }
                        });
                    }
                    stack_for_btn.set_visible_child_name("editor");
                });

                container.append(&title);
                container.append(&btn_disconnect);
                container.append(&scroll);
                stack.set_visible_child_name("external_session");

                let child_arc_for_thread = child_arc.clone();
                let tx_exit = tx.clone();

                std::thread::spawn(move || loop {
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
            });

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
                    track_session_for_launch(child, conn.name.clone());
                }
                Err(err) => {
                    toast_overlay_for_launch
                        .add_toast(adw::Toast::new(&format!("Connection failed: {}", err)));
                }
            }
        });

        // Row Selection Callback & Editor View construction
        let state_for_select = state.clone();
        let content_stack_for_select = content_stack.clone();
        let editor_container_for_select = editor_container.clone();
        let list_box_for_select = list_box.clone();
        let launch_session_for_select = launch_session.clone();

        list_box.connect_row_selected(move |_, row_opt| {
            while let Some(child) = editor_container_for_select.first_child() {
                editor_container_for_select.remove(&child);
            }

            if let Some(row) = row_opt {
                let conn_id = row.widget_name().to_string();
                state_for_select.borrow_mut().selected_id = Some(conn_id.clone());

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
                        // 1. Update connection vector
                        if let Some(c) = state_on_save
                            .borrow_mut()
                            .connections
                            .iter_mut()
                            .find(|c| c.id == updated_conn.id)
                        {
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

                    let launch_session_on_connect = launch_session_for_select.clone();
                    let on_connect = move |conn: Connection, pass: String| {
                        launch_session_on_connect(conn, pass);
                    };

                    let state_on_dup = state_for_select.clone();
                    let list_box_on_dup = list_box_for_select.clone();
                    let window_title_on_dup = window_title.clone();
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
