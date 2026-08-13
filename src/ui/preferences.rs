use std::cell::RefCell;
use std::rc::Rc;
use gtk::glib;
use gtk::prelude::*;
use libadwaita::prelude::*;
use libadwaita as adw;

use crate::models::{AppConfig, Protocol, VncScaling};
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

pub struct PreferencesWindow {
    pub config: AppConfig,
}

impl PreferencesWindow {
    pub fn new(config: AppConfig) -> Self {
        Self { config }
    }

    pub fn set_theme(&mut self, theme: &str) {
        self.config.theme = theme.to_string();
        apply_theme(theme);
    }

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
        let group_appearance = adw::PreferencesGroup::builder()
            .title("Appearance")
            .build();

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

        let config_clone_1 = config.clone();
        combo_theme.connect_selected_notify(move |row| {
            let theme = match row.selected() {
                1 => "dark",
                2 => "light",
                _ => "system",
            };
            config_clone_1.borrow_mut().theme = theme.to_string();
            apply_theme(theme);
            let _ = save_config(&config_clone_1.borrow());
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

        let config_clone_2 = config.clone();
        combo_proto.connect_selected_notify(move |row| {
            let proto = match row.selected() {
                1 => Protocol::Vnc,
                2 => Protocol::Ssh,
                3 => Protocol::Spice,
                4 => Protocol::Xrdp,
                _ => Protocol::Rdp,
            };
            config_clone_2.borrow_mut().default_protocol = proto;
            let _ = save_config(&config_clone_2.borrow());
        });

        group_defaults.add(&combo_proto);

        // Auto-connect Last Session
        let switch_autoconnect = adw::SwitchRow::builder()
            .title("Auto-connect Last Session")
            .subtitle("Automatically launch the last used connection on startup")
            .active(config.borrow().auto_connect_last)
            .build();

        let config_clone_3 = config.clone();
        switch_autoconnect.connect_active_notify(move |row| {
            config_clone_3.borrow_mut().auto_connect_last = row.is_active();
            let _ = save_config(&config_clone_3.borrow());
        });

        group_defaults.add(&switch_autoconnect);

        // Default VNC Scaling
        let scaling_model = gtk::StringList::new(&["Original Size", "Fit to Window", "Stretch"]);
        let default_scaling_idx = match config.borrow().default_vnc_scaling {
            VncScaling::OriginalSize => 0,
            VncScaling::FitToWindow => 1,
            VncScaling::Stretch => 2,
        };

        let combo_scaling = adw::ComboRow::builder()
            .title("Default VNC Display Scaling")
            .model(&scaling_model)
            .selected(default_scaling_idx)
            .build();

        let config_clone_4 = config.clone();
        combo_scaling.connect_selected_notify(move |row| {
            let scaling = match row.selected() {
                1 => VncScaling::FitToWindow,
                2 => VncScaling::Stretch,
                _ => VncScaling::OriginalSize,
            };
            config_clone_4.borrow_mut().default_vnc_scaling = scaling;
            let _ = save_config(&config_clone_4.borrow());
        });

        group_defaults.add(&combo_scaling);
        page.add(&group_defaults);

        window.add(&page);
        window
    }
}
