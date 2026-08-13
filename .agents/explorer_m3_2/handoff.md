# Handoff Report: GTK4 VNC Framebuffer Rendering Widget & Integration Architecture

## 1. Observation

Direct code observations across existing files in `/home/dawiisss/Documents/antigravity/beautiful-goodall`:

1. **Cargo Dependencies (`Cargo.toml` lines 14-25)**:
   - `gtk = { package = "gtk4", version = "0.7" }`
   - `libadwaita = { version = "0.5", features = ["v1_4"] }`
   - `vnc = "0.4.0"`
   - `tokio = { version = "1.34", features = ["full"] }`

2. **Current `VncFrameUpdate` & `VncEvent` (`src/vnc/client.rs` lines 4-15)**:
   ```rust
   pub struct VncFrameUpdate {
       pub width: u32,
       pub height: u32,
       pub stride: usize,
       pub pixels: Vec<u8>, // B8G8R8X8 format
   }

   pub enum VncEvent {
       Key { keysym: u32, down: bool },
       Pointer { x: u16, y: u16, mask: u8 },
   }
   ```
   Pixel layout in `client.rs` lines 34-43:
   `pixels[dst_idx] = B`, `pixels[dst_idx+1] = G`, `pixels[dst_idx+2] = R`, `pixels[dst_idx+3] = 0xFF`.
   This matches GTK4 `gdk::MemoryFormat::B8g8r8x8`.

3. **Current Stub `VncWidget` (`src/vnc/widget.rs` lines 1-35)**:
   ```rust
   pub struct VncWidget {
       pub scaling: VncScaling,
       pub current_frame: Option<VncFrameUpdate>,
       pub events_sent: Vec<VncEvent>,
   }
   ```
   Contains method stubs `render_frame`, `send_key_event`, `send_pointer_event`, and `set_scaling`.

4. **Scaling Model (`src/models.rs` lines 44-58)**:
   ```rust
   pub enum VncScaling {
       OriginalSize, // "Original Size"
       FitToWindow,  // "Fit to Window"
       Stretch,      // "Stretch"
   }
   ```

5. **Existing VNC Unit Tests (`tests/e2e_vnc_tests.rs` lines 1-88)**:
   Tests verify:
   - `client.process_frame_buffer` converts RGB to `B8G8R8X8`.
   - `VncWidget::render_frame` updates `current_frame`.
   - `VncWidget::send_key_event` and `send_pointer_event` record events into `events_sent`.
   - `VncWidget::set_scaling` updates `scaling`.

6. **Current UI Activation (`src/ui/window.rs` lines 414-427)**:
   `Protocol::Vnc` currently has a placeholder comment in `on_connect`:
   `// VNC launch placeholder for M2 / handled in M3`.

---

## 2. Logic Chain

### 2.1 Dynamic `Picture` Paintable Update without UI Stutter or Memory Leaks
To render incoming `VncFrameUpdate` byte buffers inside GTK4 dynamically:

1. **Zero-Copy Memory Packaging**:
   Instead of copying pixel vectors, construct `glib::Bytes` using `glib::Bytes::from_owned(frame.pixels)`. This transfers ownership of `Vec<u8>` into GLib's ref-counted container without re-allocating heap memory.
2. **Texture Creation**:
   Construct `gdk::MemoryTexture` via:
   `gdk::MemoryTexture::with_format(&bytes, frame.width as i32, frame.height as i32, gdk::MemoryFormat::B8g8r8x8, frame.stride)`
3. **Updating GTK `Picture`**:
   Assign the texture to the `gtk4::Picture` widget via `picture.set_paintable(Some(&texture))`.
4. **Stutter Prevention & Main Thread Dispatch**:
   GTK widgets must only be mutated on the GTK Main Context (UI thread). Background network tasks (Tokio tasks receiving RFB pixel updates) must send updates across a channel (`glib::MainContext::channel` or `tokio::sync::mpsc`).
   To prevent channel queue bloat during high frame-rate updates, use a single-element frame slot or dropping strategy (`watch` channel or `Arc<Mutex<Option<VncFrameUpdate>>>` processed via `glib::idle_add_local`), ensuring GTK only renders the latest frame when ready and drops stale intermediate frames.
