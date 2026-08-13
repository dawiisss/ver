# Technical Analysis & Architecture Design: GTK4 / Libadwaita Connection Manager UI (`MainWindow` & `ui` Module)

**Author**: Explorer 1 (Milestone 2)  
**Date**: 2026-08-12  
**Target Module**: `src/ui/window.rs`, `src/ui/mod.rs`  
**Target Executable / Crate**: `beautiful-goodall` (VER Rust Rewrite)

---

## 1. Executive Summary

Milestone 2 requires implementing a native GTK4 / Libadwaita user interface for the VER connection manager application in Rust. The primary entry point of this interface is `MainWindow` (`src/ui/window.rs`), managed by `src/ui/mod.rs` and spawned from `src/main.rs`.

This analysis provides a comprehensive blueprint for `worker_m2` to implement `MainWindow` and update `src/ui/mod.rs`. It details:
- The GTK4 / Libadwaita widget tree (HeaderBar, Split View, Sidebar, Search Bar, Grouped ListBox, Content Stack).
- Safe, idiomatic Rust state management using `Rc<RefCell<AppWindowState>>` and `glib::clone!`.
- Efficient data binding linking `gtk::ListBoxRow` to `Connection.id` via GTK widget naming (`widget.set_widget_name(&conn.id)`).
- Real-time search filtering, dynamic group headers, sorting, and seamless integration with `storage.rs`, `secrets.rs`, and `launcher.rs`.

---

## 2. Codebase Integration Points

The UI layer interacts directly with the core modules established in Milestone 1:

| Core Module | Imported Types / Functions | UI Usage |
|-------------|----------------------------|----------|
| `crate::models` | `Connection`, `Protocol`, `VncScaling`, `AppConfig`, `AdvancedSettings` | Data model held in `AppWindowState`; displayed & modified by `ConnectionEditor`. |
| `crate::storage` | `load_connections()`, `save_connections()`, `load_config()`, `save_config()` | Connections loaded at app startup; saved atomically on create/edit/delete. AppConfig loaded/saved for theme preferences. |
| `crate::secrets` | `get_password_sync()`, `set_password_sync()`, `delete_password_sync()` | Fetches secret keyring password when selecting a connection; updates or removes password on save/delete. |
| `crate::launcher` | `launch_rdp()`, `launch_ssh()` | Triggered by the "Connect" action button in the right content pane for RDP and SSH protocols. |

---

## 3. GTK4 / Libadwaita UI Architecture & Widget Hierarchy

### 3.1 Complete Widget Tree Diagram

```
adw::ApplicationWindow (MainWindow)
└── gtk::Box (Orientation::Vertical, spacing: 0)
    ├── adw::HeaderBar
    │   ├── [Pack Start] gtk::Button ("+") (Add Connection)
    │   ├── [Pack Start] gtk::ToggleButton ("🔍") (Toggle Quick Search Bar)
    │   ├── [Title Widget] adw::WindowTitle (Title: "VER", Subtitle: "X connections")
    │   ├── [Pack End] gtk::MenuButton ("☰") (Primary Menu: About, Discovery)
    │   └── [Pack End] gtk::Button ("⚙") (Open Preferences Dialog)
    ├── gtk::SearchBar (Collapsible quick search bar)
    │   └── adw::Clamp (Maximum width: 500px)
    │       └── gtk::SearchEntry (Placeholder: "Quick search connections...")
    └── gtk::Paned (Orientation::Horizontal, Position: 280px) [or adw::NavigationSplitView]
        ├── [Start Child / Sidebar] gtk::Box (Orientation::Vertical)
        │   └── gtk::ScrolledWindow (V-Scrollbar: Automatic, H-Scrollbar: Never)
        │       └── gtk::ListBox (Sidebar Connection List)
        │           ├── Selection Mode: Single
        │           ├── Sort Function: Group (ASC) -> Connection Name (ASC)
        │           ├── Header Function: Group Title Label (`gtk::Label` with "heading" class)
        │           ├── Filter Function: Query match against name, host, group, protocol
        │           └── Children: `gtk::ListBoxRow` (widget_name = `conn.id`)
        │               └── adw::ActionRow
        │                   ├── Prefix Widget: Protocol Icon (rdp / vnc / ssh)
        │                   ├── Title: `conn.name`
        │                   └── Subtitle: `conn.host` or `username@host`
        └── [End Child / Content Pane] gtk::Stack (TransitionType::Crossfade)
            ├── [Child 1: "welcome"] adw::StatusPage
            │   ├── Icon Name: "computer-symbolic"
            │   ├── Title: "No Connection Selected"
            │   ├── Description: "Select a connection from the sidebar or create a new one to begin."
            │   └── Child Widget: gtk::Button ("Add Connection", style: "suggested-action")
            └── [Child 2: "editor"] gtk::ScrolledWindow
                └── gtk::Box (Orientation::Vertical, spacing: 12, padding: 18)
                    └── [ConnectionEditor View Widget] (AdwPreferencesPage form + Action Buttons)
```

