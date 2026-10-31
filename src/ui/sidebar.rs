use gtk::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use super::window::AppWindowState;
use crate::models::{Connection, Protocol};
use crate::prober::HostStatus;

pub const STATUS_DOT_CSS: &str = "
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
row:not(:hover):not(:selected) .pin-button.unpinned {
    opacity: 0;
}
";

/// Header shown above pinned connections, which are listed before all groups.
pub const PINNED_SECTION: &str = "Pinned";

/// Sidebar section of a connection: pinned connections share one section ahead of the groups.
fn section_of(conn: &Connection) -> (bool, &str) {
    if conn.pinned {
        (true, PINNED_SECTION)
    } else {
        (false, conn.group.as_str())
    }
}

/// Sidebar order: pinned connections first, then groups, then names, ignoring case.
pub fn compare_connections(a: &Connection, b: &Connection) -> std::cmp::Ordering {
    b.pinned
        .cmp(&a.pinned)
        .then_with(|| {
            if a.pinned {
                std::cmp::Ordering::Equal
            } else {
                a.group.to_lowercase().cmp(&b.group.to_lowercase())
            }
        })
        .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
}

fn apply_pin_state(button: &gtk::Button, pinned: bool) {
    if pinned {
        button.set_icon_name("starred-symbolic");
        button.set_tooltip_text(Some("Unpin (Ctrl+P)"));
        button.remove_css_class("unpinned");
    } else {
        button.set_icon_name("non-starred-symbolic");
        button.set_tooltip_text(Some("Pin to Top (Ctrl+P)"));
        button.add_css_class("unpinned");
    }
}

pub fn protocol_icon_name(proto: Protocol) -> &'static str {
    match proto {
        Protocol::Rdp | Protocol::Xrdp => "video-display-symbolic",
        Protocol::Vnc => "computer-symbolic",
        Protocol::Ssh => "utilities-terminal-symbolic",
        Protocol::Spice => "video-display-symbolic",
    }
}

pub fn update_connection_row(row: &gtk::ListBoxRow, conn: &Connection) {
    if let Some(child) = row.child() {
        if let Ok(action_row) = child.downcast::<adw::ActionRow>() {
            action_row.set_title(&conn.name);
            let subtitle = if conn.username.is_empty() {
                conn.host.clone()
            } else {
                format!("{}@{}", conn.username, conn.host)
            };
            action_row.set_subtitle(&subtitle);

            let action_row = action_row.upcast::<gtk::Widget>();
            if let Some(icon) = find_named_child(&action_row, "protocol_icon") {
                if let Ok(img) = icon.downcast::<gtk::Image>() {
                    img.set_icon_name(Some(protocol_icon_name(conn.protocol)));
                }
            }
            if let Some(button) = find_named_child(&action_row, "pin_button") {
                if let Ok(button) = button.downcast::<gtk::Button>() {
                    apply_pin_state(&button, conn.pinned);
                }
            }
        }
    }
}

fn find_named_child(widget: &gtk::Widget, name: &str) -> Option<gtk::Widget> {
    if widget.widget_name() == name {
        return Some(widget.clone());
    }
    let mut child = widget.first_child();
    while let Some(c) = child {
        if let Some(found) = find_named_child(&c, name) {
            return Some(found);
        }
        child = c.next_sibling();
    }
    None
}

pub fn create_connection_row(
    conn: &Connection,
    state: Rc<RefCell<AppWindowState>>,
) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.set_widget_name(&conn.id);

    let icon_name = protocol_icon_name(conn.protocol);

    let subtitle = if conn.username.is_empty() {
        conn.host.clone()
    } else {
        format!("{}@{}", conn.username, conn.host)
    };

    let action_row = adw::ActionRow::builder()
        .title(&conn.name)
        .subtitle(subtitle)
        .use_markup(false)
        .build();

    // Prefix container with Status Dot and Protocol Icon
    let prefix_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    prefix_box.set_valign(gtk::Align::Center);

    let status_dot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    status_dot.set_valign(gtk::Align::Center);
    status_dot.set_widget_name("status_dot");

    let current_status = state
        .borrow()
        .host_statuses
        .get(&conn.id)
        .cloned()
        .unwrap_or(HostStatus::Unknown);

    status_dot.set_css_classes(&["status-dot", current_status.status_css_class()]);
    status_dot.set_tooltip_text(Some(&current_status.description()));

    let icon = gtk::Image::from_icon_name(icon_name);
    icon.set_widget_name("protocol_icon");

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

    // Toggles through the window's `win.toggle-pin` action, which saves the change.
    let pin_button = gtk::Button::builder()
        .css_classes(vec!["flat", "circular", "pin-button"])
        .valign(gtk::Align::Center)
        .action_name("win.toggle-pin")
        .action_target(&conn.id.to_variant())
        .build();
    pin_button.set_widget_name("pin_button");
    apply_pin_state(&pin_button, conn.pinned);
    action_row.add_suffix(&pin_button);

    row.set_child(Some(&action_row));
    row
}

pub fn setup_group_headers(list_box: &gtk::ListBox, state: Rc<RefCell<AppWindowState>>) {
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

        let current_section = conn.map(section_of).unwrap_or((false, "Default"));
        let prev_section = before_conn.as_ref().map(section_of);

        if prev_section != Some(current_section) {
            let label = gtk::Label::builder()
                .label(current_section.1)
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

pub fn setup_sorting(list_box: &gtk::ListBox, state: Rc<RefCell<AppWindowState>>) {
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
            (Some(a), Some(b)) => compare_connections(a, b).into(),
            _ => std::cmp::Ordering::Equal.into(),
        }
    });
}

pub fn setup_filtering(
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

/// The sidebar row for a connection ID, if one exists.
pub fn find_row(list_box: &gtk::ListBox, conn_id: &str) -> Option<gtk::ListBoxRow> {
    let mut child_opt = list_box.first_child();
    while let Some(child) = child_opt {
        if let Ok(row) = child.clone().downcast::<gtk::ListBoxRow>() {
            if row.widget_name() == conn_id {
                return Some(row);
            }
        }
        child_opt = child.next_sibling();
    }
    None
}

pub fn find_and_set_badge(widget: &gtk::Widget, is_active: bool) -> bool {
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

pub fn set_row_active_badge(list_box: &gtk::ListBox, conn_id: &str, is_active: bool) {
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

pub fn find_and_set_status_dot(widget: &gtk::Widget, status: &HostStatus) -> bool {
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

pub fn set_row_status_dot(list_box: &gtk::ListBox, conn_id: &str, status: &HostStatus) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn conn(name: &str, group: &str, pinned: bool) -> Connection {
        Connection {
            name: name.into(),
            group: group.into(),
            pinned,
            ..Default::default()
        }
    }

    #[test]
    fn pinned_connections_sort_first_by_name_across_groups() {
        let mut list = [
            conn("b", "Alpha", false),
            conn("z", "Zulu", true),
            conn("a", "Beta", false),
            conn("C", "Alpha", true),
        ];
        list.sort_by(compare_connections);
        let names: Vec<&str> = list.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["C", "z", "b", "a"]);
    }

    #[test]
    fn pinned_section_differs_from_a_group_named_pinned() {
        assert_ne!(
            section_of(&conn("a", PINNED_SECTION, true)),
            section_of(&conn("b", PINNED_SECTION, false))
        );
    }
}
