use gtk::gdk;
use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::Rc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use crate::importers::{merge_imported_connections, ImportConflictStrategy};
use crate::models::{recent_connections, AppConfig, Connection, Protocol};
use crate::network;
use crate::prober::{self, HostStatus};
use crate::secrets;
use crate::storage;
use crate::tray;
use crate::ui::discovery::{DiscoveredService, DiscoveryDialog};
use crate::ui::editor::ConnectionEditor;
use crate::ui::export_dialog::ExportDialog;
use crate::ui::import_dialog::ImportDialog;
use crate::ui::preferences::{apply_theme, PreferencesWindow};
use crate::ui::quick_connect::QuickConnectDialog;
use crate::ui::session::{self, ActiveSession};
use crate::ui::shortcuts::ShortcutsDialog;
use crate::ui::sidebar::{
    create_connection_row, find_row, set_row_status_dot, setup_filtering, setup_group_headers,
    setup_sorting, update_connection_row, STATUS_DOT_CSS,
};

pub struct AppWindowState {
    pub connections: Vec<Connection>,
    pub selected_id: Option<String>,
    pub search_query: String,
    pub config: AppConfig,
    pub active_sessions: HashMap<String, ActiveSession>,
    pub host_statuses: HashMap<String, HostStatus>,
    /// Connections whose SSH tunnel is still opening.
    pub pending_tunnels: HashSet<String>,
}

pub struct MainWindow {
    pub connections: Vec<Connection>,
    pub selected_id: Option<String>,
    pub search_filter: String,
    pub config: AppConfig,
    pub window: Option<adw::ApplicationWindow>,
}

fn commit_connections<T>(
    state: &Rc<RefCell<AppWindowState>>,
    toast: &adw::ToastOverlay,
    update: impl FnOnce(&mut Vec<Connection>) -> T,
) -> Option<T> {
    let result = storage::update_connections(&mut state.borrow_mut().connections, update);
    match result {
        Ok(value) => {
            let st = state.borrow();
            tray::update_launch_menu(&st.config, &st.connections);
            Some(value)
        }
        Err(error) => {
            toast.add_toast(adw::Toast::new(&format!(
                "Connections were not saved: {:#}",
                error
            )));
            None
        }
    }
}

async fn save_password(id: &str, password: &str, toast: &adw::ToastOverlay) -> bool {
    let result = if password.is_empty() {
        secrets::delete_password(id).await
    } else {
        secrets::set_password(id, password).await
    };
    match result {
        Ok(()) => true,
        Err(error) => {
            toast.add_toast(adw::Toast::new(&format!(
                "Connection settings saved, but the password change failed: {:#}",
                error
            )));
            false
        }
    }
}