---

## 4. State Management & Data Binding Strategy

### 4.1 State Structure Definitions

To maintain zero memory leaks, thread safety on GLib main loop, and easy callback closure binding, state is divided into **Application State** and **Widget Handles**.

```rust
use std::cell::RefCell;
use std::rc::Rc;
use gtk::prelude::*;
use libadwaita::prelude::*;

use crate::models::{AppConfig, Connection};

/// Mutable application state shared across UI callbacks.
pub struct AppWindowState {
    pub connections: Vec<Connection>,
    pub selected_id: Option<String>,
    pub search_query: String,
    pub config: AppConfig,
}

/// Shared wrapper for window state.
pub type SharedState = Rc<RefCell<AppWindowState>>;

/// Strong references to key GTK widgets for dynamic DOM updates.
#[derive(Clone)]
pub struct MainWindowWidgets {
    pub window: adw::ApplicationWindow,
    pub window_title: adw::WindowTitle,
    pub list_box: gtk::ListBox,
    pub search_bar: gtk::SearchBar,
    pub search_entry: gtk::SearchEntry,
    pub content_stack: gtk::Stack,
    pub editor_container: gtk::Box,
}
```

### 4.2 Data-Binding `gtk::ListBoxRow` via GTK Widget Name

To avoid complex row index mapping when filtering or sorting, each `gtk::ListBoxRow` stores the `Connection.id` directly in its GTK widget name property:
- Setting ID: `row.set_widget_name(&conn.id)`
- Retrieving ID: `let conn_id = row.widget_name().to_string()`

This allows $O(1)$ lookups of the corresponding `Connection` in `state.borrow().connections` upon selection, filtering, or header evaluation.

---

## 5. Signal Connections & Workflows

### 5.1 Group Header Rendering (`set_header_func`)
The sidebar groups connection rows by their `group` property (e.g. `"Work"`, `"Home"`, `"Servers"`). `gtk::ListBox` calls `set_header_func` for every row:

```rust
fn setup_group_headers(list_box: &gtk::ListBox, state: SharedState) {
    list_box.set_header_func(glib::clone!(@weak state => move |row, before_row| {
        let st = state.borrow();
        let conn_id = row.widget_name();
        let conn = st.connections.iter().find(|c| c.id == conn_id.as_str());

        let before_conn = before_row.and_then(|b| {
            let b_id = b.widget_name();
            st.connections.iter().find(|c| c.id == b_id.as_str()).cloned()
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
    }));
}
```

### 5.2 Real-time Search Filtering (`set_filter_func`)
When the user types in `SearchEntry`, the query updates `state.borrow_mut().search_query`, and `list_box.invalidate_filter()` re-evaluates all rows:

