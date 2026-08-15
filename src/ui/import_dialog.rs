use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;

use crate::importers::{
    import_connections_json, import_rdp_file, import_remmina_file, import_ssh_config_file,
    scan_remmina_profiles, ImportConflictStrategy,
};
use crate::models::Connection;

#[derive(Clone)]
struct ImportCandidate {
    pub connection: Connection,
    pub source_desc: String,
    pub selected: Rc<RefCell<bool>>,
}

pub struct ImportDialog;

impl ImportDialog {
    pub fn show<FImport>(parent: &impl IsA<gtk::Window>, on_import: FImport)
    where
        FImport: Fn(Vec<Connection>, ImportConflictStrategy) + 'static,
    {
        let window = adw::Window::builder()
            .transient_for(parent)
            .modal(true)
            .title("Import Connections")
            .default_width(580)
            .default_height(600)
            .build();

        let header_bar = adw::HeaderBar::new();
        let title = adw::WindowTitle::new("Import Connections", "Migrate from Remmina, SSH, RDP, or JSON");
        header_bar.set_title_widget(Some(&title));

        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&header_bar);

        let content_box = gtk::Box::new(gtk::Orientation::Vertical, 12);
        content_box.set_margin_top(16);
        content_box.set_margin_bottom(16);
        content_box.set_margin_start(16);
        content_box.set_margin_end(16);

        // Source Selection Group
        let source_group = adw::PreferencesGroup::builder()
            .title("Import Source")
            .build();

        let source_types = gtk::StringList::new(&[
            "Remmina Profiles (~/.local/share/remmina)",
            "OpenSSH Config (~/.ssh/config)",
            "Microsoft RDP File (.rdp)",
            "VER JSON Backup (.json)",
        ]);

        let source_combo = adw::ComboRow::builder()
            .title("Format")
            .model(&source_types)
            .build();
        source_group.add(&source_combo);

        // Action button row for scanning / file choosing
        let action_row = adw::ActionRow::builder()
            .title("Source Location")
            .subtitle("Select a file or auto-scan default paths")
            .build();

        let scan_btn = gtk::Button::builder()
            .label("Auto-Scan Defaults")
            .css_classes(vec!["pill"])
            .valign(gtk::Align::Center)
            .build();

        let browse_btn = gtk::Button::builder()
            .label("Browse File...")
            .css_classes(vec!["pill"])
            .valign(gtk::Align::Center)
            .build();

        action_row.add_suffix(&scan_btn);
        action_row.add_suffix(&browse_btn);
        source_group.add(&action_row);

        // Conflict Strategy Group
        let conflict_group = adw::PreferencesGroup::builder()
            .title("Conflict Handling")
            .build();

        let strategy_list = gtk::StringList::new(&[
            "Skip Duplicates (Keep Existing)",
            "Overwrite Existing Connections",
            "Keep Both (Append '(Imported)' suffix)",
        ]);

        let strategy_combo = adw::ComboRow::builder()
            .title("If Connection Already Exists")
            .model(&strategy_list)
            .build();
        conflict_group.add(&strategy_combo);

        // Candidates Preview Group
        let preview_group = adw::PreferencesGroup::builder()
            .title("Found Connections")
            .description("Select which connections to import into VER")
            .build();

        let candidates_box = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .css_classes(vec!["boxed-list"])
            .build();

        let candidates: Rc<RefCell<Vec<ImportCandidate>>> = Rc::new(RefCell::new(Vec::new()));

        let select_all_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let select_all_btn = gtk::Button::builder().label("Select All").css_classes(vec!["flat"]).build();
        let deselect_all_btn = gtk::Button::builder().label("Deselect All").css_classes(vec!["flat"]).build();
        select_all_box.append(&select_all_btn);
        select_all_box.append(&deselect_all_btn);
        select_all_box.set_visible(false);

        preview_group.add(&select_all_box);
        preview_group.add(&candidates_box);

