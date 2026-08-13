use ksni::{Tray, MenuItem, menu::StandardItem};

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
        "com.example.ver".into()
    }

    fn icon_name(&self) -> String {
        "com.example.ver".into()
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
            }.into(),
            StandardItem {
                label: "Quit".into(),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.tx.try_send(TrayMessage::Quit);
                }),
                ..Default::default()
            }.into(),
        ]
    }
}