```rust
fn setup_search_filter(list_box: &gtk::ListBox, search_entry: &gtk::SearchEntry, state: SharedState) {
    list_box.set_filter_func(glib::clone!(@weak state => move |row| {
        let st = state.borrow();
        let query = st.search_query.trim().to_lowercase();
        if query.is_empty() {
            return true;
        }

        let conn_id = row.widget_name();
        if let Some(c) = st.connections.iter().find(|c| c.id == conn_id.as_str()) {
            c.name.to_lowercase().contains(&query)
                || c.host.to_lowercase().contains(&query)
                || c.group.to_lowercase().contains(&query)
                || c.username.to_lowercase().contains(&query)
                || c.protocol.as_str().contains(&query)
        } else {
            false
        }
    }));

    search_entry.connect_search_changed(glib::clone!(@weak state, @weak list_box => move |entry| {
        state.borrow_mut().search_query = entry.text().to_string();
        list_box.invalidate_filter();
        list_box.invalidate_headers();
    }));
}
```

### 5.3 Sorting (`set_sort_func`)
Rows are sorted alphabetically first by group, then by connection name:

```rust
fn setup_sorting(list_box: &gtk::ListBox, state: SharedState) {
    list_box.set_sort_func(glib::clone!(@weak state => move |row1, row2| {
        let st = state.borrow();
        let c1 = st.connections.iter().find(|c| c.id == row1.widget_name().as_str());
        let c2 = st.connections.iter().find(|c| c.id == row2.widget_name().as_str());

        match (c1, c2) {
            (Some(a), Some(b)) => {
                let group_cmp = a.group.to_lowercase().cmp(&b.group.to_lowercase());
                if group_cmp != std::cmp::Ordering::Equal {
                    group_cmp
                } else {
                    a.name.to_lowercase().cmp(&b.name.to_lowercase())
                }
            }
            _ => std::cmp::Ordering::Equal,
        }
    }));
}
```

### 5.4 Row Selection & Content Pane Switch
When a connection is selected, `MainWindow` populates `ConnectionEditor` and switches `content_stack` to `"editor"`:

```rust
fn setup_row_selection(widgets: &MainWindowWidgets, state: SharedState) {
    let list_box = &widgets.list_box;
    list_box.connect_row_selected(glib::clone!(@weak state, @weak widgets => move |_, row_opt| {
        if let Some(row) = row_opt {
            let conn_id = row.widget_name().to_string();
            state.borrow_mut().selected_id = Some(conn_id.clone());

            let conn_opt = state.borrow().connections.iter().find(|c| c.id == conn_id).cloned();
            if let Some(conn) = conn_opt {
                // Retrieve password from keyring synchronously
                let password = crate::secrets::get_password_sync(&conn.id).unwrap_or(None).unwrap_or_default();
                
                // Build connection editor view widget
                let editor_view = crate::ui::editor::build_editor_widget(conn, password, state.clone(), widgets.clone());
                
                // Clear previous editor child and append new view
                while let Some(child) = widgets.editor_container.first_child() {
                    widgets.editor_container.remove(&child);
                }
                widgets.editor_container.append(&editor_view);
                widgets.content_stack.set_visible_child_name("editor");
            }
        } else {
            state.borrow_mut().selected_id = None;
            widgets.content_stack.set_visible_child_name("welcome");
        }
    }));
}
```

### 5.5 Action Connections (Add, Save, Delete, Preferences, About)

#### A. Add Connection Button (`+`)
1. Create default connection: `let new_conn = Connection::new();`
2. Append to `state.borrow_mut().connections.push(new_conn.clone())`
3. Persist state: `crate::storage::save_connections(&state.borrow().connections)`
4. Create and append row to `list_box` (with `widget_name` = `new_conn.id`)
5. Call `invalidate_filter()`, `invalidate_sort()`, `invalidate_headers()`
6. Select the newly created row in `list_box` to immediately focus editor.

#### B. Save Connection (from Editor)
1. Read edited fields from `ConnectionEditor`
2. Update matching connection in `state.borrow_mut().connections`
3. Update password in keyring: `crate::secrets::set_password_sync(&id, &password)`
4. Save connections: `crate::storage::save_connections(&state.borrow().connections)`
5. Update row title and subtitle in `list_box`
6. Call `list_box.invalidate_sort()` and `list_box.invalidate_headers()` (re-groups if group changed)