        // Bottom Action Buttons
        let bottom_box = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        bottom_box.set_halign(gtk::Align::End);
        bottom_box.set_margin_top(8);

        let cancel_btn = gtk::Button::builder().label("Cancel").build();
        let import_btn = gtk::Button::builder()
            .label("Import (0)")
            .css_classes(vec!["suggested-action", "pill"])
            .sensitive(false)
            .build();

        bottom_box.append(&cancel_btn);
        bottom_box.append(&import_btn);

        content_box.append(&source_group);
        content_box.append(&conflict_group);
        content_box.append(&preview_group);
        content_box.append(&bottom_box);

        let clamp = adw::Clamp::builder()
            .maximum_size(600)
            .child(&content_box)
            .build();

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .child(&clamp)
            .build();

        toolbar_view.set_content(Some(&scrolled));
        window.set_content(Some(&toolbar_view));

        // State update helper
        let refresh_list = {
            let candidates = candidates.clone();
            let candidates_box = candidates_box.clone();
            let select_all_box = select_all_box.clone();
            let import_btn = import_btn.clone();

            move || {
                while let Some(child) = candidates_box.first_child() {
                    candidates_box.remove(&child);
                }

                let items = candidates.borrow();
                if items.is_empty() {
                    select_all_box.set_visible(false);
                    import_btn.set_sensitive(false);
                    import_btn.set_label("Import (0)");
                    return;
                }

                select_all_box.set_visible(true);
                let mut selected_count = 0;

                for cand in items.iter() {
                    let is_sel = *cand.selected.borrow();
                    if is_sel {
                        selected_count += 1;
                    }

                    let row = adw::ActionRow::builder()
                        .title(&cand.connection.name)
                        .subtitle(format!(
                            "{} | {}:{} ({})",
                            cand.connection.protocol.as_str(),
                            cand.connection.host,
                            cand.connection.port,
                            cand.source_desc
                        ))
                        .build();

                    let check = gtk::CheckButton::builder()
                        .active(is_sel)
                        .valign(gtk::Align::Center)
                        .build();

                    let selected_rc = cand.selected.clone();
                    let import_btn_clone = import_btn.clone();
                    let candidates_clone = candidates.clone();
                    check.connect_toggled(move |btn| {
                        *selected_rc.borrow_mut() = btn.is_active();
                        let count = candidates_clone
                            .borrow()
                            .iter()
                            .filter(|c| *c.selected.borrow())
                            .count();
                        import_btn_clone.set_label(&format!("Import ({})", count));
                        import_btn_clone.set_sensitive(count > 0);
                    });

                    row.add_prefix(&check);
                    candidates_box.append(&row);
                }

                import_btn.set_label(&format!("Import ({})", selected_count));
                import_btn.set_sensitive(selected_count > 0);
            }
        };

        // Auto-Scan handler
        let candidates_scan = candidates.clone();
        let refresh_scan = refresh_list.clone();
        let source_combo_scan = source_combo.clone();
        scan_btn.connect_clicked(move |_| {
            let selected_type = source_combo_scan.selected();
            let mut found: Vec<ImportCandidate> = Vec::new();

            match selected_type {
                0 => {
                    // Remmina
                    for (path, res) in scan_remmina_profiles() {
                        if let Ok(conn) = res {
                            let desc = path
                                .file_name()
                                .and_then(|s| s.to_str())
                                .unwrap_or("Remmina")
                                .to_string();
                            found.push(ImportCandidate {
                                connection: conn,
                                source_desc: desc,
                                selected: Rc::new(RefCell::new(true)),
                            });
                        }
                    }
                }
                1 => {
                    // SSH Config
                    if let Some(home) = dirs::home_dir() {
                        let ssh_cfg = home.join(".ssh").join("config");
                        if ssh_cfg.exists() {
                            if let Ok(conns) = import_ssh_config_file(&ssh_cfg) {
                                for conn in conns {
                                    found.push(ImportCandidate {
                                        connection: conn,
                                        source_desc: "~/.ssh/config".to_string(),
                                        selected: Rc::new(RefCell::new(true)),
                                    });
                                }
                            }
                        }
                    }
                }
                _ => {}
            }

            *candidates_scan.borrow_mut() = found;
            refresh_scan();
        });

