use gtk::glib;
use gtk::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::models::{AppConfig, Protocol, VncColorLevel};
use crate::storage::save_config;

pub fn apply_theme(theme_str: &str) {
    if !gtk::is_initialized() || !glib::MainContext::default().is_owner() {
        return;
    }
    let _ = std::panic::catch_unwind(|| {
        let style_manager = adw::StyleManager::default();
        match theme_str.to_lowercase().as_str() {
            "dark" => style_manager.set_color_scheme(adw::ColorScheme::ForceDark),
            "light" => style_manager.set_color_scheme(adw::ColorScheme::ForceLight),
            _ => style_manager.set_color_scheme(adw::ColorScheme::Default),
        }
    });
}

fn persist_preferences(
    config: &Rc<RefCell<AppConfig>>,
    window: &adw::PreferencesWindow,
    update: impl FnOnce(&mut AppConfig),
) -> bool {
    let mut proposed = config.borrow().clone();
    update(&mut proposed);
    match save_config(&proposed) {
        Ok(()) => {
            *config.borrow_mut() = proposed;
            true
        }
        Err(error) => {
            window.add_toast(adw::Toast::new(&format!(
                "Preferences were not saved: {:#}",
                error
            )));
            false
        }
    }
}

pub struct PreferencesWindow;

impl PreferencesWindow {
    pub fn build_window(
        parent: Option<&impl IsA<gtk::Window>>,
        config: Rc<RefCell<AppConfig>>,
    ) -> adw::PreferencesWindow {
        let window = adw::PreferencesWindow::builder()
            .title("Preferences")
            .modal(true)
            .default_width(520)
            .default_height(480)
            .build();

        if let Some(p) = parent {
            window.set_transient_for(Some(p));
        }

        let page = adw::PreferencesPage::builder()
            .title("General")
            .icon_name("preferences-system-symbolic")
            .build();

        // 1. Appearance Group
        let group_appearance = adw::PreferencesGroup::builder().title("Appearance").build();

        let theme_model = gtk::StringList::new(&["System Default", "Dark Mode", "Light Mode"]);
        let current_theme = config.borrow().theme.clone();
        let selected_theme_idx = match current_theme.to_lowercase().as_str() {
            "dark" => 1,
            "light" => 2,
            _ => 0,
        };

        let combo_theme = adw::ComboRow::builder()
            .title("Application Theme")
            .model(&theme_model)
            .selected(selected_theme_idx)
            .build();

        let changing = Rc::new(Cell::new(false));
        let changing_1 = changing.clone();
        let window_1 = window.downgrade();
        let config_clone_1 = config.clone();
        combo_theme.connect_selected_notify(move |row| {
            if changing_1.get() {
                return;
            }
            let Some(window) = window_1.upgrade() else {
                return;
            };
            let old = match config_clone_1.borrow().theme.as_str() {
                "dark" => 1,
                "light" => 2,
                _ => 0,
            };
            let theme = match row.selected() {
                1 => "dark",
                2 => "light",
                _ => "system",
            };
            if persist_preferences(&config_clone_1, &window, |config| {
                config.theme = theme.to_string()
            }) {
                apply_theme(theme);
            } else {
                changing_1.set(true);
                row.set_selected(old);
                changing_1.set(false);
            }
        });

        group_appearance.add(&combo_theme);
        page.add(&group_appearance);

        // 2. Defaults & Behavior Group
        let group_defaults = adw::PreferencesGroup::builder()
            .title("Defaults &amp; Behavior")
            .build();

        // Default Protocol
        let proto_model = gtk::StringList::new(&["RDP", "VNC", "SSH", "SPICE", "XRDP"]);
        let default_proto_idx = match config.borrow().default_protocol {
            Protocol::Rdp => 0,
            Protocol::Vnc => 1,
            Protocol::Ssh => 2,
            Protocol::Spice => 3,
            Protocol::Xrdp => 4,
        };

        let combo_proto = adw::ComboRow::builder()
            .title("Default Protocol")
            .model(&proto_model)
            .selected(default_proto_idx)
            .build();

        let changing_2 = changing.clone();
        let window_2 = window.downgrade();
        let config_clone_2 = config.clone();
        combo_proto.connect_selected_notify(move |row| {
            if changing_2.get() {
                return;
            }
            let Some(window) = window_2.upgrade() else {
                return;
            };
            let old = match config_clone_2.borrow().default_protocol {
                Protocol::Rdp => 0,
                Protocol::Vnc => 1,
                Protocol::Ssh => 2,
                Protocol::Spice => 3,
                Protocol::Xrdp => 4,
            };
            let proto = match row.selected() {
                1 => Protocol::Vnc,
                2 => Protocol::Ssh,
                3 => Protocol::Spice,
                4 => Protocol::Xrdp,
                _ => Protocol::Rdp,
            };
            if !persist_preferences(&config_clone_2, &window, |config| {
                config.default_protocol = proto
            }) {
                changing_2.set(true);
                row.set_selected(old);
                changing_2.set(false);
            }
        });

        group_defaults.add(&combo_proto);

        // Auto-connect Last Session
        let switch_autoconnect = adw::SwitchRow::builder()
            .title("Auto-connect Last Session")
            .subtitle("Automatically launch the last used connection on startup")
            .active(config.borrow().auto_connect_last)
            .build();

        let changing_3 = changing.clone();
        let window_3 = window.downgrade();
        let config_clone_3 = config.clone();
        switch_autoconnect.connect_active_notify(move |row| {
            if changing_3.get() {
                return;
            }
            let Some(window) = window_3.upgrade() else {
                return;
            };
            let old = config_clone_3.borrow().auto_connect_last;
            if !persist_preferences(&config_clone_3, &window, |config| {
                config.auto_connect_last = row.is_active()
            }) {
                changing_3.set(true);
                row.set_active(old);
                changing_3.set(false);
            }
        });

        group_defaults.add(&switch_autoconnect);

        // Default VNC Color Level
        let color_model =
            gtk::StringList::new(&["Full Color (Default)", "Medium", "Low", "Very Low"]);
        let default_color_idx = match config.borrow().default_vnc_color_level {
            VncColorLevel::Full => 0,
            VncColorLevel::Medium => 1,
            VncColorLevel::Low => 2,
            VncColorLevel::VeryLow => 3,
        };

        let combo_color = adw::ComboRow::builder()
            .title("Default VNC Color Level")
            .model(&color_model)
            .selected(default_color_idx)
            .build();

        let changing_4 = changing.clone();
        let window_4 = window.downgrade();
        let config_clone_4 = config.clone();
        combo_color.connect_selected_notify(move |row| {
            if changing_4.get() {
                return;
            }
            let Some(window) = window_4.upgrade() else {
                return;
            };
            let old = match config_clone_4.borrow().default_vnc_color_level {
                VncColorLevel::Full => 0,
                VncColorLevel::Medium => 1,
                VncColorLevel::Low => 2,
                VncColorLevel::VeryLow => 3,
            };
            let color_level = match row.selected() {
                1 => VncColorLevel::Medium,
                2 => VncColorLevel::Low,
                3 => VncColorLevel::VeryLow,
                _ => VncColorLevel::Full,
            };
            if !persist_preferences(&config_clone_4, &window, |config| {
                config.default_vnc_color_level = color_level
            }) {
                changing_4.set(true);
                row.set_selected(old);
                changing_4.set(false);
            }
        });

        group_defaults.add(&combo_color);
        page.add(&group_defaults);

        window.add(&page);
        window
    }
}
