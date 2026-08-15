use std::cell::RefCell;
use std::fs;
use std::rc::Rc;

use gtk::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;

use crate::importers::{export_connections_json, export_rdp_file};
use crate::models::{Connection, Protocol};

#[derive(Clone)]
struct ExportCandidate {
    pub connection: Connection,
    pub selected: Rc<RefCell<bool>>,
}

pub struct ExportDialog;

impl ExportDialog {
    pub fn show(
        parent: &impl IsA<gtk::Window>,
        connections: &[Connection],
        selected_connection: Option<&Connection>,
    ) {
        let window = adw::Window::builder()
            .transient_for(parent)
            .modal(true)
            .title("Export Connections")
            .default_width(560)
            .default_height(580)
            .build();

        let header_bar = adw::HeaderBar::new();
        let title = adw::WindowTitle::new(
            "Export Connections",
            "Choose format and select connections to export",
        );
        header_bar.set_title_widget(Some(&title));

        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&header_bar);

        let content_box = gtk::Box::new(gtk::Orientation::Vertical, 12);
        content_box.set_margin_top(16);
        content_box.set_margin_bottom(16);
        content_box.set_margin_start(16);
        content_box.set_margin_end(16);

        // Format Selection Group
        let format_group = adw::PreferencesGroup::builder()
            .title("Export Format")
            .build();

        let format_types = gtk::StringList::new(&[
            "VER JSON Backup (.json - multi-connection)",
            "Microsoft RDP File (.rdp - single profile)",
        ]);

        let format_combo = adw::ComboRow::builder()
            .title("Format")
            .model(&format_types)
            .build();
        format_group.add(&format_combo);

        // Selection List Group
        let list_group = adw::PreferencesGroup::builder()
            .title("Select Connections")
            .description("Check the connections you want to include in the export")
            .build();

        let select_all_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let select_all_btn = gtk::Button::builder()
            .label("Select All")
            .css_classes(vec!["flat"])
            .build();
        let deselect_all_btn = gtk::Button::builder()
            .label("Deselect All")
            .css_classes(vec!["flat"])
            .build();
        select_all_box.append(&select_all_btn);
        select_all_box.append(&deselect_all_btn);

        let list_box = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .css_classes(vec!["boxed-list"])
            .build();

        list_group.add(&select_all_box);
        list_group.add(&list_box);

        // Bottom Action Buttons
        let bottom_box = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        bottom_box.set_halign(gtk::Align::End);
        bottom_box.set_margin_top(8);

        let cancel_btn = gtk::Button::builder().label("Cancel").build();
        let export_btn = gtk::Button::builder()
            .label("Export Selected (0)...")
            .css_classes(vec!["suggested-action", "pill"])
            .sensitive(false)
            .build();

        bottom_box.append(&cancel_btn);
        bottom_box.append(&export_btn);

        content_box.append(&format_group);
        content_box.append(&list_group);
        content_box.append(&bottom_box);

        let clamp = adw::Clamp::builder()
            .maximum_size(580)
            .child(&content_box)
            .build();

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .child(&clamp)
            .build();

        toolbar_view.set_content(Some(&scrolled));
        window.set_content(Some(&toolbar_view));

        // Build Candidates: If selected_connection is present, pre-check only that connection; otherwise pre-check all
        let sel_id = selected_connection.map(|c| c.id.as_str());
        let candidates: Rc<RefCell<Vec<ExportCandidate>>> = Rc::new(RefCell::new(
            connections
                .iter()
                .map(|conn| {
                    let initial_checked = match sel_id {
                        Some(id) => conn.id == id,
                        None => true,
                    };
                    ExportCandidate {
                        connection: conn.clone(),
                        selected: Rc::new(RefCell::new(initial_checked)),
                    }
                })
                .collect(),
        ));