        // Browse File handler
        let candidates_browse = candidates.clone();
        let refresh_browse = refresh_list.clone();
        let source_combo_browse = source_combo.clone();
        let window_browse = window.clone();
        browse_btn.connect_clicked(move |_| {
            let dialog = gtk::FileChooserNative::new(
                Some("Select Configuration File to Import"),
                Some(&window_browse),
                gtk::FileChooserAction::Open,
                Some("Open"),
                Some("Cancel"),
            );

            let candidates_inner = candidates_browse.clone();
            let refresh_inner = refresh_browse.clone();
            let source_idx = source_combo_browse.selected();

            dialog.connect_response(move |d, response| {
                if response == gtk::ResponseType::Accept {
                    if let Some(file) = d.file() {
                        if let Some(path) = file.path() {
                            let mut found = Vec::new();
                            match source_idx {
                                0 => {
                                    if let Ok(conn) = import_remmina_file(&path) {
                                        let desc = path.file_name().and_then(|s| s.to_str()).unwrap_or("Remmina").to_string();
                                        found.push(ImportCandidate {
                                            connection: conn,
                                            source_desc: desc,
                                            selected: Rc::new(RefCell::new(true)),
                                        });
                                    }
                                }
                                1 => {
                                    if let Ok(conns) = import_ssh_config_file(&path) {
                                        let desc = path.file_name().and_then(|s| s.to_str()).unwrap_or("SSH Config").to_string();
                                        for conn in conns {
                                            found.push(ImportCandidate {
                                                connection: conn,
                                                source_desc: desc.clone(),
                                                selected: Rc::new(RefCell::new(true)),
                                            });
                                        }
                                    }
                                }
                                2 => {
                                    if let Ok(conn) = import_rdp_file(&path) {
                                        let desc = path.file_name().and_then(|s| s.to_str()).unwrap_or("RDP File").to_string();
                                        found.push(ImportCandidate {
                                            connection: conn,
                                            source_desc: desc,
                                            selected: Rc::new(RefCell::new(true)),
                                        });
                                    }
                                }
                                3 => {
                                    if let Ok(content) = std::fs::read_to_string(&path) {
                                        if let Ok(conns) = import_connections_json(&content) {
                                            let desc = path.file_name().and_then(|s| s.to_str()).unwrap_or("JSON Backup").to_string();
                                            for conn in conns {
                                                found.push(ImportCandidate {
                                                    connection: conn,
                                                    source_desc: desc.clone(),
                                                    selected: Rc::new(RefCell::new(true)),
                                                });
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }
                            *candidates_inner.borrow_mut() = found;
                            refresh_inner();
                        }
                    }
                }
            });

            dialog.show();
        });

        // Select All / Deselect All
        let candidates_sa = candidates.clone();
        let refresh_sa = refresh_list.clone();
        select_all_btn.connect_clicked(move |_| {
            for c in candidates_sa.borrow().iter() {
                *c.selected.borrow_mut() = true;
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

        // Import Button
        let on_import = Rc::new(on_import);
        let win_import = window.downgrade();
        let candidates_final = candidates.clone();
        let strategy_combo_final = strategy_combo.clone();
        import_btn.connect_clicked(move |_| {
            let strategy = match strategy_combo_final.selected() {
                0 => ImportConflictStrategy::SkipDuplicates,
                1 => ImportConflictStrategy::Overwrite,
                _ => ImportConflictStrategy::RenameWithSuffix,
            };

            let to_import: Vec<Connection> = candidates_final
                .borrow()
                .iter()
                .filter(|c| *c.selected.borrow())
                .map(|c| c.connection.clone())
                .collect();

            if let Some(win) = win_import.upgrade() {
                win.close();
            }

            on_import(to_import, strategy);
        });

        window.present();
    }
}
