# VNC Keyboard and Mouse Input Event Propagation Design & Report

## 1. Observation

Direct observations from existing codebase files:

### Existing Rust VNC Types & Contracts
- **`src/vnc/client.rs`** (lines 11–15, 17–21):
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq)]
  pub enum VncEvent {
      Key { keysym: u32, down: bool },
      Pointer { x: u16, y: u16, mask: u8 },
  }

  pub struct VncClient {
      pub host: String,
      pub port: u16,
      pub scaling: VncScaling,
  }
  ```
- **`src/vnc/widget.rs`** (lines 4–8, 23–29):
  ```rust
  pub struct VncWidget {
      pub scaling: VncScaling,
      pub current_frame: Option<VncFrameUpdate>,
      pub events_sent: Vec<VncEvent>,
  }

  impl VncWidget {
      pub fn send_key_event(&mut self, keysym: u32, down: bool) {
          self.events_sent.push(VncEvent::Key { keysym, down });
      }

      pub fn send_pointer_event(&mut self, x: u16, y: u16, mask: u8) {
          self.events_sent.push(VncEvent::Pointer { x, y, mask });
      }
  }
  ```

### Existing Python Implementation (`src/ui/vnc_widget.py`)
- **Event controller setup** (lines 76–90):
  ```python
  self.key_ctrl = Gtk.EventControllerKey()
  self.key_ctrl.connect("key-pressed", self._on_key_pressed)
  self.key_ctrl.connect("key-released", self._on_key_released)
  self.add_controller(self.key_ctrl)

  self.motion_ctrl = Gtk.EventControllerMotion()
  self.motion_ctrl.connect("motion", self._on_motion)
  self.add_controller(self.motion_ctrl)

  self.click_ctrl = Gtk.GestureClick()
  self.click_ctrl.connect("pressed", self._on_click_pressed)
  self.click_ctrl.connect("released", self._on_click_released)
  self.add_controller(self.click_ctrl)
  ```
- **Coordinate translation** (lines 129–158):
  ```python
  def _map_coords(self, x, y):
      if not self._fb_w or not self._fb_h:
          return int(x), int(y)
      widget_w = self.get_width()
      widget_h = self.get_height()
      if widget_w == 0 or widget_h == 0:
          return int(x), int(y)
      img_aspect = self._fb_w / self._fb_h
      widget_aspect = widget_w / widget_h
      if widget_aspect > img_aspect:
          draw_h = widget_h
          draw_w = widget_h * img_aspect
          offset_x = (widget_w - draw_w) / 2
          offset_y = 0
      else:
          draw_w = widget_w
          draw_h = widget_w / img_aspect
          offset_x = 0
          offset_y = (widget_h - draw_h) / 2
      img_x = (x - offset_x) * (self._fb_w / draw_w)
      img_y = (y - offset_y) * (self._fb_h / draw_h)
      img_x = max(0, min(self._fb_w - 1, img_x))
      img_y = max(0, min(self._fb_h - 1, img_y))
      return int(img_x), int(img_y)
  ```
- **Pointer button masks** (lines 177–196):
  - Left click (button 1): `mask |= 1` (pressed), `mask &= ~1` (released)
  - Middle click (button 2): `mask |= 2` (pressed), `mask &= ~2` (released)
  - Right click (button 3): `mask |= 4` (pressed), `mask &= ~4` (released)

### Existing Test Contract (`tests/e2e_vnc_tests.rs`)
- **Key event verification** (lines 56–57, 63–70):
  - `widget.send_key_event(0xFF0D, true);` (Enter key down)
  - `widget.send_key_event(0xFF0D, false);` (Enter key up)
  - Asserts `VncEvent::Key { keysym: 0xFF0D, down: true / false }`
- **Pointer event verification** (lines 60, 71–74):
  - `widget.send_pointer_event(100, 200, 1);`
  - Asserts `VncEvent::Pointer { x: 100, y: 200, mask: 1 }`

---

## 2. Logic Chain

From the observed code and requirements, we derive the full technical specification for VNC input propagation:

### Step 1: GTK4 Event Controller Integration
1. GTK4 separates input handling into discrete `gtk::EventController` subclasses attached to widgets via `widget.add_controller(...)`.
2. For keyboard input:
   - `gtk::EventControllerKey` captures `key-pressed` and `key-released` signals.
   - The GTK widget must set `widget.set_focusable(true)`.
   - On mouse click, `widget.grab_focus()` is called to ensure keypresses are directed to the VNC session.
3. For mouse motion:
   - `gtk::EventControllerMotion` captures `motion(x, y)` signals where $(x, y)$ are widget-relative floating point coordinates (`f64`).
4. For mouse buttons:
   - `gtk::GestureClick` with `set_button(0)` listens to all mouse buttons (left, middle, right).
   - Captures `pressed(n_press, x, y)` and `released(n_press, x, y)` signals.

### Step 2: GDK Keyval to RFB Keysym Mapping
1. The RFB protocol specification (RFC 6143 §7.5.4) transmits keyboard events as 32-bit X11 keysym values.
2. In GTK4 (`gdk4` crate), `gdk::Key` values directly align with X11 keysym definitions for printable characters and standard navigation/modifier keys.
3. Mapping table design:
   - **Enter / Return**: `gdk::Key::Return` / `gdk::Key::KP_Enter` $\to$ `0xFF0D`
   - **BackSpace**: `gdk::Key::BackSpace` $\to$ `0xFF08`
   - **Tab**: `gdk::Key::Tab` / `gdk::Key::ISO_Left_Tab` $\to$ `0xFF09`
   - **Escape**: `gdk::Key::Escape` $\to$ `0xFF1B`
   - **Delete**: `gdk::Key::Delete` $\to$ `0xFFFF`
   - **Arrow Keys**: `Left` (`0xFF51`), `Up` (`0xFF52`), `Right` (`0xFF53`), `Down` (`0xFF54`)
   - **Page Controls**: `Home` (`0xFF50`), `Page_Up` (`0xFF55`), `Page_Down` (`0xFF56`), `End` (`0xFF57`), `Insert` (`0xFF63`)
   - **Modifiers**: `Shift_L` (`0xFFE1`), `Shift_R` (`0xFFE2`), `Control_L` (`0xFFE3`), `Control_R` (`0xFFE4`), `Alt_L` / `Meta_L` (`0xFFE9`), `Alt_R` / `Meta_R` (`0xFFEA`), `Super_L` (`0xFFEB`), `Super_R` (`0xFFEC`)
   - **Alphanumeric & ASCII**: GDK `keyval.into_glib()` directly matches X11/RFB keysyms for $0x0020 \dots 0x007E$.
   - **Unicode Fallback**: If `keyval.to_unicode()` yields character $c$, maps to `0x01000000 | (c as u32)`.

### Step 3: Mouse Coordinate Translation & Button Mask Generation
1. **Coordinate Translation**:
   - Given widget size $(W_{w}, H_{w})$ and remote framebuffer size $(W_{fb}, H_{fb})$:
   - **Mode: `VncScaling::FitToWindow` (Aspect Ratio Preserved)**:
     $$\text{Aspect}_{fb} = \frac{W_{fb}}{H_{fb}}, \quad \text{Aspect}_{w} = \frac{W_{w}}{H_{w}}$$
     - If $\text{Aspect}_{w} > \text{Aspect}_{fb}$ (pillarboxed):
       $$\text{draw}_{h} = H_{w}, \quad \text{draw}_{w} = H_{w} \cdot \text{Aspect}_{fb}$$
       $$\text{offset}_{x} = \frac{W_{w} - \text{draw}_{w}}{2}, \quad \text{offset}_{y} = 0$$
     - Else (letterboxed):
       $$\text{draw}_{w} = W_{w}, \quad \text{draw}_{h} = \frac{W_{w}}{\text{Aspect}_{fb}}$$
       $$\text{offset}_{x} = 0, \quad \text{offset}_{y} = \frac{H_{w} - \text{draw}_{h}}{2}$$
     - Local coordinate mapping:
       $$X_{remote} = \text{clamp}\left(\left\lfloor (x - \text{offset}_{x}) \cdot \frac{W_{fb}}{\text{draw}_{w}} \right\rfloor, 0, W_{fb} - 1\right)$$
       $$Y_{remote} = \text{clamp}\left(\left\lfloor (y - \text{offset}_{y}) \cdot \frac{H_{fb}}{\text{draw}_{h}} \right\rfloor, 0, H_{fb} - 1\right)$$
   - **Mode: `VncScaling::Stretch`**:
     $$X_{remote} = \text{clamp}\left(\left\lfloor x \cdot \frac{W_{fb}}{W_{w}} \right\rfloor, 0, W_{fb} - 1\right), \quad Y_{remote} = \text{clamp}\left(\left\lfloor y \cdot \frac{H_{fb}}{H_{w}} \right\rfloor, 0, H_{fb} - 1\right)$$
   - **Mode: `VncScaling::OriginalSize`**:
     $$X_{remote} = \text{clamp}\left(\lfloor x \rfloor, 0, W_{fb} - 1\right), \quad Y_{remote} = \text{clamp}\left(\lfloor y \rfloor, 0, H_{fb} - 1\right)$$

2. **Button Mask Generation**:
   - RFB Bitfield spec: Bit 0 (`0x01`) = Left, Bit 1 (`0x02`) = Middle, Bit 2 (`0x04`) = Right.
   - Pressed event: `mask |= bit_for_button`
   - Released event: `mask &= !bit_for_button`

### Step 4: Message Transmission Channel Architecture
1. The GTK main loop runs on the primary thread and must never perform blocking async network calls.
2. An unbounded Tokio channel (`tokio::sync::mpsc::unbounded_channel::<VncEvent>()`) transfers events asynchronously from GTK callbacks to the background Tokio RFB client loop.
3. In `VncWidget`:
   - Holds `tx: mpsc::UnboundedSender<VncEvent>`.
   - On `send_key_event` and `send_pointer_event`, pushes to `events_sent` (for test inspection) AND sends via `tx.send(...)`.
4. In `VncClient` background loop:
   - Receives events via `rx.recv().await` and invokes RFB client send methods (`client.send_key_event(keysym, down).await`, `client.send_pointer_event(x, y, mask).await`).

---

## 3. Caveats

1. **Non-US Keyboard Layouts**: GDK keyvals depend on the client host layout. The mapped RFB keysyms assume X11 keysym standard semantics.
2. **Scroll Wheel & Extra Buttons**: Standard RFB supports scroll wheel up (`0x08`) and down (`0x10`). `gtk::EventControllerScroll` can be added as a minor extension if desired.
3. **Read-Only Scope**: This report provides the complete architecture and blueprint. No source files were modified during this investigation turn.

---

## 4. Conclusion

VNC input handling for `beautiful-goodall` can be implemented via a clean `events.rs` module and GTK controller wiring in `widget.rs`:
1. `gtk::EventControllerKey`, `gtk::EventControllerMotion`, and `gtk::GestureClick` manage focus and user input callbacks.
2. `gdk_key_to_rfb_keysym` translates `gdk::Key` to 32-bit RFB keysyms with full coverage for special, modifier, and alphanumeric keys.
3. `translate_coordinates` converts GTK local float coordinates to remote RFB pixel coordinates $[0, W_{fb}-1] \times [0, H_{fb}-1]$ taking scaling mode into account.
4. An async Tokio `mpsc` channel provides lock-free, zero-latency event propagation to the RFB protocol thread.

---

## 5. Verification Method

To verify the implementation once integrated:
1. Run unit and E2E VNC tests:
   ```bash
   cargo test --test e2e_vnc_tests
   ```
2. Verify test output confirms:
   - `test_vnc_widget_render_frame_and_events` passes.
   - Key down/up events for `0xFF0D` (Enter) are correctly formatted as `VncEvent::Key`.
   - Pointer events with $(x=100, y=200, \text{mask}=1)$ are correctly formatted as `VncEvent::Pointer`.
3. Invalidation conditions:
   - Any keyval conversion producing `0` for valid keys.
   - Out-of-bounds mouse coordinates ($X \ge W_{fb}$ or $Y \ge H_{fb}$).
