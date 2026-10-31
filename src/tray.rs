use gtk::gdk_pixbuf::Pixbuf;
use ksni::menu::{StandardItem, SubMenu};
use ksni::{blocking::Handle, Icon, MenuItem, Tray};
use std::io::Cursor;
use std::sync::{mpsc, OnceLock};

use crate::models::{pinned_connections, recent_connections, AppConfig, Connection};

static TRAY_ICON: OnceLock<Option<Icon>> = OnceLock::new();
static MENU_UPDATES: OnceLock<mpsc::Sender<LaunchEntries>> = OnceLock::new();

#[allow(clippy::chunks_exact_to_as_chunks)]
fn get_tray_icon() -> Option<Icon> {
    TRAY_ICON
        .get_or_init(|| {
            let bytes = include_bytes!("../data/tray_icon.png");
            let pixbuf = Pixbuf::from_read(Cursor::new(bytes)).ok()?;
            let width = pixbuf.width();
            let height = pixbuf.height();
            let pixels = pixbuf.read_pixel_bytes();
            let mut data = pixels.as_ref().to_vec();

            if data.len() % 4 != 0 {
                return None;
            }

            for pixel in data.chunks_exact_mut(4) {
                pixel.rotate_right(1);
            }

            Some(Icon {
                width,
                height,
                data,
            })
        })
        .clone()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrayMessage {
    Show,
    Connect(String),
    Quit,
}

/// A connection that can be launched from the tray menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayConnection {
    pub id: String,
    pub name: String,
}

impl From<&Connection> for TrayConnection {
    fn from(conn: &Connection) -> Self {
        Self {
            id: conn.id.clone(),
            name: conn.name.clone(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LaunchEntries {
    pub pinned: Vec<TrayConnection>,
    pub recent: Vec<TrayConnection>,
}

impl LaunchEntries {
    pub fn new(config: &AppConfig, connections: &[Connection]) -> Self {
        Self {
            pinned: pinned_connections(connections)
                .into_iter()
                .map(TrayConnection::from)
                .collect(),
            recent: recent_connections(config, connections)
                .into_iter()
                .map(TrayConnection::from)
                .collect(),
        }
    }
}

/// Escape a name for menus that treat `_` as a mnemonic marker.
pub fn menu_label(name: &str) -> String {
    name.replace('_', "__")
}

/// Apply menu updates on a worker thread, because `Handle::update` waits for the tray service.
pub fn attach_handle(handle: Handle<VerTray>) {
    let (tx, rx) = mpsc::channel::<LaunchEntries>();
    std::thread::spawn(move || {
        while let Ok(mut entries) = rx.recv() {
            // Only the newest menu matters.
            while let Ok(newer) = rx.try_recv() {
                entries = newer;
            }
            if handle.update(|tray| tray.entries = entries).is_none() {
                break;
            }
        }
    });
    let _ = MENU_UPDATES.set(tx);
}

/// Refresh the tray's pinned and recent connections. Does nothing without a tray.
pub fn update_launch_menu(config: &AppConfig, connections: &[Connection]) {
    if let Some(tx) = MENU_UPDATES.get() {
        let _ = tx.send(LaunchEntries::new(config, connections));
    }
}

fn connect_item(entry: &TrayConnection) -> MenuItem<VerTray> {
    let id = entry.id.clone();
    StandardItem {
        label: menu_label(&entry.name),
        activate: Box::new(move |this: &mut VerTray| {
            let _ = this.tx.try_send(TrayMessage::Connect(id.clone()));
        }),
        ..Default::default()
    }
    .into()
}

pub struct VerTray {
    pub tx: async_channel::Sender<TrayMessage>,
    pub entries: LaunchEntries,
}

impl Tray for VerTray {
    fn id(&self) -> String {
        "io.github.dawiisss.ver".into()
    }

    fn icon_name(&self) -> String {
        "io.github.dawiisss.ver".into()
    }

    fn icon_theme_path(&self) -> String {
        dirs::data_dir()
            .map(|d| d.join("icons").to_string_lossy().to_string())
            .unwrap_or_default()
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        get_tray_icon().into_iter().collect()
    }

    fn title(&self) -> String {
        "VER Connection Manager".into()
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = self.tx.try_send(TrayMessage::Show);
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let mut items: Vec<MenuItem<Self>> = vec![StandardItem {
            label: "Show".into(),
            activate: Box::new(|this: &mut Self| {
                let _ = this.tx.try_send(TrayMessage::Show);
            }),
            ..Default::default()
        }
        .into()];

        if !self.entries.pinned.is_empty() {
            items.push(MenuItem::Separator);
            items.extend(self.entries.pinned.iter().map(connect_item));
        }
        if !self.entries.recent.is_empty() {
            items.push(MenuItem::Separator);
            items.push(
                SubMenu {
                    label: "Recent".into(),
                    submenu: self.entries.recent.iter().map(connect_item).collect(),
                    ..Default::default()
                }
                .into(),
            );
        }

        items.push(MenuItem::Separator);
        items.push(
            StandardItem {
                label: "Quit".into(),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.tx.try_send(TrayMessage::Quit);
                }),
                ..Default::default()
            }
            .into(),
        );
        items
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(items: &[MenuItem<VerTray>]) -> Vec<String> {
        items
            .iter()
            .map(|item| match item {
                MenuItem::Standard(item) => item.label.clone(),
                MenuItem::SubMenu(menu) => format!("{} >", menu.label),
                MenuItem::Separator => "--".into(),
                _ => "?".into(),
            })
            .collect()
    }

    fn entry(id: &str, name: &str) -> TrayConnection {
        TrayConnection {
            id: id.into(),
            name: name.into(),
        }
    }

    #[test]
    fn menu_labels_keep_underscores_visible() {
        assert_eq!(menu_label("db_prod_1"), "db__prod__1");
    }

    #[test]
    fn menu_without_entries_has_only_show_and_quit() {
        let (tx, _rx) = async_channel::unbounded();
        let tray = VerTray {
            tx,
            entries: LaunchEntries::default(),
        };
        assert_eq!(labels(&tray.menu()), ["Show", "--", "Quit"]);
    }

    #[test]
    fn menu_lists_pinned_inline_and_recent_in_submenu() {
        let (tx, rx) = async_channel::unbounded();
        let mut tray = VerTray {
            tx,
            entries: LaunchEntries {
                pinned: vec![entry("p1", "web_01")],
                recent: vec![entry("r1", "Router")],
            },
        };
        let mut menu = tray.menu();
        assert_eq!(
            labels(&menu),
            ["Show", "--", "web__01", "--", "Recent >", "--", "Quit"]
        );

        let MenuItem::Standard(pinned) = menu.remove(2) else {
            panic!("pinned entry should be a standard item")
        };
        (pinned.activate)(&mut tray);
        assert_eq!(rx.try_recv(), Ok(TrayMessage::Connect("p1".into())));

        let MenuItem::SubMenu(recent) = menu.remove(3) else {
            panic!("recent entries should be a submenu")
        };
        assert_eq!(labels(&recent.submenu), ["Router"]);
    }

    #[test]
    fn launch_entries_follow_pins_and_history() {
        let connections = vec![
            Connection {
                id: "a".into(),
                name: "A".into(),
                pinned: true,
                ..Default::default()
            },
            Connection {
                id: "b".into(),
                name: "B".into(),
                ..Default::default()
            },
        ];
        let mut config = AppConfig::default();
        config.record_recent("b");
        let entries = LaunchEntries::new(&config, &connections);
        assert_eq!(entries.pinned, [entry("a", "A")]);
        assert_eq!(entries.recent, [entry("b", "B")]);
    }
}