5. **Memory Leak Prevention**:
   Replacing `picture.set_paintable(Some(&new_texture))` drops the reference to the previous `gdk::MemoryTexture`, freeing the underlying `glib::Bytes` automatically.

### 2.2 VNC Display Scaling Architecture
GTK4 `gtk4::Picture` supports content fit and shrink configuration:

| `VncScaling` Mode | `picture.set_can_shrink(...)` | `picture.set_content_fit(...)` | `ScrolledWindow` Policy | Behavior |
|-------------------|-------------------------------|--------------------------------|-------------------------|----------|
| `OriginalSize` | `false` | `gtk::ContentFit::Contain` | `PolicyType::Automatic` | Keeps native pixel resolution. If smaller than container, scrollbars appear. |
| `FitToWindow` | `true` | `gtk::ContentFit::Contain` | `PolicyType::Never` / `Automatic` | Scales frame to fit container maintaining aspect ratio. No unwanted scrollbars. |
| `Stretch` | `true` | `gtk::ContentFit::Fill` | `PolicyType::Never` / `Automatic` | Stretches frame to fill available space completely, ignoring aspect ratio. |

### 2.3 Mouse Pointer Coordinate Translation
When user clicks or moves mouse on `gtk4::Picture`, GTK gives widget relative coordinates `(x, y)`. These must be mapped to remote framebuffer coordinates `(frame_x, frame_y)`:

- **OriginalSize**:
  `frame_x = x as u16`, `frame_y = y as u16` (clamped to `[0, frame_width]`).
- **Stretch**:
  `frame_x = ((x / widget_width) * frame_width) as u16`,
  `frame_y = ((y / widget_height) * frame_height) as u16`.
- **FitToWindow (Contain)**:
  Compute scaling factor `s = min(widget_width / frame_width, widget_height / frame_height)`.
  Compute letterbox/pillarbox padding:
  `offset_x = (widget_width - frame_width * s) / 2.0`
  `offset_y = (widget_height - frame_height * s) / 2.0`
  `frame_x = (((x - offset_x) / s).clamp(0.0, frame_width as f64)) as u16`
  `frame_y = (((y - offset_y) / s).clamp(0.0, frame_height as f64)) as u16`

### 2.4 Keyboard Input Mapping
- Attach `gtk::EventControllerKey` to the container widget (`picture.set_focusable(true)`).
- `connect_key_pressed`: Convert `gdk::Key` keyval to X11 keysym `u32`. Dispatch `VncEvent::Key { keysym, down: true }`.
- `connect_key_released`: Convert `gdk::Key` keyval to X11 keysym `u32`. Dispatch `VncEvent::Key { keysym, down: false }`.

### 2.5 Container Integration into `src/ui/window.rs`
1. **Content Stack Expansion**:
   Add a new named stack child `"vnc_session"` to `content_stack` in `MainWindow`.
2. **VNC Toolbar & Container**:
   Build `vnc_container` containing a top action bar (`[Disconnect]`, `[Scaling Mode Combo]`, `[Send Ctrl+Alt+Del]`) and the `VncWidget` wrapped in `gtk::ScrolledWindow`.
3. **Activation Handler**:
   In `on_connect` when `conn.protocol == Protocol::Vnc`:
   - Retrieve password from keyring via `secrets::get_password_sync(&conn.id)`.
   - Instantiate `VncWidget`.
   - Establish async channels for `VncFrameUpdate` and `VncEvent`.
   - Spawn background Tokio task running `VncClient`.
   - Switch `content_stack` to `"vnc_session"`.
   - On Disconnect click or connection error, terminate VNC client, clear widget, and switch `content_stack` back to `"editor"`.

---

## 3. Caveats

1. **Display Server Dependency**: Full GTK rendering tests require a running display server (X11 / Wayland / XVFB). Pure unit tests test state and coordinate transformations without requiring display hardware.
2. **Backwards Compatibility**: The updated `VncWidget` must retain fields (`scaling`, `current_frame`, `events_sent`) and existing public methods (`render_frame`, `send_key_event`, `send_pointer_event`, `set_scaling`) so all existing unit tests in `tests/e2e_vnc_tests.rs` continue to pass 100%.

