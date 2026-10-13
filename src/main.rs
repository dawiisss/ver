use gtk::gdk;
use gtk::glib;
use ksni::blocking::TrayMethods;
use libadwaita::prelude::*;
use ver::tray::{TrayMessage, VerTray};
use ver::{load_config, load_connections, ui::apply_theme, ui::MainWindow};

fn ensure_icons_installed() {
    let Some(data_dir) = dirs::data_dir() else {
        return;
    };
    let hicolor = data_dir.join("icons").join("hicolor");
    let dir_256 = hicolor.join("256x256").join("apps");
    let dir_scalable = hicolor.join("scalable").join("apps");
    let dir_pixmaps = data_dir.join("pixmaps");

    let _ = std::fs::create_dir_all(&dir_256);
    let _ = std::fs::create_dir_all(&dir_scalable);
    let _ = std::fs::create_dir_all(&dir_pixmaps);

    let png_dest = dir_256.join("io.github.dawiisss.ver.png");
    if !png_dest.exists() {
        let _ = std::fs::write(
            &png_dest,
            include_bytes!("../data/io.github.dawiisss.ver.png"),
        );
    }

    let svg_dest = dir_scalable.join("io.github.dawiisss.ver.svg");
    if !svg_dest.exists() {
        let _ = std::fs::write(
            &svg_dest,
            include_bytes!("../data/io.github.dawiisss.ver.svg"),
        );
    }

    let pixmap_dest = dir_pixmaps.join("io.github.dawiisss.ver.png");
    if !pixmap_dest.exists() {
        let _ = std::fs::write(
            &pixmap_dest,
            include_bytes!("../data/io.github.dawiisss.ver.png"),
        );
    }
}

fn main() {
    libadwaita::init().expect("Failed to initialize Libadwaita");

    ensure_icons_installed();

    if let Some(display) = gdk::Display::default() {
        let theme = gtk::IconTheme::for_display(&display);
        if let Some(data_dir) = dirs::data_dir() {
            theme.add_search_path(data_dir.join("icons"));
            theme.add_search_path(data_dir.join("icons").join("hicolor"));
            theme.add_search_path(data_dir.join("pixmaps"));
        }
        theme.add_search_path("data");
    }

    gtk::Window::set_default_icon_name("io.github.dawiisss.ver");

    let app = libadwaita::Application::builder()
        .application_id("io.github.dawiisss.ver")
        .build();

    let (tx, rx) = async_channel::unbounded();
    let tray = VerTray { tx };
    let _tray_handle = match tray.spawn() {
        Ok(handle) => Some(handle),
        Err(e) => {
            eprintln!("Warning: StatusNotifierItem system tray unavailable: {}", e);
            None
        }
    };

    let app_clone = app.clone();
    glib::MainContext::default().spawn_local(async move {
        while let Ok(msg) = rx.recv().await {
            match msg {
                TrayMessage::Show => {
                    if let Some(window) = app_clone.active_window() {
                        window.present();
                    } else {
                        app_clone.activate();
                    }
                }
                TrayMessage::Quit => {
                    app_clone.quit();
                }
            }
        }
    });

    app.connect_activate(|app| {
        let config = load_config().unwrap_or_default();
        apply_theme(&config.theme);

        let connections = load_connections().unwrap_or_default();
        let window = MainWindow::build_ui(app, connections, config);
        window.present();
    });

    app.run();
}