fn launch_with_stored_password<F>(conn: Connection, launch: Rc<F>, toast: &adw::ToastOverlay)
where
    F: Fn(Connection, String) + 'static,
{
    if conn.protocol == Protocol::Ssh {
        launch(conn, String::new());
        return;
    }
    let toast = toast.clone();
    glib::MainContext::default().spawn_local(async move {
        match secrets::get_password(&conn.id).await {
            Ok(password) => launch(conn, password.unwrap_or_default()),
            Err(error) => toast.add_toast(adw::Toast::new(&format!(
                "Could not load password. Enter it in the editor to connect: {:#}",
                error
            ))),
        }
    });
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
        css_provider.load_from_data(STATUS_DOT_CSS);
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
            pending_tunnels: HashSet::new(),
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

        let recent_menu = gio::Menu::new();
        let recent_btn = gtk::MenuButton::builder()
            .icon_name("document-open-recent-symbolic")
            .tooltip_text("Recent Connections")
            .menu_model(&recent_menu)
            .build();

        let search_toggle = gtk::ToggleButton::builder()
            .icon_name("system-search-symbolic")
            .tooltip_text("Search Connections (Ctrl+F)")
            .build();

        header_bar.pack_start(&add_btn);
        header_bar.pack_start(&quick_btn);
        header_bar.pack_start(&recent_btn);
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
        let loading = adw::StatusPage::builder()
            .title("Loading Connection")
            .description("Retrieving the saved password…")
            .child(&gtk::Spinner::builder().spinning(true).build())
            .build();
        content_stack.add_named(&loading, Some("loading"));

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

        if storage::was_corrupt_backup_failed() {
            let toast = adw::Toast::new(
                "A corrupt configuration file could not be backed up; VER will not save over it",
            );
            toast.set_timeout(10);
            toast_overlay.add_toast(toast);
        } else if storage::was_corrupt_backup_created() {
            let toast = adw::Toast::new(
                "Warning: Corrupt configuration file detected; backup created (.corrupt)",
            );
            toast.set_timeout(10);
            toast_overlay.add_toast(toast);
        }

        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("VER - Connection Manager")
            .icon_name("io.github.dawiisss.ver")
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
            create_connection_row(conn, state_for_row.clone())
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
        setup_group_headers(&list_box, state.clone());

        // Sort Func (Group ASC -> Name ASC)
        setup_sorting(&list_box, state.clone());

        // Filter Func
        setup_filtering(&list_box, &search_entry, state.clone());

        let show_external_session = {
            let container = external_session_container.clone();
            let stack = content_stack.clone();
            let state_for_show = state.clone();
            let list_box_for_show = list_box.clone();
            Rc::new(move |conn_id: &str| {
                session::show_external_session(
                    &container,
                    &stack,
                    state_for_show.clone(),
                    &list_box_for_show,
                    conn_id,
                );
            })
        };

        let state_for_launch = state.clone();
        let toast_overlay_for_launch = toast_overlay.clone();
        let list_box_for_launch = list_box.clone();
        let stack_for_launch = content_stack.clone();
        let container_for_launch = external_session_container.clone();
        let launch_session = Rc::new(move |conn: Connection, pass: String| {
            session::launch_session(
                conn,
                pass,
                state_for_launch.clone(),
                &toast_overlay_for_launch,
                &list_box_for_launch,
                &stack_for_launch,
                &container_for_launch,
            );
        });

        // Background Batch Reachability Prober
        let trigger_batch_probe = {
            let state_for_probe = state.clone();
            let list_box_for_probe = list_box.clone();
            // Results from a superseded batch must not overwrite newer ones.
            let probe_generation = Rc::new(Cell::new(0u64));
            Rc::new(move || {
                let generation = probe_generation.get().wrapping_add(1);
                probe_generation.set(generation);
                let targets: Vec<(String, String, u16)> = {
                    let st = state_for_probe.borrow();
                    st.connections
                        .iter()
                        .map(|c| {
                            let (host, port) = c.probe_target();
                            (c.id.clone(), host, port)
                        })
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
                let probe_generation = probe_generation.clone();

                let (tx, rx) = async_channel::unbounded::<(String, HostStatus)>();
                prober::spawn_batch_probe(targets, Duration::from_millis(2000), 8, tx);

                glib::MainContext::default().spawn_local(async move {
                    while let Ok((id, status)) = rx.recv().await {
                        if probe_generation.get() != generation {
                            break;
                        }
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
            let toast = toast_overlay.clone();
            Rc::new(move || {
                let default_protocol = state.borrow().config.default_protocol;
                let mut new_conn = Connection::new_with_protocol(default_protocol);
                new_conn.advanced_settings.vnc_color_level =
                    state.borrow().config.default_vnc_color_level;
                if commit_connections(&state, &toast, |connections| {
                    connections.push(new_conn.clone())
                })
                .is_none()
                {
                    return;
                }

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
        let toast_for_qc = toast_overlay.clone();

        let open_quick_connect = Rc::new(move || {
            let default_proto = state_for_qc.borrow().config.default_protocol;
            let launch_connect = launch_session_for_qc.clone();
            let state_save = state_for_qc.clone();
            let list_box_save = list_box_for_qc.clone();
            let window_title_save = window_title_for_qc.clone();
            let launch_save = launch_session_for_qc.clone();
            let create_row_save = create_row_for_qc.clone();
            let toast_save = toast_for_qc.clone();

            QuickConnectDialog::show(
                &window_for_qc,
                default_proto,
                move |conn, pass| {
                    launch_connect(conn, pass.unwrap_or_default());
                },
                move |conn, pass| {
                    let pass_str = pass.unwrap_or_default();
                    if commit_connections(&state_save, &toast_save, |connections| {
                        connections.push(conn.clone())
                    })
                    .is_none()
                    {
                        return false;
                    }
                    let row = create_row_save(&conn);
                    list_box_save.append(&row);
                    list_box_save.invalidate_filter();
                    list_box_save.invalidate_sort();
                    list_box_save.invalidate_headers();
                    window_title_save.set_subtitle(&format!(
                        "{} connections",
                        state_save.borrow().connections.len()
                    ));
                    let selection_before = list_box_save.selected_row();
                    let state_save = state_save.clone();
                    let list_box_save = list_box_save.clone();
                    let toast_save = toast_save.clone();
                    let launch_save = launch_save.clone();
                    glib::MainContext::default().spawn_local(async move {
                        if !pass_str.is_empty() {
                            save_password(&conn.id, &pass_str, &toast_save).await;
                        }
                        if !state_save
                            .borrow()
                            .connections
                            .iter()
                            .any(|c| c.id == conn.id)
                        {
                            return;
                        }
                        if row.parent().is_some()
                            && list_box_save.selected_row() == selection_before
                        {
                            list_box_save.select_row(Some(&row));
                        }
                        launch_save(conn, pass_str);
                    });
                    true
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
                        return false;
                    }

                    let Some(((added, updated, skipped), orphaned_ids)) =
                        commit_connections(&state_inner, &toast_inner, |connections| {
                            let before: Vec<String> =
                                connections.iter().map(|c| c.id.clone()).collect();
                            let counts =
                                merge_imported_connections(connections, imported_conns, strategy);
                            let orphaned: Vec<String> = before
                                .into_iter()
                                .filter(|id| !connections.iter().any(|c| &c.id == id))
                                .collect();
                            (counts, orphaned)
                        })
                    else {
                        return false;
                    };

                    // Overwrites that changed endpoint got new IDs; drop the old passwords.
                    for id in orphaned_ids {
                        let toast = toast_inner.clone();
                        glib::MainContext::default().spawn_local(async move {
                            if let Err(error) = secrets::delete_password(&id).await {
                                toast.add_toast(adw::Toast::new(&format!(
                                    "An overwritten connection's old password could not be removed: {:#}",
                                    error
                                )));
                            }
                        });
                    }

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
                    true
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
                let mut config = config_rc.borrow().clone();
                let mut st = state_for_close.borrow_mut();
                // Connections launched from the tray while Preferences was open stay recent.
                config.last_connected_id = st.config.last_connected_id.clone();
                config.recent_connection_ids = st.config.recent_connection_ids.clone();
                st.config = config;
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
        let toast_disc = toast_overlay.clone();

        let open_discovery = Rc::new(move || {
            let state_inner = state_for_disc.clone();
            let list_box_inner = list_box_for_disc.clone();
            let window_title_inner = window_title_for_disc.clone();
            let create_row_inner = create_row_disc.clone();
            let trigger_probe_inner = trigger_probe_disc.clone();
            let toast = toast_disc.clone();

            let state_for_check = state_inner.clone();
            let is_already_added = move |service: &DiscoveredService| -> bool {
                DiscoveryDialog::is_service_in_connections(
                    service,
                    &state_for_check.borrow().connections,
                )
            };

            let disc_window = DiscoveryDialog::build_window(
                Some(&window_for_disc),
                is_already_added,
                move |new_conn: Connection| {
                    if commit_connections(&state_inner, &toast, |connections| {
                        connections.push(new_conn.clone())
                    })
                    .is_none()
                    {
                        return false;
                    }

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
                    true
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
                .application_name("VER - Very Easy Remote Manager")
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
        let window_title_for_select = window_title.clone();

        let selection_request = Rc::new(Cell::new(0u64));
        list_box.connect_row_selected(move |_, row_opt| {
            let request = selection_request.get().wrapping_add(1);
            selection_request.set(request);
            let selection_request = selection_request.clone();
            while let Some(child) = editor_container_for_select.first_child() {
                editor_container_for_select.remove(&child);
            }

            state_for_select.borrow_mut().selected_id = row_opt.map(|row| row.widget_name().to_string());
            let row_opt = row_opt.cloned();
            let state_for_select = state_for_select.clone();
            let editor_container_for_select = editor_container_for_select.clone();
            let content_stack_for_select = content_stack_for_select.clone();
            let list_box_for_select = list_box_for_select.clone();
            let launch_session_for_select = launch_session_for_select.clone();
            let show_session_for_select = show_session_for_select.clone();
            let create_row_for_select = create_row_for_select.clone();
            let toast_overlay_for_select = toast_overlay_for_select.clone();
            let window_title_for_select = window_title_for_select.clone();
            content_stack_for_select.set_visible_child_name(if row_opt.is_some() { "loading" } else { "welcome" });
            glib::MainContext::default().spawn_local(async move {
                if selection_request.get() != request {
                    return;
                }
                if let Some(row) = row_opt {
                    let conn_id = row.widget_name().to_string();

                    let conn_opt = state_for_select
                        .borrow()
                        .connections
                        .iter()
                        .find(|c| c.id == conn_id)
                        .cloned();
                    if let Some(conn) = conn_opt {
                        let result = if conn.protocol == Protocol::Ssh {
                            Ok(None)
                        } else {
                            secrets::get_password(&conn.id).await
                        };
                        // A slow keyring response must never replace a newer selection's editor.
                        if selection_request.get() != request
                            || list_box_for_select.selected_row().as_ref() != Some(&row)
                        {
                            return;
                        }
                        let is_session_active = state_for_select
                            .borrow()
                            .active_sessions
                            .get(&conn_id)
                            .map(|s| s.is_running.load(Ordering::SeqCst))
                            .unwrap_or(false);

                        let original_password = match result {
                            Ok(password) => Some(password.unwrap_or_default()),
                            Err(error) => {
                                toast_overlay_for_select.add_toast(adw::Toast::new(&format!("Could not load password; it will be left unchanged unless you enter a replacement: {:#}", error)));
                                None
                            }
                        };
                        let password = original_password.clone().unwrap_or_default();
                        let saved_password = Rc::new(RefCell::new(original_password));

                        // Wire Callbacks for ConnectionEditor
                        let state_on_save = state_for_select.clone();
                        let list_box_on_save = list_box_for_select.clone();
                        let toast_on_save = toast_overlay_for_select.clone();
                        let row_on_save = row.clone();
                        let on_save = move |mut updated_conn: Connection, updated_pass: String| {
                            // The pin may have been toggled since this editor was opened.
                            if let Some(current) = state_on_save.borrow().connections.iter().find(|c| c.id == updated_conn.id) {
                                updated_conn.pinned = current.pinned;
                            }
                            if commit_connections(&state_on_save, &toast_on_save, |connections| {
                                if let Some(c) = connections.iter_mut().find(|c| c.id == updated_conn.id) {
                                    *c = updated_conn.clone();
                                }
                            })
                            .is_none()
                            {
                                return;
                            }

                            let password_changed = saved_password
                                .borrow()
                                .as_ref()
                                .map(|old| old != &updated_pass)
                                .unwrap_or(!updated_pass.is_empty());
                            if password_changed {
                                let id = updated_conn.id.clone();
                                let toast = toast_on_save.clone();
                                let saved = saved_password.clone();
                                glib::MainContext::default().spawn_local(async move {
                                    if save_password(&id, &updated_pass, &toast).await {
                                        *saved.borrow_mut() = Some(updated_pass);
                                        toast.add_toast(adw::Toast::new("Connection and password saved"));
                                    }
                                });
                            } else {
                                toast_on_save.add_toast(adw::Toast::new("Connection settings saved"));
                            }

                            update_connection_row(&row_on_save, &updated_conn);

                            let (host_probe, port_probe) = updated_conn.probe_target();
                            let id_probe = updated_conn.id.clone();
                            let state_probe = state_on_save.clone();
                            let list_box_probe = list_box_on_save.clone();
                            glib::MainContext::default().spawn_local(async move {
                                let status = prober::probe_host_async(
                                    host_probe,
                                    port_probe,
                                    Duration::from_millis(1500),
                                )
                                .await;
                                state_probe
                                    .borrow_mut()
                                    .host_statuses
                                    .insert(id_probe.clone(), status.clone());
                                set_row_status_dot(&list_box_probe, &id_probe, &status);
                            });

                            list_box_on_save.invalidate_sort();
                            list_box_on_save.invalidate_headers();
                        };

                        let launch_session_on_connect = launch_session_for_select.clone();
                        let on_connect = move |conn: Connection, pass: String| {
                            launch_session_on_connect(conn, pass);
                        };

                        let state_on_dup = state_for_select.clone();
                        let list_box_on_dup = list_box_for_select.clone();
                        let window_title_on_dup = window_title_for_select.clone();
                        let toast_on_dup = toast_overlay_for_select.clone();
                        let create_row_on_dup = create_row_for_select.clone();
                        let on_duplicate = move |mut dup_conn: Connection, dup_pass: String| {
                            dup_conn.pinned = false;
                            if commit_connections(&state_on_dup, &toast_on_dup, |connections| {
                                connections.push(dup_conn.clone())
                            })
                            .is_none()
                            {
                                return;
                            }
                            let new_row = create_row_on_dup(&dup_conn);
                            list_box_on_dup.append(&new_row);
                            list_box_on_dup.invalidate_filter();
                            list_box_on_dup.invalidate_sort();
                            list_box_on_dup.invalidate_headers();
                            window_title_on_dup.set_subtitle(&format!(
                                "{} connections",
                                state_on_dup.borrow().connections.len()
                            ));
                            let selection_before = list_box_on_dup.selected_row();
                            let list_box = list_box_on_dup.clone();
                            let toast = toast_on_dup.clone();
                            glib::MainContext::default().spawn_local(async move {
                                if !dup_pass.is_empty() {
                                    save_password(&dup_conn.id, &dup_pass, &toast).await;
                                }
                                if new_row.parent().is_some() && list_box.selected_row() == selection_before {
                                    list_box.select_row(Some(&new_row));
                                }
                                toast.add_toast(adw::Toast::new("Connection duplicated"));
                            });
                        };

                        let state_on_del = state_for_select.clone();
                        let list_box_on_del = list_box_for_select.clone();
                        let window_title_on_del = window_title_for_select.clone();
                        let toast_on_del = toast_overlay_for_select.clone();
                        let row_on_del = row.clone();
                        let on_delete = move |del_id: String| {
                            if commit_connections(&state_on_del, &toast_on_del, |connections| {
                                connections.retain(|c| c.id != del_id)
                            })
                            .is_none()
                            {
                                return;
                            }
                            let toast = toast_on_del.clone();
                            glib::MainContext::default().spawn_local(async move {
                                        if let Err(error) = secrets::delete_password(&del_id).await {
                                            toast.add_toast(adw::Toast::new(&format!("Connection deleted, but its stored password could not be removed: {:#}", error)));
                                        }
                                    });

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

                        let on_wake =
                            move |mac: String, target_host: String, target_port: u16, target_name: String| {
                                let toast_poll = toast_on_wake.clone();
                                if let Err(error) = network::send_wol(&mac) {
                                    toast_poll.add_toast(adw::Toast::new(&format!(
                                        "Wake-on-LAN packet was not sent: {}",
                                        error
                                    )));
                                    return;
                                }
                                toast_poll.add_toast(adw::Toast::new("Wake-on-LAN packet sent"));
                                let target_id = conn_for_wake.id.clone();

                                let state_poll = state_on_wake.clone();
                                let list_box_poll = list_box_on_wake.clone();

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
        let toast_overlay_key = toast_overlay.clone();
        let window_title_key = window_title.clone();
        let window_key = window.clone();

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
                    gdk::Key::p | gdk::Key::P => {
                        if let Some(sel_row) = list_box_key.selected_row() {
                            let id = sel_row.widget_name().to_string();
                            ActionGroupExt::activate_action(&window_key, "toggle-pin", Some(&id.to_variant()));
                        }
                        return glib::Propagation::Stop;
                    }
                    gdk::Key::q | gdk::Key::Q => {
                        app_key.quit();
                        return glib::Propagation::Stop;
                    }
                    _ => {}
                }
            } else {
                let sidebar_has_focus = GtkWindowExt::focus(&window_key)
                    .map(|w| {
                        w == list_box_key.clone().upcast::<gtk::Widget>()
                            || w.is_ancestor(&list_box_key)
                    })
                    .unwrap_or(false);

                match keyval {
                    gdk::Key::F5 => {
                        probe_key();
                        return glib::Propagation::Stop;
                    }
                    gdk::Key::F1 => {
                        shortcuts_key();
                        return glib::Propagation::Stop;
                    }
                    gdk::Key::Delete if sidebar_has_focus => {
                        if let Some(sel_row) = list_box_key.selected_row() {
                            let id = sel_row.widget_name().to_string();
                            let conn_opt = state_key
                                .borrow()
                                .connections
                                .iter()
                                .find(|c| c.id == id)
                                .cloned();
                            if let Some(conn) = conn_opt {
                                let dialog = adw::MessageDialog::builder()
                                    .heading("Delete Connection?")
                                    .body(format!(
                                        "Are you sure you want to delete \"{}\"? This action cannot be undone.",
                                        conn.name
                                    ))
                                    .build();
                                dialog.add_response("cancel", "Cancel");
                                dialog.add_response("delete", "Delete");
                                dialog.set_response_appearance(
                                    "delete",
                                    adw::ResponseAppearance::Destructive,
                                );
                                dialog.set_default_response(Some("cancel"));
                                dialog.set_close_response("cancel");

                                let state_del = state_key.clone();
                                let list_box_del = list_box_key.clone();
                                let window_title_del = window_title_key.clone();
                                let toast_del = toast_overlay_key.clone();
                                let sel_row_del = sel_row.clone();
                                let del_id = conn.id.clone();

                                dialog.connect_response(None, move |d, response| {
                                    if response == "delete" {
                                        if commit_connections(&state_del, &toast_del, |connections| connections.retain(|c| c.id != del_id)).is_none() {
                                            d.close();
                                            return;
                                        }
                                        let del_id_clone = del_id.clone();
                                        let toast = toast_del.clone();
                                        glib::MainContext::default().spawn_local(async move {
                                            if let Err(error) = secrets::delete_password(&del_id_clone).await {
                                                toast.add_toast(adw::Toast::new(&format!("Connection deleted, but its stored password could not be removed: {:#}", error)));
                                            }
                                        });
                                        list_box_del.remove(&sel_row_del);
                                        list_box_del.unselect_all();
                                        window_title_del.set_subtitle(&format!(
                                            "{} connections",
                                            state_del.borrow().connections.len()
                                        ));
                                        toast_del.add_toast(adw::Toast::new("Connection deleted"));
                                    }
                                    d.close();
                                });

                                dialog.set_transient_for(Some(&window_key));
                                dialog.present();
                                return glib::Propagation::Stop;
                            }
                        }
                    }
                    gdk::Key::Return if sidebar_has_focus => {
                        if let Some(sel_row) = list_box_key.selected_row() {
                            let id = sel_row.widget_name().to_string();
                            let conn_opt = state_key
                                .borrow()
                                .connections
                                .iter()
                                .find(|c| c.id == id)
                                .cloned();
                            if let Some(conn) = conn_opt {
                                launch_with_stored_password(conn, launch_session_key.clone(), &toast_overlay_key);
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

        // Pin toggling, used by the row star buttons and Ctrl+P
        let toggle_pin_action = gio::SimpleAction::new("toggle-pin", Some(glib::VariantTy::STRING));
        let state_for_pin = state.clone();
        let list_box_for_pin = list_box.clone();
        let toast_overlay_for_pin = toast_overlay.clone();
        toggle_pin_action.connect_activate(move |_, param| {
            let Some(id) = param.and_then(|p| p.get::<String>()) else {
                return;
            };
            let updated =
                commit_connections(&state_for_pin, &toast_overlay_for_pin, |connections| {
                    connections.iter_mut().find(|c| c.id == id).map(|c| {
                        c.pinned = !c.pinned;
                        c.clone()
                    })
                })
                .flatten();
            if let Some(conn) = updated {
                if let Some(row) = find_row(&list_box_for_pin, &conn.id) {
                    update_connection_row(&row, &conn);
                }
                list_box_for_pin.invalidate_sort();
                list_box_for_pin.invalidate_headers();
            }
        });
        window.add_action(&toggle_pin_action);

        // Launch by ID, used by the Recent menu and the tray menu
        let launch_action =
            gio::SimpleAction::new("launch-connection", Some(glib::VariantTy::STRING));
        let state_for_launch_action = state.clone();
        let list_box_for_launch_action = list_box.clone();
        let toast_overlay_for_launch_action = toast_overlay.clone();
        let launch_session_for_action = launch_session.clone();
        launch_action.connect_activate(move |_, param| {
            let Some(id) = param.and_then(|p| p.get::<String>()) else {
                return;
            };
            let conn_opt = state_for_launch_action
                .borrow()
                .connections
                .iter()
                .find(|c| c.id == id)
                .cloned();
            let Some(conn) = conn_opt else {
                toast_overlay_for_launch_action
                    .add_toast(adw::Toast::new("That connection no longer exists"));
                return;
            };
            if let Some(row) = find_row(&list_box_for_launch_action, &conn.id) {
                list_box_for_launch_action.select_row(Some(&row));
            }
            launch_with_stored_password(
                conn,
                launch_session_for_action.clone(),
                &toast_overlay_for_launch_action,
            );
        });
        window.add_action(&launch_action);

        // Placeholder shown as insensitive when there is no launch history
        let no_recent_action = gio::SimpleAction::new("no-recent", None);
        no_recent_action.set_enabled(false);
        window.add_action(&no_recent_action);

        // Rebuild the Recent menu each time it opens
        let state_for_recent = state.clone();
        recent_btn.set_create_popup_func(move |_| {
            recent_menu.remove_all();
            let st = state_for_recent.borrow();
            let recent = recent_connections(&st.config, &st.connections);
            if recent.is_empty() {
                recent_menu.append(Some("No Recent Connections"), Some("win.no-recent"));
            }
            for conn in recent {
                let item = gio::MenuItem::new(Some(&tray::menu_label(&conn.name)), None);
                item.set_action_and_target_value(
                    Some("win.launch-connection"),
                    Some(&conn.id.to_variant()),
                );
                recent_menu.append_item(&item);
            }
        });

        {
            let st = state.borrow();
            tray::update_launch_menu(&st.config, &st.connections);
        }

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
            if let Some(row) = find_row(&list_box, &conn.id) {
                list_box.select_row(Some(&row));
            }

            launch_with_stored_password(conn, launch_session.clone(), &toast_overlay);
        }

        window
    }
}