---

## 4. Conclusion & Implementation Blueprint

### 4.1 Implementation Blueprint for `src/vnc/widget.rs`

Below is the concrete implementation blueprint for `src/vnc/widget.rs`:

```rust
use std::cell::RefCell;
use std::rc::Rc;
use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;

use crate::models::VncScaling;
use crate::vnc::client::{VncEvent, VncFrameUpdate};

/// GTK4 VNC Framebuffer Rendering Widget.
pub struct VncWidget {
    pub scaling: VncScaling,
    pub current_frame: Option<VncFrameUpdate>,
    pub events_sent: Vec<VncEvent>,
    picture: gtk::Picture,
    container: gtk::ScrolledWindow,
}

impl VncWidget {
    pub fn new(scaling: VncScaling) -> Self {
        let picture = gtk::Picture::builder()
            .can_shrink(false)
            .content_fit(gtk::ContentFit::Contain)
            .focusable(true)
            .build();

        let container = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Automatic)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .child(&picture)
            .build();

        let mut widget = Self {
            scaling,
            current_frame: None,
            events_sent: Vec::new(),
            picture,
            container,
        };

        widget.apply_scaling(scaling);
        widget
    }

    /// Return reference to the top-level GTK container widget.
    pub fn widget(&self) -> &gtk::ScrolledWindow {
        &self.container
    }

    /// Return reference to the inner GTK Picture widget.
    pub fn picture(&self) -> &gtk::Picture {
        &self.picture
    }

    /// Update picture paintable from incoming frame update.
    pub fn render_frame(&mut self, frame: VncFrameUpdate) {
        let bytes = glib::Bytes::from_owned(frame.pixels.clone());
        let texture = gdk::MemoryTexture::with_format(
            &bytes,
            frame.width as i32,
            frame.height as i32,
            gdk::MemoryFormat::B8g8r8x8,
            frame.stride,
        );

        self.picture.set_paintable(Some(&texture));
        self.current_frame = Some(frame);
    }

    pub fn send_key_event(&mut self, keysym: u32, down: bool) {
        self.events_sent.push(VncEvent::Key { keysym, down });
    }

    pub fn send_pointer_event(&mut self, x: u16, y: u16, mask: u8) {
        self.events_sent.push(VncEvent::Pointer { x, y, mask });
    }

    pub fn set_scaling(&mut self, scaling: VncScaling) {
        self.scaling = scaling;
        self.apply_scaling(scaling);
    }

    fn apply_scaling(&self, scaling: VncScaling) {
        match scaling {
            VncScaling::OriginalSize => {
                self.picture.set_can_shrink(false);
                self.picture.set_content_fit(gtk::ContentFit::Contain);
                self.container.set_hscrollbar_policy(gtk::PolicyType::Automatic);
                self.container.set_vscrollbar_policy(gtk::PolicyType::Automatic);
            }
            VncScaling::FitToWindow => {
                self.picture.set_can_shrink(true);
                self.picture.set_content_fit(gtk::ContentFit::Contain);
                self.container.set_hscrollbar_policy(gtk::PolicyType::Never);
                self.container.set_vscrollbar_policy(gtk::PolicyType::Never);
            }
            VncScaling::Stretch => {
                self.picture.set_can_shrink(true);
                self.picture.set_content_fit(gtk::ContentFit::Fill);
                self.container.set_hscrollbar_policy(gtk::PolicyType::Never);
                self.container.set_vscrollbar_policy(gtk::PolicyType::Never);
            }
        }
    }

    /// Calculate remote framebuffer (x, y) coordinates from widget local (x, y).
    pub fn translate_coordinates(&self, local_x: f64, local_y: f64) -> (u16, u16) {
        let frame = match &self.current_frame {
            Some(f) => f,
            None => return (local_x.max(0.0) as u16, local_y.max(0.0) as u16),
        };

        let fw = frame.width as f64;
        let fh = frame.height as f64;
        let ww = self.picture.width() as f64;
        let wh = self.picture.height() as f64;

        if ww <= 0.0 || wh <= 0.0 {
            return (local_x.max(0.0) as u16, local_y.max(0.0) as u16);
        }

        match self.scaling {
            VncScaling::OriginalSize => (
                local_x.clamp(0.0, fw - 1.0) as u16,
                local_y.clamp(0.0, fh - 1.0) as u16,
            ),
            VncScaling::Stretch => (
                ((local_x / ww) * fw).clamp(0.0, fw - 1.0) as u16,
                ((local_y / wh) * fh).clamp(0.0, fh - 1.0) as u16,
            ),
            VncScaling::FitToWindow => {
                let scale = (ww / fw).min(wh / fh);
                let offset_x = (ww - fw * scale) / 2.0;
                let offset_y = (wh - fh * scale) / 2.0;

                let fx = ((local_x - offset_x) / scale).clamp(0.0, fw - 1.0);
                let fy = ((local_y - offset_y) / scale).clamp(0.0, fh - 1.0);
                (fx as u16, fy as u16)
            }
        }
    }

    /// Wire GTK EventControllers for Keyboard and Mouse Input Propagation.
    pub fn setup_event_controllers<FEvent>(&self, event_callback: FEvent)
    where
        FEvent: Fn(VncEvent) + 'static + Clone,
    {
        // 1. Keyboard Controller
        let key_controller = gtk::EventControllerKey::new();
        let cb_press = event_callback.clone();
        key_controller.connect_key_pressed(move |_, keyval, _, _| {
            let keysym = keyval.into_glib();
            cb_press(VncEvent::Key { keysym, down: true });
            glib::Propagation::Stop
        });

        let cb_release = event_callback.clone();
        key_controller.connect_key_released(move |_, keyval, _, _| {
            let keysym = keyval.into_glib();
            cb_release(VncEvent::Key { keysym, down: false });
        });

        self.picture.add_controller(key_controller);

        // 2. Motion Controller
        let motion_controller = gtk::EventControllerMotion::new();
        let cb_motion = event_callback.clone();
        let widget_ref = self.picture.clone();
        let scaling_mode = self.scaling;
        let current_frame_opt = self.current_frame.clone();

        motion_controller.connect_motion(move |_, x, y| {
            if let Some(ref frame) = current_frame_opt {
                let (fx, fy) = translate_coords_static(x, y, widget_ref.width() as f64, widget_ref.height() as f64, frame.width, frame.height, scaling_mode);
                cb_motion(VncEvent::Pointer { x: fx, y: fy, mask: 0 });
            }
        });

        self.picture.add_controller(motion_controller);
    }
}

fn translate_coords_static(lx: f64, ly: f64, ww: f64, wh: f64, fw_u: u32, fh_u: u32, scaling: VncScaling) -> (u16, u16) {
    let fw = fw_u as f64;
    let fh = fh_u as f64;
    if ww <= 0.0 || wh <= 0.0 {
        return (lx.max(0.0) as u16, ly.max(0.0) as u16);
    }
    match scaling {
        VncScaling::OriginalSize => (lx.clamp(0.0, fw - 1.0) as u16, ly.clamp(0.0, fh - 1.0) as u16),
        VncScaling::Stretch => (((lx / ww) * fw).clamp(0.0, fw - 1.0) as u16, ((ly / wh) * fh).clamp(0.0, fh - 1.0) as u16),
        VncScaling::FitToWindow => {
            let scale = (ww / fw).min(wh / fh);
            let off_x = (ww - fw * scale) / 2.0;
            let off_y = (wh - fh * scale) / 2.0;
            let fx = ((lx - off_x) / scale).clamp(0.0, fw - 1.0);
            let fy = ((ly - off_y) / scale).clamp(0.0, fh - 1.0);
            (fx as u16, fy as u16)
        }
    }
}
```

---

## 5. Verification Method

1. **Compile & Unit Test Verification**:
   ```bash
   cargo check --tests
   cargo test --test e2e_vnc_tests
   cargo test
   ```
2. **Invalidation Conditions**:
   - `gdk::MemoryFormat::B8g8r8x8` mismatches client frame pixel ordering.
   - `picture.set_can_shrink(false)` is omitted for `OriginalSize` mode.
   - Coordinate translation does not subtract letterbox offsets in `FitToWindow` mode.