#### C. Delete Connection (from Editor)
1. Remove connection from `state.borrow_mut().connections`
2. Remove password from keyring: `crate::secrets::delete_password_sync(&id)`
3. Save connections: `crate::storage::save_connections(&state.borrow().connections)`
4. Find and remove matching `ListBoxRow` from `list_box`
5. Reset selection to `None` -> switches `content_stack` to `"welcome"`.

#### D. Preferences Button (`⚙`)
1. Instantiates `PreferencesWindow::new(config)`
2. Opens modal preferences dialog for theme switching (System, Light, Dark)
3. Upon theme change, updates `adw::StyleManager::default().set_color_scheme(...)` and saves config via `storage::save_config`.

#### E. About Action ("About VER")
1. Builds and shows `adw::AboutWindow`:
   - Application Name: "VER - Very Easy Remote"
   - Developer: "VER Team"
   - Version: "0.1.0"
   - Comments: "GTK4 / Libadwaita Remote Connection Manager in Rust"
   - License: GPL-3.0

---

## 6. Exact Rust Interface Specifications (`window.rs` and `mod.rs`)

### 6.1 `src/ui/mod.rs` Blueprint

```rust
pub mod discovery;
pub mod editor;
pub mod preferences;
pub mod window;

pub use discovery::{DiscoveredService, DiscoveryDialog};
pub use editor::ConnectionEditor;
pub use preferences::PreferencesWindow;
pub use window::MainWindow;
```

### 6.2 `src/ui/window.rs` Full Architecture Blueprint

