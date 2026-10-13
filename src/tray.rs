use gtk::gdk_pixbuf::Pixbuf;
use ksni::{menu::StandardItem, Icon, MenuItem, Tray};
use std::io::Cursor;
use std::sync::OnceLock;

static TRAY_ICON: OnceLock<Option<Icon>> = OnceLock::new();

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayMessage {
    Show,
    Quit,
}

pub struct VerTray {
    pub tx: async_channel::Sender<TrayMessage>,
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
        vec![
            StandardItem {
                label: "Show".into(),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.tx.try_send(TrayMessage::Show);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Quit".into(),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.tx.try_send(TrayMessage::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}
