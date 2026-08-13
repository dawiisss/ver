use ksni::{Tray, MenuItem, menu::StandardItem};

pub struct VerTray {
    pub tx: async_channel::Sender<()>,
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
        let _ = self.tx.try_send(());
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        vec![
            StandardItem {
                label: "Show".into(),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.tx.try_send(());
                }),
                ..Default::default()
            }.into(),
            StandardItem {
                label: "Quit".into(),
                activate: Box::new(|_| {
                    std::process::exit(0);
                }),
                ..Default::default()
            }.into(),
        ]
    }
}