        // Refresh List UI
        let refresh_list = {
            let candidates = candidates.clone();
            let list_box = list_box.clone();
            let export_btn = export_btn.clone();
            let format_combo = format_combo.clone();

            move || {
                while let Some(child) = list_box.first_child() {
                    list_box.remove(&child);
                }

                let items = candidates.borrow();
                let is_rdp_format = format_combo.selected() == 1;
                let mut checked_count = 0;

                for cand in items.iter() {
                    if is_rdp_format && cand.connection.protocol != Protocol::Rdp {
                        continue;
                    }

                    let is_checked = *cand.selected.borrow();
                    if is_checked {
                        checked_count += 1;
                    }

                    let subtitle = if cand.connection.username.is_empty() {
                        format!("{}:{} [{}]", cand.connection.host, cand.connection.port, cand.connection.group)
                    } else {
                        format!("{}@{}:{} [{}]", cand.connection.username, cand.connection.host, cand.connection.port, cand.connection.group)
                    };

                    let row = adw::ActionRow::builder()
                        .title(&cand.connection.name)
                        .subtitle(subtitle)
                        .build();

                    let check = gtk::CheckButton::builder()
                        .active(is_checked)
                        .valign(gtk::Align::Center)
                        .build();

                    let selected_rc = cand.selected.clone();
                    let export_btn_clone = export_btn.clone();
                    let candidates_clone = candidates.clone();
                    let format_combo_clone = format_combo.clone();

                    check.connect_toggled(move |btn| {
                        *selected_rc.borrow_mut() = btn.is_active();
                        let is_rdp = format_combo_clone.selected() == 1;
                        let count = candidates_clone
                            .borrow()
                            .iter()
                            .filter(|c| (!is_rdp || c.connection.protocol == Protocol::Rdp) && *c.selected.borrow())
                            .count();
                        export_btn_clone.set_label(&format!("Export Selected ({})...", count));
                        export_btn_clone.set_sensitive(count > 0);
                    });

                    row.add_prefix(&check);
                    list_box.append(&row);
                }

                export_btn.set_label(&format!("Export Selected ({})...", checked_count));
                export_btn.set_sensitive(checked_count > 0);
            }
        };

        refresh_list();

        // React to Format changes
        {
            let refresh_for_format = refresh_list.clone();
            format_combo.connect_selected_notify(move |_| {
                refresh_for_format();
            });
        }

        // Select All / Deselect All Handlers
        let candidates_sa = candidates.clone();
        let refresh_sa = refresh_list.clone();
        let format_combo_sa = format_combo.clone();
        select_all_btn.connect_clicked(move |_| {
            let is_rdp = format_combo_sa.selected() == 1;
            for c in candidates_sa.borrow().iter() {
                if !is_rdp || c.connection.protocol == Protocol::Rdp {
                    *c.selected.borrow_mut() = true;
                }
            }
            refresh_sa();
        });

        let candidates_da = candidates.clone();
        let refresh_da = refresh_list.clone();
        deselect_all_btn.connect_clicked(move |_| {
            for c in candidates_da.borrow().iter() {
                *c.selected.borrow_mut() = false;
            }
            refresh_da();
        });

        // Cancel
        let win_cancel = window.downgrade();
        cancel_btn.connect_clicked(move |_| {
            if let Some(win) = win_cancel.upgrade() {
                win.close();
            }
        });

        // Export Button Handler
        let candidates_export = candidates.clone();
        let win_export = window.clone();
        let format_combo_export = format_combo.clone();

        export_btn.connect_clicked(move |_| {
            let is_rdp_format = format_combo_export.selected() == 1;
            let target_conns: Vec<Connection> = candidates_export
                .borrow()
                .iter()
                .filter(|c| (!is_rdp_format || c.connection.protocol == Protocol::Rdp) && *c.selected.borrow())
                .map(|c| c.connection.clone())
                .collect();

            if target_conns.is_empty() {
                return;
            }

            let (suggested_name, filter_name, filter_pattern) = if is_rdp_format {
                let name = target_conns
                    .first()
                    .map(|c| format!("{}.rdp", c.name))
                    .unwrap_or_else(|| "connection.rdp".to_string());
                (name, "RDP Configuration Files (*.rdp)", "*.rdp")
            } else {
                ("ver_backup.json".to_string(), "JSON Files (*.json)", "*.json")
            };

            let save_dialog = gtk::FileChooserNative::new(
                Some("Save Exported Connections"),
                Some(&win_export),
                gtk::FileChooserAction::Save,
                Some("Save"),
                Some("Cancel"),
            );

            save_dialog.set_current_name(&suggested_name);
            let filter = gtk::FileFilter::new();
            filter.set_name(Some(filter_name));
            filter.add_pattern(filter_pattern);
            save_dialog.add_filter(&filter);

            let conns_for_save = target_conns.clone();
            let win_to_close = win_export.clone();

            save_dialog.connect_response(move |d, response| {
                if response == gtk::ResponseType::Accept {
                    if let Some(file) = d.file() {
                        if let Some(path) = file.path() {
                            let result = if is_rdp_format {
                                if let Some(conn) = conns_for_save.first() {
                                    let rdp_content = export_rdp_file(conn);
                                    fs::write(&path, rdp_content).map_err(|e| e.to_string())
                                } else {
                                    Err("No connection selected to export as RDP".to_string())
                                }
                            } else {
                                match export_connections_json(&conns_for_save) {
                                    Ok(json) => fs::write(&path, json).map_err(|e| e.to_string()),
                                    Err(e) => Err(e.to_string()),
                                }
                            };

                            if let Err(e) = result {
                                eprintln!("Failed to export: {}", e);
                            } else {
                                win_to_close.close();
                            }
                        }
                    }
                }
            });

            save_dialog.show();
        });

        window.present();
    }
}