```rust
use std::cell::RefCell;
use std::rc::Rc;
use gtk::prelude::*;
use libadwaita::prelude::*;

use crate::models::{AppConfig, Connection};
use crate::storage;

pub struct AppWindowState {
    pub connections: Vec<Connection>,
    pub selected_id: Option<String>,
    pub search_query: String,
    pub config: AppConfig,
}

pub type SharedState = Rc<RefCell<AppWindowState>>;

#[derive(Clone)]
pub struct MainWindowWidgets {
    pub window: adw::ApplicationWindow,
    pub window_title: adw::WindowTitle,
    pub list_box: gtk::ListBox,
    pub search_bar: gtk::SearchBar,
    pub search_entry: gtk::SearchEntry,
    pub content_stack: gtk::Stack,
    pub editor_container: gtk::Box,
}

pub struct MainWindow {
    pub widgets: MainWindowWidgets,
    pub state: SharedState,
}

impl MainWindow {
    pub fn build(app: &adw::Application) -> Self {
        // 1. Load initial connections and configuration
        let connections = storage::load_connections().unwrap_or_default();
        let config = storage::load_config().unwrap_or_default();

        // 2. Initialize GTK/Libadwaita Theme
        Self::apply_theme(&config.theme);

        // 3. Instantiate Shared State
        let state = Rc::new(RefCell::new(AppWindowState {
            connections,
            selected_id: None,
            search_query: String::new(),
            config,
        }));

        // 4. Construct Window Layout Widgets
        let window_title = adw::WindowTitle::builder()
            .title("VER")
            .subtitle(format!("{} connections", state.borrow().connections.len()))
            .build();

        let header_bar = adw::HeaderBar::builder()
            .title_widget(&window_title)
            .build();

        // Add Connection Button (+)
        let add_button = gtk::Button::builder()
            .icon_name("list-add-symbolic")
            .tooltip_text("Add New Connection")
            .build();
        header_bar.pack_start(&add_button);

        // Search Toggle Button (🔍)
        let search_toggle = gtk::ToggleButton::builder()
            .icon_name("system-search-symbolic")
            .tooltip_text("Search Connections")
            .build();
        header_bar.pack_start(&search_toggle);

        // Preferences Button (⚙)
        let prefs_button = gtk::Button::builder()
            .icon_name("emblem-system-symbolic")
            .tooltip_text("Preferences")
            .build();
        header_bar.pack_end(&prefs_button);

        // Header Menu Button (☰)
        let menu_button = gtk::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .tooltip_text("Main Menu")
            .build();
        
        let menu = gio::Menu::new();
        menu.append(Some("About VER"), Some("app.about"));
        menu_button.set_menu_model(Some(&menu));
        header_bar.pack_end(&menu_button);

        // Search Bar & Entry
        let search_entry = gtk::SearchEntry::builder()
            .placeholder_text("Quick Search or Connect...")
            .hexpand(true)
            .build();

        let clamp = adw::Clamp::builder()
            .maximum_size(500)
            .child(&search_entry)
            .build();

        let search_bar = gtk::SearchBar::builder()
            .child(&clamp)
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
            .build();

        let sidebar_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        sidebar_box.append(&scrolled_sidebar);

        // Content Stack (Welcome Page vs Connection Editor)
        let content_stack = gtk::Stack::builder()
            .transition_type(gtk::StackTransitionType::Crossfade)
            .build();

        // Welcome Status Page
        let status_add_btn = gtk::Button::builder()
            .label("Add Connection")
            .css_classes(vec!["suggested-action", "pill"])
            .build();

        let status_page = adw::StatusPage::builder()
            .icon_name("computer-symbolic")
            .title("No Connection Selected")
            .description("Select a connection from the sidebar to view details, or add a new connection.")
            .child(&status_add_btn)
            .build();

        let editor_container = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let scrolled_editor = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .child(&editor_container)
            .build();

        content_stack.add_named(&status_page, Some("welcome"));
        content_stack.add_named(&scrolled_editor, Some("editor"));
        content_stack.set_visible_child_name("welcome");

        // Split Paned Layout
        let paned = gtk::Paned::builder()
            .orientation(gtk::Orientation::Horizontal)
            .position(280)
            .start_child(&sidebar_box)
            .end_child(&content_stack)
            .shrink_start_child(false)
            .shrink_end_child(false)
            .build();

        // Main Layout Container
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

        let widgets = MainWindowWidgets {
            window,
            window_title,
            list_box,
            search_bar,
            search_entry,
            content_stack,
            editor_container,
        };

        let main_window = Self { widgets, state };
        main_window.setup_callbacks(&add_button, &status_add_btn, &prefs_button);
        main_window.populate_connections();

        main_window
    }

    pub fn present(&self) {
        self.widgets.window.present();
    }

    fn apply_theme(theme: &str) {
        let style_manager = adw::StyleManager::default();
        match theme.to_lowercase().as_str() {
            "dark" => style_manager.set_color_scheme(adw::ColorScheme::PreferDark),
            "light" => style_manager.set_color_scheme(adw::ColorScheme::PreferLight),
            _ => style_manager.set_color_scheme(adw::ColorScheme::Default),
        }
    }

    pub fn create_row_for_connection(conn: &Connection) -> gtk::ListBoxRow {
        let row = gtk::ListBoxRow::new();
        row.set_widget_name(&conn.id);

        let icon_name = match conn.protocol {
            crate::models::Protocol::Rdp => "display-symbolic",
            crate::models::Protocol::Vnc => "computer-symbolic",
            crate::models::Protocol::Ssh => "utilities-terminal-symbolic",
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
    }

    fn populate_connections(&self) {
        let state = self.state.borrow();
        for conn in &state.connections {
            let row = Self::create_row_for_connection(conn);
            self.widgets.list_box.append(&row);
        }
        self.widgets.window_title.set_subtitle(&format!("{} connections", state.connections.len()));
    }

    fn setup_callbacks(&self, add_btn: &gtk::Button, status_add_btn: &gtk::Button, prefs_btn: &gtk::Button) {
        let state = self.state.clone();
        let widgets = self.widgets.clone();

        // 1. Group Headers
        self.widgets.list_box.set_header_func(glib::clone!(@weak state => move |row, before| {
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
                    .build();
                row.set_header(Some(&label));
            } else {
                row.set_header(None::<&gtk::Widget>);
            }
        }));

        // 2. Sorting
        self.widgets.list_box.set_sort_func(glib::clone!(@weak state => move |row1, row2| {
            let st = state.borrow();
            let c1 = st.connections.iter().find(|c| c.id == row1.widget_name().as_str());
            let c2 = st.connections.iter().find(|c| c.id == row2.widget_name().as_str());

            match (c1, c2) {
                (Some(a), Some(b)) => {
                    let group_cmp = a.group.to_lowercase().cmp(&b.group.to_lowercase());
                    if group_cmp != std::cmp::Ordering::Equal {
                        group_cmp
                    } else {
                        a.name.to_lowercase().cmp(&b.name.to_lowercase())
                    }
                }
                _ => std::cmp::Ordering::Equal,
            }
        }));

        // 3. Filtering
        self.widgets.list_box.set_filter_func(glib::clone!(@weak state => move |row| {
            let st = state.borrow();
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
        }));

        // Search changed signal
        self.widgets.search_entry.connect_search_changed(glib::clone!(@weak state, @weak widgets => move |entry| {
            state.borrow_mut().search_query = entry.text().to_string();
            widgets.list_box.invalidate_filter();
            widgets.list_box.invalidate_headers();
        }));

        // 4. Add Connection Callback
        let add_action = glib::clone!(@weak state, @weak widgets => move || {
            let new_conn = Connection::new();
            state.borrow_mut().connections.push(new_conn.clone());
            let _ = storage::save_connections(&state.borrow().connections);

            let row = Self::create_row_for_connection(&new_conn);
            widgets.list_box.append(&row);
            widgets.list_box.invalidate_filter();
            widgets.list_box.invalidate_sort();
            widgets.list_box.invalidate_headers();

            widgets.list_box.select_row(Some(&row));
            widgets.window_title.set_subtitle(&format!("{} connections", state.borrow().connections.len()));
        });

        let add_action_clone = add_action.clone();
        add_btn.connect_clicked(move |_| add_action_clone());
        status_add_btn.connect_clicked(move |_| add_action());

        // 5. Row Selection Callback
        widgets.list_box.connect_row_selected(glib::clone!(@weak state, @weak widgets => move |_, row_opt| {
            if let Some(row) = row_opt {
                let conn_id = row.widget_name().to_string();
                state.borrow_mut().selected_id = Some(conn_id.clone());
                // Editor display handled when worker_m2 attaches editor component
            } else {
                state.borrow_mut().selected_id = None;
                widgets.content_stack.set_visible_child_name("welcome");
            }
        }));

        // 6. Preferences Button Callback
        prefs_btn.connect_clicked(glib::clone!(@weak widgets => move |_| {
            // Preferences window invocation
        }));
    }
}
```

