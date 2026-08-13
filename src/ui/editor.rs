use gtk::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;

use crate::models::{AdvancedSettings, Connection, Protocol, RdpColorDepth, RdpNetworkProfile};

pub struct ConnectionEditor {
    pub connection: Connection,
    pub password: String,
    pub is_dirty: bool,
}

impl ConnectionEditor {
    pub fn new(connection: Connection, password: String) -> Self {
        Self {
            connection,
            password,
            is_dirty: false,
        }
    }

    pub fn update_name(&mut self, name: &str) {
        self.connection.name = name.to_string();
        self.is_dirty = true;
    }

    pub fn update_host(&mut self, host: &str) {
        self.connection.host = host.to_string();
        self.is_dirty = true;
    }

    pub fn update_port(&mut self, port: u16) {
        self.connection.port = port;
        self.is_dirty = true;
    }

    pub fn update_password(&mut self, password: &str) {
        self.password = password.to_string();
        self.is_dirty = true;
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.connection.name.trim().is_empty() {
            return Err("Connection name cannot be empty".to_string());
        }
        if self.connection.host.trim().is_empty() {
            return Err("Host address cannot be empty".to_string());
        }
        if self.connection.port == 0 {
            return Err("Port must be a valid number between 1 and 65535".to_string());
        }
        self.connection.validate_mac()?;
        Ok(())
    }

