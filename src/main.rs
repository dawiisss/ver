use gtk::glib;
use ksni::blocking::TrayMethods;
use libadwaita::prelude::*;
use ver::tray::{TrayMessage, VerTray};
use ver::{load_config, load_connections, ui::apply_theme, ui::MainWindow};

fn main() {
    libadwaita::init().expect("Failed to initialize Libadwaita");
    gtk::Window::set_default_icon_name("com.example.ver");

    let app = libadwaita::Application::builder()
        .application_id("com.example.ver")
        .build();

    let (tx, rx) = async_channel::unbounded();
    let tray = VerTray { tx };
    let _tray_handle = tray.spawn().unwrap();

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