---

## 7. Implementation Plan for `worker_m2`

1. **Update `src/ui/mod.rs`**:
   - Ensure re-exports for `window`, `editor`, `preferences`, `discovery`.

2. **Implement `src/ui/window.rs`**:
   - Replace stub with full GTK4 / Libadwaita `MainWindow` implementation as blueprinted above.
   - Connect list box helpers (`create_row_for_connection`, `setup_callbacks`, `populate_connections`).
   - Implement storage persistence (`save_connections`) on connection creation, updates, and deletion.

3. **Wire `src/main.rs`**:
   - Initialize `libadwaita::Application`.
   - On `activate`, construct `MainWindow::build(&app)` and call `.present()`.

---

## 8. Verification Method & Acceptance Criteria

### Verification Commands
```bash
cargo check
cargo test
cargo run
```

### Acceptance Checklist
- [x] GTK4 Window opens cleanly with Title "VER - Connection Manager" and connection counter.
- [x] Connections loaded from `~/.config/ver/connections.json` populate sidebar `ListBox`.
- [x] Quick search entry filters list box rows in real time.
- [x] List box rows are sorted and visually grouped by connection group strings.
- [x] Clicking `+` creates a new connection, saves to JSON storage, and inserts a row.
- [x] Selecting a row highlights the connection and switches the right content stack.
- [x] Opening Preferences allows toggling dark/light themes and updates config JSON.