    /// Build full GTK4 / Libadwaita ConnectionEditor widget wrapper with form fields, switches, and action buttons.
    pub fn build_widget<FSave, FConn, FDup, FDel, FWake>(
        conn: Connection,
        password: String,
        on_save: FSave,
        on_connect: FConn,
        on_duplicate: FDup,
        on_delete: FDel,
        on_wake: FWake,
    ) -> adw::ToastOverlay
    where
        FSave: Fn(Connection, String) + 'static,
        FConn: Fn(Connection, String) + 'static,
        FDup: Fn(Connection, String) + 'static,
        FDel: Fn(String) + 'static,
        FWake: Fn(String) + 'static,
    {
        let toast_overlay = adw::ToastOverlay::new();

        let page = adw::PreferencesPage::new();

        // 1. General Settings
        let group_general = adw::PreferencesGroup::builder()
            .title("General Settings")
            .build();

        let entry_name = adw::EntryRow::builder()
            .title("Name")
            .text(&conn.name)
            .build();

        let entry_group = adw::EntryRow::builder()
            .title("Group")
            .text(&conn.group)
            .build();

        let proto_model = gtk::StringList::new(&["RDP", "VNC", "SSH", "SPICE", "XRDP"]);
        let selected_proto_idx = match conn.protocol {
            Protocol::Rdp => 0,
            Protocol::Vnc => 1,
            Protocol::Ssh => 2,
            Protocol::Spice => 3,
            Protocol::Xrdp => 4,
        };

        let combo_proto = adw::ComboRow::builder()
            .title("Protocol")
            .model(&proto_model)
            .selected(selected_proto_idx)
            .build();

        let entry_host = adw::EntryRow::builder()
            .title("Host (IP or Domain)")
            .text(&conn.host)
            .build();

        let entry_port = adw::EntryRow::builder()
            .title("Port")
            .text(conn.port.to_string())
            .build();

        let entry_username = adw::EntryRow::builder()
            .title("Username")
            .text(&conn.username)
            .build();

        let entry_password = adw::PasswordEntryRow::builder()
            .title("Password")
            .text(&password)
            .build();

        group_general.add(&entry_name);
        group_general.add(&entry_group);
        group_general.add(&combo_proto);
        group_general.add(&entry_host);
        group_general.add(&entry_port);
        group_general.add(&entry_username);
        group_general.add(&entry_password);

        page.add(&group_general);

        // 2. Network & Hardware
        let group_network = adw::PreferencesGroup::builder()
            .title("Network &amp; Hardware")
            .build();

        let entry_mac = adw::EntryRow::builder()
            .title("MAC Address (for Wake-on-LAN)")
            .text(&conn.mac_address)
            .build();

        group_network.add(&entry_mac);
        page.add(&group_network);

        // 3. Advanced RDP Settings
        let group_rdp = adw::PreferencesGroup::builder()
            .title("Advanced RDP Settings")
            .build();

        let switch_rdp_fullscreen = adw::SwitchRow::builder()
            .title("Fullscreen Mode")
            .active(conn.advanced_settings.rdp_fullscreen)
            .build();

        let switch_rdp_multimon = adw::SwitchRow::builder()
            .title("Multi-Monitor Support")
            .active(conn.advanced_settings.rdp_multimon)
            .build();

        let switch_rdp_audio = adw::SwitchRow::builder()
            .title("Audio Redirection")
            .active(conn.advanced_settings.rdp_audio)
            .build();

        let entry_rdp_domain = adw::EntryRow::builder()
            .title("Domain")
            .text(&conn.advanced_settings.rdp_domain)
            .build();

        let entry_rdp_gateway = adw::EntryRow::builder()
            .title("RD Gateway")
            .text(&conn.advanced_settings.rdp_gateway)
            .build();

        let entry_rdp_shared_folder = adw::EntryRow::builder()
            .title("Shared Folder (Local Path)")
            .text(&conn.advanced_settings.rdp_shared_folder)
            .build();

        let switch_rdp_dynamic_res = adw::SwitchRow::builder()
            .title("Dynamic Resolution")
            .active(conn.advanced_settings.rdp_dynamic_resolution)
            .build();

        let entry_rdp_custom_res = adw::EntryRow::builder()
            .title("Custom Resolution (e.g. 1920x1080)")
            .text(&conn.advanced_settings.rdp_custom_resolution)
            .build();

        let network_model = gtk::StringList::new(&["Auto", "LAN", "WAN", "Broadband", "Modem"]);
        let network_idx = match conn.advanced_settings.rdp_network_profile {
            RdpNetworkProfile::Auto => 0,
            RdpNetworkProfile::Lan => 1,
            RdpNetworkProfile::Wan => 2,
            RdpNetworkProfile::Broadband => 3,
            RdpNetworkProfile::Modem => 4,
        };
        let combo_rdp_network = adw::ComboRow::builder()
            .title("Network Profile")
            .model(&network_model)
            .selected(network_idx)
            .build();

        let switch_rdp_disable_wallpaper = adw::SwitchRow::builder()
            .title("Disable Wallpaper")
            .active(conn.advanced_settings.rdp_disable_wallpaper)
            .build();

        let switch_rdp_disable_themes = adw::SwitchRow::builder()
            .title("Disable Themes")
            .active(conn.advanced_settings.rdp_disable_themes)
            .build();

        let switch_rdp_disable_animations = adw::SwitchRow::builder()
            .title("Disable Animations")
            .active(conn.advanced_settings.rdp_disable_animations)
            .build();

        let switch_rdp_glyph_cache = adw::SwitchRow::builder()
            .title("Glyph Caching")
            .subtitle("Improves performance by caching font glyphs locally")
            .active(conn.advanced_settings.rdp_glyph_cache)
            .build();

        let switch_rdp_microphone = adw::SwitchRow::builder()
            .title("Microphone Redirection")
            .subtitle("Forward local microphone to the remote host")
            .active(conn.advanced_settings.rdp_microphone)
            .build();

        let switch_rdp_usb_redirect = adw::SwitchRow::builder()
            .title("USB Redirection (Auto)")
            .subtitle("Automatically redirect connected USB devices")
            .active(conn.advanced_settings.rdp_usb_redirect)
            .build();

        let switch_rdp_smooth_fonts = adw::SwitchRow::builder()
            .title("Smooth Fonts (ClearType)")
            .active(conn.advanced_settings.rdp_smooth_fonts)
            .build();

        let switch_rdp_desktop_composition = adw::SwitchRow::builder()
            .title("Desktop Composition (Aero)")
            .active(conn.advanced_settings.rdp_desktop_composition)
            .build();

        let switch_rdp_hw_accel = adw::SwitchRow::builder()
            .title("Hardware Graphics (GFX)")
            .subtitle("Use hardware accelerated graphics pipeline if supported")
            .active(conn.advanced_settings.rdp_hw_accel)
            .build();

        group_rdp.add(&switch_rdp_fullscreen);
        group_rdp.add(&switch_rdp_multimon);
        group_rdp.add(&switch_rdp_audio);
        group_rdp.add(&switch_rdp_microphone);
        group_rdp.add(&switch_rdp_usb_redirect);
        group_rdp.add(&entry_rdp_domain);
        group_rdp.add(&entry_rdp_gateway);
        group_rdp.add(&entry_rdp_shared_folder);
        group_rdp.add(&combo_rdp_network);
        group_rdp.add(&switch_rdp_dynamic_res);
        group_rdp.add(&entry_rdp_custom_res);
        group_rdp.add(&switch_rdp_glyph_cache);
        group_rdp.add(&switch_rdp_smooth_fonts);
        group_rdp.add(&switch_rdp_desktop_composition);
        group_rdp.add(&switch_rdp_hw_accel);
        group_rdp.add(&switch_rdp_disable_wallpaper);
        group_rdp.add(&switch_rdp_disable_themes);
        group_rdp.add(&switch_rdp_disable_animations);
        page.add(&group_rdp);

        // 4. Advanced VNC Settings
        let group_vnc = adw::PreferencesGroup::builder()
            .title("Advanced VNC Settings")
            .build();

        let switch_vnc_fullscreen = adw::SwitchRow::builder()
            .title("Fullscreen")
            .active(conn.advanced_settings.vnc_fullscreen)
            .build();

        let switch_vnc_clipboard = adw::SwitchRow::builder()
            .title("Clipboard Sync")
            .active(conn.advanced_settings.vnc_clipboard)
            .build();

        let color_model =
            gtk::StringList::new(&["Full Color (Default)", "Medium", "Low", "Very Low"]);
        let vnc_color_idx = match conn.advanced_settings.vnc_color_level {
            crate::models::VncColorLevel::Full => 0,
            crate::models::VncColorLevel::Medium => 1,
            crate::models::VncColorLevel::Low => 2,
            crate::models::VncColorLevel::VeryLow => 3,
        };
        let combo_vnc_color = adw::ComboRow::builder()
            .title("Color Level")
            .model(&color_model)
            .selected(vnc_color_idx)
            .build();

        let compress_model = gtk::StringList::new(&[
            "Auto (Default)",
            "1 (Fast)",
            "2",
            "3",
            "4",
            "5",
            "6",
            "7",
            "8",
            "9 (Best)",
        ]);
        let combo_vnc_compress = adw::ComboRow::builder()
            .title("Compression Level")
            .model(&compress_model)
            .selected(conn.advanced_settings.vnc_compress_level as u32)
            .build();

        let quality_model = gtk::StringList::new(&[
            "Auto (Default)",
            "1 (Low)",
            "2",
            "3",
            "4",
            "5",
            "6",
            "7",
            "8",
            "9 (High)",
        ]);
        let combo_vnc_quality = adw::ComboRow::builder()
            .title("JPEG Quality Level")
            .model(&quality_model)
            .selected(conn.advanced_settings.vnc_quality_level as u32)
            .build();

        let switch_vnc_viewonly = adw::SwitchRow::builder()
            .title("View-Only Mode")
            .active(conn.advanced_settings.vnc_viewonly)
            .build();

        let switch_vnc_shared = adw::SwitchRow::builder()
            .title("Shared Session")
            .active(conn.advanced_settings.vnc_shared)
            .build();

        let encoding_model = gtk::StringList::new(&["Auto", "Tight", "ZRLE", "Raw"]);
        let vnc_encoding_idx = match conn.advanced_settings.vnc_encoding {
            crate::models::VncEncodingOption::Auto => 0,
            crate::models::VncEncodingOption::Tight => 1,
            crate::models::VncEncodingOption::Zrle => 2,
            crate::models::VncEncodingOption::Raw => 3,
        };

        let combo_vnc_encoding = adw::ComboRow::builder()
            .title("VNC Encoding")
            .model(&encoding_model)
            .selected(vnc_encoding_idx)
            .build();

        group_vnc.add(&switch_vnc_fullscreen);
        group_vnc.add(&switch_vnc_clipboard);
        group_vnc.add(&combo_vnc_color);
        group_vnc.add(&combo_vnc_compress);
        group_vnc.add(&combo_vnc_quality);
        group_vnc.add(&combo_vnc_encoding);
        group_vnc.add(&switch_vnc_viewonly);
        group_vnc.add(&switch_vnc_shared);
        page.add(&group_vnc);

        // 4.5 Advanced SPICE Settings
        let group_spice = adw::PreferencesGroup::builder()
            .title("Advanced SPICE Settings")
            .build();

        let switch_spice_fullscreen = adw::SwitchRow::builder()
            .title("Fullscreen Mode")
            .active(conn.advanced_settings.spice_fullscreen)
            .build();

        let switch_spice_usb_redirect = adw::SwitchRow::builder()
            .title("USB Redirection")
            .active(conn.advanced_settings.spice_usb_redirect)
            .build();

        let switch_spice_scale_to_window = adw::SwitchRow::builder()
            .title("Scale to Window")
            .active(conn.advanced_settings.spice_scale_to_window)
            .build();

        group_spice.add(&switch_spice_fullscreen);
        group_spice.add(&switch_spice_usb_redirect);
        group_spice.add(&switch_spice_scale_to_window);
        page.add(&group_spice);

        // Toggle Visibility based on Protocol
        let update_protocol_visibility = {
            let group_rdp = group_rdp.clone();
            let group_vnc = group_vnc.clone();
            let group_spice = group_spice.clone();
            let entry_port = entry_port.clone();
            move |idx: u32, set_default_port: bool| {
                match idx {
                    0 | 4 => {
                        // RDP / XRDP
                        group_rdp.set_visible(true);
                        group_vnc.set_visible(false);
                        group_spice.set_visible(false);
                        if set_default_port {
                            entry_port.set_text("3389");
                        }
                    }
                    1 => {
                        // VNC
                        group_rdp.set_visible(false);
                        group_vnc.set_visible(true);
                        group_spice.set_visible(false);
                        if set_default_port {
                            entry_port.set_text("5900");
                        }
                    }
                    3 => {
                        // SPICE
                        group_rdp.set_visible(false);
                        group_vnc.set_visible(false);
                        group_spice.set_visible(true);
                        if set_default_port {
                            entry_port.set_text("5900");
                        }
                    }
                    _ => {
                        // SSH
                        group_rdp.set_visible(false);
                        group_vnc.set_visible(false);
                        group_spice.set_visible(false);
                        if set_default_port {
                            entry_port.set_text("22");
                        }
                    }
                }
            }
        };

        update_protocol_visibility(selected_proto_idx, false);

        {
            let update_vis = update_protocol_visibility.clone();
            combo_proto.connect_selected_notify(move |row| {
                update_vis(row.selected(), true);
            });
        }

        // 5. Common Advanced Settings
        let group_common = adw::PreferencesGroup::builder()
            .title("Common Advanced Settings")
            .build();

        let switch_clipboard = adw::SwitchRow::builder()
            .title("Clipboard Sharing")
            .active(conn.advanced_settings.clipboard_sharing)
            .build();

        let color_model = gtk::StringList::new(&[
            "Auto (Default)",
            "GFX AVC444 (32 bpp)",
            "GFX AVC420 (32 bpp)",
            "GFX RFX (32 bpp)",
            "GFX RFX Progressive (32 bpp)",
            "RemoteFX (32 bpp)",
            "True colour (32 bpp)",
            "True colour (24 bpp)",
            "High colour (16 bpp)",
            "High colour (15 bpp)",
            "256 colours (8 bpp)",
        ]);
        let color_idx = match conn.advanced_settings.rdp_color_depth {
            RdpColorDepth::GfxAvc444 => 1,
            RdpColorDepth::GfxAvc420 => 2,
            RdpColorDepth::GfxRfx => 3,
            RdpColorDepth::GfxRfxProgressive => 4,
            RdpColorDepth::RemoteFx => 5,
            RdpColorDepth::TrueColor32 => 6,
            RdpColorDepth::TrueColor24 => 7,
            RdpColorDepth::HighColor16 => 8,
            RdpColorDepth::HighColor15 => 9,
            RdpColorDepth::Colors256 => 10,
            _ => 0,
        };

        let combo_color_depth = adw::ComboRow::builder()
            .title("Color Depth")
            .model(&color_model)
            .selected(color_idx)
            .build();

        group_common.add(&switch_clipboard);
        group_common.add(&combo_color_depth);
        page.add(&group_common);

        // 6. Action Button Bar
        let btn_box = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        btn_box.set_halign(gtk::Align::Center);
        btn_box.set_margin_top(24);
        btn_box.set_margin_bottom(24);

        let btn_delete = gtk::Button::builder()
            .label("Delete")
            .css_classes(vec!["destructive-action"])
            .build();

        let btn_duplicate = gtk::Button::builder().label("Duplicate").build();

        let btn_wake = gtk::Button::builder().label("Wake").build();

        let btn_save = gtk::Button::builder()
            .label("Save")
            .css_classes(vec!["suggested-action"])
            .build();

        let btn_connect = gtk::Button::builder()
            .label("Connect")
            .css_classes(vec!["accent", "pill"])
            .build();

        btn_box.append(&btn_delete);
        btn_box.append(&btn_duplicate);
        btn_box.append(&btn_wake);
        btn_box.append(&btn_save);
        btn_box.append(&btn_connect);

        let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .child(&page)
            .vexpand(true)
            .build();

        container.append(&scrolled);
        container.append(&btn_box);

        toast_overlay.set_child(Some(&container));

        // Helper to extract Connection and Password from form
        let conn_id = conn.id.clone();
        let mac_for_extract = entry_mac.clone();
        let extract_form = move || -> Result<(Connection, String), String> {
            let name = entry_name.text().to_string();
            if name.trim().is_empty() {
                return Err("Connection name cannot be empty".to_string());
            }

            let host = entry_host.text().to_string();
            if host.trim().is_empty() {
                return Err("Host address cannot be empty".to_string());
            }

            let port_str = entry_port.text().to_string();
            let port: u16 = match port_str.parse() {
                Ok(p) if p > 0 => p,
                _ => return Err("Port must be a valid number between 1 and 65535".to_string()),
            };

            let protocol = match combo_proto.selected() {
                1 => Protocol::Vnc,
                2 => Protocol::Ssh,
                3 => Protocol::Spice,
                4 => Protocol::Xrdp,
                _ => Protocol::Rdp,
            };

            let vnc_color_level = match combo_vnc_color.selected() {
                1 => crate::models::VncColorLevel::Medium,
                2 => crate::models::VncColorLevel::Low,
                3 => crate::models::VncColorLevel::VeryLow,
                _ => crate::models::VncColorLevel::Full,
            };

            let vnc_encoding = match combo_vnc_encoding.selected() {
                1 => crate::models::VncEncodingOption::Tight,
                2 => crate::models::VncEncodingOption::Zrle,
                3 => crate::models::VncEncodingOption::Raw,
                _ => crate::models::VncEncodingOption::Auto,
            };

            let rdp_color_depth = match combo_color_depth.selected() {
                1 => RdpColorDepth::GfxAvc444,
                2 => RdpColorDepth::GfxAvc420,
                3 => RdpColorDepth::GfxRfx,
                4 => RdpColorDepth::GfxRfxProgressive,
                5 => RdpColorDepth::RemoteFx,
                6 => RdpColorDepth::TrueColor32,
                7 => RdpColorDepth::TrueColor24,
                8 => RdpColorDepth::HighColor16,
                9 => RdpColorDepth::HighColor15,
                10 => RdpColorDepth::Colors256,
                _ => RdpColorDepth::Automatic,
            };

            let rdp_network_profile = match combo_rdp_network.selected() {
                1 => RdpNetworkProfile::Lan,
                2 => RdpNetworkProfile::Wan,
                3 => RdpNetworkProfile::Broadband,
                4 => RdpNetworkProfile::Modem,
                _ => RdpNetworkProfile::Auto,
            };

            let group_str = entry_group.text().to_string();
            let group = if group_str.trim().is_empty() {
                "Default".to_string()
            } else {
                group_str.trim().to_string()
            };

            let connection = Connection {
                id: conn_id.clone(),
                name: name.trim().to_string(),
                protocol,
                host: host.trim().to_string(),
                port,
                username: entry_username.text().trim().to_string(),
                mac_address: mac_for_extract.text().trim().to_string(),
                group,
                advanced_settings: AdvancedSettings {
                    rdp_fullscreen: switch_rdp_fullscreen.is_active(),
                    rdp_multimon: switch_rdp_multimon.is_active(),
                    rdp_audio: switch_rdp_audio.is_active(),
                    rdp_domain: entry_rdp_domain.text().to_string(),
                    rdp_gateway: entry_rdp_gateway.text().to_string(),
                    rdp_shared_folder: entry_rdp_shared_folder.text().to_string(),
                    rdp_dynamic_resolution: switch_rdp_dynamic_res.is_active(),
                    rdp_custom_resolution: entry_rdp_custom_res.text().to_string(),
                    rdp_network_profile,
                    rdp_disable_wallpaper: switch_rdp_disable_wallpaper.is_active(),
                    rdp_disable_themes: switch_rdp_disable_themes.is_active(),
                    rdp_disable_animations: switch_rdp_disable_animations.is_active(),
                    rdp_glyph_cache: switch_rdp_glyph_cache.is_active(),
                    rdp_microphone: switch_rdp_microphone.is_active(),
                    rdp_usb_redirect: switch_rdp_usb_redirect.is_active(),
                    rdp_smooth_fonts: switch_rdp_smooth_fonts.is_active(),
                    rdp_desktop_composition: switch_rdp_desktop_composition.is_active(),
                    rdp_hw_accel: switch_rdp_hw_accel.is_active(),
                    vnc_viewonly: switch_vnc_viewonly.is_active(),
                    vnc_shared: switch_vnc_shared.is_active(),
                    clipboard_sharing: switch_clipboard.is_active(),
                    color_depth: 0,
                    rdp_color_depth,
                    vnc_fullscreen: switch_vnc_fullscreen.is_active(),
                    vnc_clipboard: switch_vnc_clipboard.is_active(),
                    vnc_color_level,
                    vnc_compress_level: combo_vnc_compress.selected() as u8,
                    vnc_quality_level: combo_vnc_quality.selected() as u8,
                    vnc_encoding,
                    spice_fullscreen: switch_spice_fullscreen.is_active(),
                    spice_usb_redirect: switch_spice_usb_redirect.is_active(),
                    spice_scale_to_window: switch_spice_scale_to_window.is_active(),
                },
            };

            connection.validate_mac()?;

            let pass = entry_password.text().to_string();
            Ok((connection, pass))
        };

        // Wire Action Handlers
        let extract_for_save = extract_form.clone();
        let toast_overlay_save = toast_overlay.clone();
        btn_save.connect_clicked(move |_| match extract_for_save() {
            Ok((c, p)) => {
                on_save(c, p);
                toast_overlay_save.add_toast(adw::Toast::new("Connection saved successfully"));
            }
            Err(err) => {
                toast_overlay_save.add_toast(adw::Toast::new(&err));
            }
        });

        let extract_for_connect = extract_form.clone();
        let toast_overlay_conn = toast_overlay.clone();
        btn_connect.connect_clicked(move |_| match extract_for_connect() {
            Ok((c, p)) => {
                on_connect(c, p);
            }
            Err(err) => {
                toast_overlay_conn.add_toast(adw::Toast::new(&err));
            }
        });

        let extract_for_dup = extract_form.clone();
        let toast_overlay_dup = toast_overlay.clone();
        btn_duplicate.connect_clicked(move |_| match extract_for_dup() {
            Ok((mut c, p)) => {
                c.id = uuid::Uuid::new_v4().to_string();
                c.name = format!("{} (Copy)", c.name);
                on_duplicate(c, p);
                toast_overlay_dup.add_toast(adw::Toast::new("Connection duplicated"));
            }
            Err(err) => {
                toast_overlay_dup.add_toast(adw::Toast::new(&err));
            }
        });

        let del_id = conn.id.clone();
        btn_delete.connect_clicked(move |_| {
            on_delete(del_id.clone());
        });

        let mac_for_wake = entry_mac.clone();
        let toast_overlay_wake = toast_overlay.clone();
        btn_wake.connect_clicked(move |_| {
            let mac_str = mac_for_wake.text().to_string();
            let test_conn = Connection {
                mac_address: mac_str,
                ..Default::default()
            };
            match test_conn.validate_mac() {
                Ok(Some(clean_mac)) => {
                    on_wake(clean_mac);
                    toast_overlay_wake.add_toast(adw::Toast::new("Wake-on-LAN packet sent"));
                }
                Ok(None) => {
                    toast_overlay_wake
                        .add_toast(adw::Toast::new("Please specify a valid MAC address"));
                }
                Err(err) => {
                    toast_overlay_wake.add_toast(adw::Toast::new(&err));
                }
            }
        });

        toast_overlay
    }
}
