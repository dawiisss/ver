use std::cell::RefCell;
use std::rc::Rc;
use gtk::gdk;
use gtk::glib;
use gtk::glib::translate::IntoGlib;
use gtk::prelude::*;
use tokio::sync::mpsc::UnboundedSender;

use crate::models::VncScaling;
use crate::vnc::client::{VncCommand, VncEventLocal, VncFrameUpdate};

pub struct VncWidget {
    pub scaling: VncScaling,
    pub current_frame: Option<VncFrameUpdate>,
    pub events_sent: Vec<VncEventLocal>,
    picture: Option<gtk::Picture>,
    container: Option<gtk::ScrolledWindow>,
    cmd_tx: Option<UnboundedSender<VncCommand>>,
}

impl VncWidget {
    pub fn new(scaling: VncScaling) -> Self {
        let (picture, container) = if gtk::is_initialized() {
            let pic = gtk::Picture::builder()
                .can_shrink(false)
                .keep_aspect_ratio(true)
                .focusable(true)
                .hexpand(true)
                .vexpand(true)
                .cursor(&gtk::gdk::Cursor::from_name("crosshair", None).unwrap())
                .build();

            let cont = gtk::ScrolledWindow::builder()
                .hscrollbar_policy(gtk::PolicyType::Automatic)
                .vscrollbar_policy(gtk::PolicyType::Automatic)
                .overlay_scrolling(false)
                .child(&pic)
                .vexpand(true)
                .hexpand(true)
                .build();

            (Some(pic), Some(cont))
        } else {
            (None, None)
        };

        let widget = Self {
            scaling,
            current_frame: None,
            events_sent: Vec::new(),
            picture,
            container,
            cmd_tx: None,
        };

        widget.apply_scaling(scaling);
        widget
    }

    pub fn set_cmd_tx(&mut self, cmd_tx: UnboundedSender<VncCommand>) {
        self.cmd_tx = Some(cmd_tx);
    }

    pub fn widget(&self) -> Option<&gtk::ScrolledWindow> {
        self.container.as_ref()
    }

    pub fn picture(&self) -> Option<&gtk::Picture> {
        self.picture.as_ref()
    }

    pub fn render_frame(&mut self, frame: VncFrameUpdate) {
        if let Some(ref picture) = self.picture {
            if frame.width > 0 && frame.height > 0 {
                let bytes = glib::Bytes::from_owned(frame.pixels.clone());
                let texture = gdk::MemoryTexture::new(
                    frame.width as i32,
                    frame.height as i32,
                    gdk::MemoryFormat::B8g8r8a8Premultiplied,
                    &bytes,
                    frame.stride,
                );
                picture.set_paintable(Some(&texture));
            }
        }
        self.current_frame = Some(frame);
    }

    pub fn send_key_event(&mut self, keysym: u32, down: bool) {
        self.events_sent.push(VncEventLocal::Key { keysym, down });
        if let Some(ref tx) = self.cmd_tx {
            let _ = tx.send(VncCommand::KeyEvent { keysym, down });
        }
    }

    pub fn send_pointer_event(&mut self, x: u16, y: u16, mask: u8) {
        self.events_sent.push(VncEventLocal::Pointer { x, y, mask });
        if let Some(ref tx) = self.cmd_tx {
            let _ = tx.send(VncCommand::PointerEvent { x, y, mask });
        }
    }

    pub fn set_scaling(&mut self, scaling: VncScaling) {
        self.scaling = scaling;
        self.apply_scaling(scaling);
        if let Some(ref tx) = self.cmd_tx {
            let _ = tx.send(VncCommand::SetScaling(scaling));
        }
    }

    fn apply_scaling(&self, scaling: VncScaling) {
        if let (Some(picture), Some(container)) = (&self.picture, &self.container) {
            match scaling {
                VncScaling::OriginalSize => {
                    picture.set_can_shrink(false);
                    picture.set_keep_aspect_ratio(true);
                    container.set_hscrollbar_policy(gtk::PolicyType::Automatic);
                    container.set_vscrollbar_policy(gtk::PolicyType::Automatic);
                }
                VncScaling::FitToWindow => {
                    picture.set_can_shrink(true);
                    picture.set_keep_aspect_ratio(true);
                    container.set_hscrollbar_policy(gtk::PolicyType::Never);
                    container.set_vscrollbar_policy(gtk::PolicyType::Never);
                }
                VncScaling::Stretch => {
                    picture.set_can_shrink(true);
                    picture.set_keep_aspect_ratio(false);
                    container.set_hscrollbar_policy(gtk::PolicyType::Never);
                    container.set_vscrollbar_policy(gtk::PolicyType::Never);
                }
            }
        }
    }

    pub fn translate_coordinates(&self, local_x: f64, local_y: f64) -> (u16, u16) {
        let (fw_u32, fh_u32) = match &self.current_frame {
            Some(f) => (f.width, f.height),
            None => return (0, 0),
        };

        if fw_u32 == 0 || fh_u32 == 0 {
            return (0, 0);
        }

        let fw = fw_u32 as f64;
        let fh = fh_u32 as f64;

        let (mut ww, mut wh) = match &self.picture {
            Some(p) => (p.width() as f64, p.height() as f64),
            None => (fw, fh),
        };

        if ww <= 0.0 || wh <= 0.0 {
            ww = fw;
            wh = fh;
        }

        let (rx, ry) = match self.scaling {
            VncScaling::OriginalSize => (local_x, local_y),
            VncScaling::Stretch => (
                (local_x / ww) * fw,
                (local_y / wh) * fh,
            ),
            VncScaling::FitToWindow => {
                let scale = (ww / fw).min(wh / fh);
                let offset_x = (ww - fw * scale) / 2.0;
                let offset_y = (wh - fh * scale) / 2.0;

                (
                    (local_x - offset_x) / scale,
                    (local_y - offset_y) / scale,
                )
            }
        };

        let max_x = (fw_u32.saturating_sub(1)) as f64;
        let max_y = (fh_u32.saturating_sub(1)) as f64;

        let clamped_x = rx.clamp(0.0, max_x).round() as u16;
        let clamped_y = ry.clamp(0.0, max_y).round() as u16;

        (clamped_x, clamped_y)
    }

    pub fn setup_event_controllers(&self, widget_rc: Rc<RefCell<VncWidget>>) {
        let picture = match &self.picture {
            Some(p) => p.clone(),
            None => return,
        };

        // 1. Keyboard Controller
        let key_controller = gtk::EventControllerKey::new();
        let w1 = widget_rc.clone();
        key_controller.connect_key_pressed(move |_, keyval, _, _| {
            let keysym = keyval.into_glib();
            w1.borrow_mut().send_key_event(keysym, true);
            glib::Propagation::Stop
        });

        let w2 = widget_rc.clone();
        key_controller.connect_key_released(move |_, keyval, _, _| {
            let keysym = keyval.into_glib();
            w2.borrow_mut().send_key_event(keysym, false);
        });

        picture.add_controller(key_controller);

        // 2. Motion Controller
        let motion_controller = gtk::EventControllerMotion::new();
        let w3 = widget_rc.clone();
        let current_mask = Rc::new(RefCell::new(0u8));
        let mask_motion = current_mask.clone();

        motion_controller.connect_motion(move |_, x, y| {
            let mut w = w3.borrow_mut();
            let (rx, ry) = w.translate_coordinates(x, y);
            let mask = *mask_motion.borrow();
            w.send_pointer_event(rx, ry, mask);
        });

        picture.add_controller(motion_controller);

        // 3. Click Controller
        let click_controller = gtk::GestureClick::new();
        click_controller.set_button(0); // All buttons

        let w4 = widget_rc.clone();
        let mask_press = current_mask.clone();
        let pic_press = picture.clone();
        click_controller.connect_pressed(move |gesture, _, x, y| {
            pic_press.grab_focus();
            let button = gesture.current_button();
            let bit = match button {
                1 => 0x01, // Left
                2 => 0x02, // Middle
                3 => 0x04, // Right
                _ => 0,
            };
            {
                let mut m = mask_press.borrow_mut();
                *m |= bit;
            }
            let mut w = w4.borrow_mut();
            let (rx, ry) = w.translate_coordinates(x, y);
            let mask = *mask_press.borrow();
            w.send_pointer_event(rx, ry, mask);
        });

        let w5 = widget_rc.clone();
        let mask_release = current_mask.clone();
        click_controller.connect_released(move |gesture, _, x, y| {
            let button = gesture.current_button();
            let bit = match button {
                1 => 0x01,
                2 => 0x02,
                3 => 0x04,
                _ => 0,
            };
            {
                let mut m = mask_release.borrow_mut();
                *m &= !bit;
            }
            let mut w = w5.borrow_mut();
            let (rx, ry) = w.translate_coordinates(x, y);
            let mask = *mask_release.borrow();
            w.send_pointer_event(rx, ry, mask);
        });

        picture.add_controller(click_controller);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translate_coordinates_zero_framebuffer() {
        let mut widget = VncWidget::new(VncScaling::OriginalSize);
        widget.current_frame = Some(VncFrameUpdate {
            width: 0,
            height: 0,
            stride: 0,
            pixels: vec![],
        });
        assert_eq!(widget.translate_coordinates(100.0, 100.0), (0, 0));
    }

    #[test]
    fn test_translate_coordinates_unrealized_widget_clamping() {
        let mut widget = VncWidget::new(VncScaling::OriginalSize);
        widget.current_frame = Some(VncFrameUpdate {
            width: 1000,
            height: 500,
            stride: 4000,
            pixels: vec![0; 1000 * 500 * 4],
        });
        // Unrealized widget (no picture realization or ww/wh <= 0)
        assert_eq!(widget.translate_coordinates(1500.0, 800.0), (999, 499));
        assert_eq!(widget.translate_coordinates(-50.0, -10.0), (0, 0));
    }

    #[test]
    fn test_translate_coordinates_no_frame() {
        let widget = VncWidget::new(VncScaling::OriginalSize);
        assert_eq!(widget.translate_coordinates(100.0, 100.0), (0, 0));
    }
}


