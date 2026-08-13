# Technical Analysis: VNC, C Extension, RDP, SSH, and Rust Replacement Roadmap

## Executive Summary
This document provides a comprehensive technical investigation of the remote connection mechanisms in `beautiful-goodall` (VER - Very Easy Remote). It details the existing C extension for VNC, GTK4 rendering architecture, input event handling, RDP (`xfreerdp3`) and SSH subprocess management, and presents a complete architectural plan for porting these components to pure Rust using `gtk4-rs`, `libadwaita`, `vnc` (vnc-rs v0.4.0), and `std::process::Command`.

---

## 1. Existing C Extension for VNC (`src/core/ext/vnc_ext.c`)

### 1.1 Overview & Dependencies
The C extension bridges Python (via `ctypes`) to `libvncclient` (part of LibVNCServer/LibVNCClient).
- Header: `#include <rfb/rfbclient.h>`
- Threading: Posix threads (`pthread.h`)
- Memory: Standard heap allocation (`malloc`, `calloc`, `strdup`, `free`)

### 1.2 Data Structures
```c
typedef void (*framebuffer_cb_t)(int w, int h, int stride, const unsigned char* pixels);

typedef struct {
    rfbClient* client;
    pthread_t thread;
    int is_running;
    char* password;
    framebuffer_cb_t cb;
} VncContext;
```
- `VncContext`: Encapsulates libvncclient instance, worker thread handle, loop control flag, credentials, and framebuffer callback function pointer.
- `client_data_tag`: Static integer pointer tag (`static int client_data_tag = 0`) used with `rfbClientSetClientData` / `rfbClientGetClientData` to associate `VncContext` with the `rfbClient` instance.

### 1.3 RFB Connection & Setup (`vnc_connect`)
1. `rfbGetClient(8, 3, 4)` initializes an `rfbClient` expecting 8 bits per sample, 3 samples per pixel (RGB), and 4 bytes per pixel (32 bpp).
2. Explicit format setup:
   - `depth = 24`, `bitsPerPixel = 32`, `trueColour = 1`
   - `redShift = 16`, `greenShift = 8`, `blueShift = 0`
   - `redMax = 255`, `greenMax = 255`, `blueMax = 255`
   - On little-endian x86_64 systems, `redShift=16, greenShift=8, blueShift=0` produces pixel memory arranged as `B G R x` (BGRx8888).
3. Preferred encodings: `cl->appData.encodingsString = "tight zrle hextile raw"`.
   - `libvncclient` handles Tight and ZRLE decompression internally in C.
4. Callbacks:
   - `cl->GetPassword = get_password`: returns `strdup(ctx->password)` when requested by RFB authentication.
   - `cl->GotFrameBufferUpdate = update_framebuffer`: called by libvncclient after receiving RFB updates.
     ```c
     static void update_framebuffer(rfbClient* client, int x, int y, int w, int h) {
         VncContext* ctx = (VncContext*)rfbClientGetClientData(client, &client_data_tag);
         if (ctx && ctx->cb && client->frameBuffer) {
             int stride = client->width * (client->format.bitsPerPixel / 8);
             ctx->cb(client->width, client->height, stride, client->frameBuffer);
         }
     }
     ```
     Note: The C code passes the full `client->frameBuffer` pointer to Python on every update rectangle.

### 1.4 Background Worker Thread (`vnc_thread`)
1. Calls `rfbInitClient(ctx->client, NULL, NULL)` to negotiate RFB handshake (version 3.3/3.7/3.8), security type, and VNC authentication.
2. Sets `ctx->is_running = 1`.
3. Loop:
   ```c
   while (ctx->is_running) {
       int i = WaitForMessage(ctx->client, 50000); // 50ms timeout
       if (i < 0) break; // Error / disconnect
       if (i > 0) {
           if (!HandleRFBServerMessage(ctx->client)) break;
       }
   }
   ```
4. On exit, sets `ctx->is_running = 0`.

### 1.5 Disconnection (`vnc_disconnect`)
- Sets `ctx->is_running = 0`.
- Joins worker thread `pthread_join(ctx->thread, NULL)`.
- Frees password string, calls `rfbClientCleanup(ctx->client)`, and frees `ctx`.

### 1.6 Input Functions
- `vnc_send_key(ctx, keysym, down)`: Invokes `SendKeyEvent(ctx->client, keysym, down)`.
  - `keysym`: X11 key symbol code.
  - `down`: `1` for key press, `0` for key release.
- `vnc_send_pointer(ctx, x, y, button_mask)`: Invokes `SendPointerEvent(ctx->client, x, y, button_mask)`.
  - `x`, `y`: Absolute integer pixel coordinates in remote framebuffer.
  - `button_mask`: Bitmask of pressed buttons (bit 0 = Left / 0x01, bit 1 = Middle / 0x02, bit 2 = Right / 0x04).

---

## 2. Python VNC Widget & GTK Rendering (`src/ui/vnc_widget.py`)

### 2.1 Widget Class & Initialization
- Subclasses `Gtk.Picture`.
- Sets `set_focusable(True)` to receive keyboard focus.
- Connects to C extension via `ctypes` (`vnc_lib.vnc_connect`). Passes Python C-callback `_on_framebuffer_update`.

### 2.2 Rendering Pipeline
1. C callback updates state:
   - Sets `_fb_w`, `_fb_h`, `_fb_stride`, `_fb_ptr`.
   - Sets `_dirty = True`.
2. GLib Timer: `GLib.timeout_add(16, self._render_frame)` (~60 FPS).
3. Frame generation:
   - Reads buffer: `ctypes.string_at(self._fb_ptr, self._fb_stride * self._fb_h)`.
   - Wraps in `GLib.Bytes.new(buffer)`.
   - Creates `Gdk.MemoryTexture.new(w, h, Gdk.MemoryFormat.B8G8R8X8, bytes_glib, stride)`.
   - Displays frame: `self.set_paintable(texture)`.

### 2.3 Aspect Ratio & Coordinate Mapping (`_map_coords(x, y)`)
`Gtk.Picture` maintains image aspect ratio. To accurately map mouse events from widget space to remote framebuffer pixels:
1. Calculates `img_aspect = fb_w / fb_h` and `widget_aspect = widget_w / widget_h`.
2. Calculates scale and letterbox/pillarbox offset:
   - If `widget_aspect > img_aspect` (Pillarbox - black bars on sides):
     `draw_h = widget_h`, `draw_w = widget_h * img_aspect`, `offset_x = (widget_w - draw_w) / 2`, `offset_y = 0`.
   - Else (Letterbox - black bars top/bottom):
     `draw_w = widget_w`, `draw_h = widget_w / img_aspect`, `offset_x = 0`, `offset_y = (widget_h - draw_h) / 2`.
3. Transform widget event coordinates `(x, y)`:
   - `img_x = (x - offset_x) * (fb_w / draw_w)`
   - `img_y = (y - offset_y) * (fb_h / draw_h)`
4. Clamps `img_x` to `[0, fb_w - 1]` and `img_y` to `[0, fb_h - 1]`.

### 2.4 Event Controllers & Handlers
- `Gtk.EventControllerKey`:
  - `key-pressed`: sends `vnc_send_key(ctx, keyval, 1)`.
  - `key-released`: sends `vnc_send_key(ctx, keyval, 0)`.
- `Gtk.EventControllerMotion`:
  - `motion`: converts `(x, y)` using `_map_coords`, sends `vnc_send_pointer(ctx, mapped_x, mapped_y, button_mask)`.
- `Gtk.GestureClick`:
  - `pressed`: grabs focus (`self.grab_focus()`), updates button mask (`btn 1 -> |= 1`, `btn 2 -> |= 2`, `btn 3 -> |= 4`), maps coords, sends `vnc_send_pointer`.
  - `released`: updates button mask (`btn 1 -> &= ~1`, etc.), maps coords, sends `vnc_send_pointer`.

---

## 3. RDP and SSH Integration Mechanics

### 3.1 RDP Integration (`src/core/launcher.py` & `src/core/rdp_client.py`)
- Executable Discovery: Checks system PATH for `xfreerdp3`, fallback to `xfreerdp` or `wlfreerdp` using `shutil.which`.
- CLI Argument Construction:
  - Host/Port: `/v:<host>:<port>`
  - Username: `/u:<username>` (if configured)
  - Domain: `/d:<domain>` (in `rdp_client.py`)
  - Password: `/p:<password>` (retrieved from secrets store)
  - Certificate handling: `/cert:ignore`
  - Resolution: `/dynamic-resolution`
  - Clipboard sharing: `+clipboard` (if enabled) or `-clipboard`
  - Color depth: `/bpp:<depth>` (if > 0)
  - Multi-monitor: `/multimon` (if enabled)
  - Fullscreen: `/f` (if enabled)
  - Audio redirection: `/sound` (or `+sound`)
  - Parent Window Embedding: `/parent-window:<embed_xid>` (in `rdp_client.py` for embedding into GTK/X11 socket)
- Child Process Lifecycle Management:
  - `subprocess.Popen(args, stdin=DEVNULL, stdout=DEVNULL, stderr=DEVNULL)`
  - Standard streams are set to `DEVNULL` so the child process detaches from the parent shell and continues running even if VER GUI closes.

### 3.2 SSH Integration (`src/core/launcher.py` & `src/ui/terminal.py`)
- External Terminal Launcher (`launcher.py`):
  - Constructs SSH base command: `["ssh", "-p", "<port>", "<user>@<host>"]` (or omitting `-p` if port == 22).
  - Terminal Emulator Fallback Search Order:
    1. `ptyxis` (`ptyxis -- <ssh_cmd>`)
    2. `kgx` (`kgx -e <ssh_cmd>`)
    3. `gnome-terminal` (`gnome-terminal -- <ssh_cmd>`)
    4. `konsole` (`konsole -e <ssh_cmd>`)
    5. `xfce4-terminal` (`xfce4-terminal -e <ssh_cmd>`)
    6. `kitty` (`kitty <ssh_cmd>`)
    7. `alacritty` (`alacritty -e <ssh_cmd>`)
    8. `xterm` (`xterm -e <ssh_cmd>`)
  - Spawned via `subprocess.Popen` with `DEVNULL` streams.
- Embedded Terminal Widget (`src/ui/terminal.py`):
  - Uses `Vte.Terminal()` widget.
  - Spawns async shell process via `Vte.Terminal.spawn_async`.

---

## 4. Rust Replacement Requirements & Architecture Plan

### 4.1 Cargo Dependencies (`Cargo.toml`)
The project `Cargo.toml` specifies:
```toml
[dependencies]
gtk = { package = "gtk4", version = "0.7" }
libadwaita = "0.5"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
vnc = "0.4.0"
anyhow = "1.0"
oo7 = "0.3" 
tokio = { version = "1.34", features = ["full"] }
```

### 4.2 Pure Rust VNC Client (`vnc` crate)
- Protocol Handling: The `vnc` crate (v0.4.0) provides native RFB protocol client capability.
- Connection: `vnc::client::Connector::new()` connects to TCP socket `host:port`, executes RFB handshake, and negotiates authentication (Password / None).
- Framebuffer Updates:
  - Running event loop on background thread / tokio task.
  - Receives frame updates: width, height, pixel buffer (`Vec<u8>`).
- Encodings: `vnc` crate supports Raw, ZRLE, Tight, Hextile encodings out of the box.
- Thread Communication: Use `glib::MainContext::channel(glib::Priority::DEFAULT)` to send updated framebuffer pixel buffers safely from background VNC task to GTK GUI main thread.

### 4.3 GTK4 Rendering (`gtk4::Picture` + `gdk::MemoryTexture`)
- Widget: Subclass or wrap `gtk4::Picture`.
- Setting Texture:
  ```rust
  let bytes = glib::Bytes::from_owned(pixel_data);
  let texture = gdk::MemoryTexture::new(
      width as i32,
      height as i32,
      gdk::MemoryFormat::B8g8r8x8, // or R8g8b8a8 depending on vnc format
      &bytes,
      stride as usize,
  );
  picture.set_paintable(Some(&texture));
  ```
- Performance: `gdk::MemoryTexture` passes pixel data directly to GTK4's GSK render node (NGL/Vulkan compositor), delivering high-performance GPU-accelerated rendering.

### 4.4 Mouse and Keyboard Event Mapping (`gtk4-rs`)
- Keyboard:
  ```rust
  let key_ctrl = gtk4::EventControllerKey::new();
  key_ctrl.connect_key_pressed(glib::clone!(@strong vnc_sender => move |_, keyval, _keycode, _state| {
      let keysym = keyval.into_glib();
      vnc_sender.send(VncEvent::KeyEvent { keysym, down: true }).ok();
      glib::Propagation::Stop
  }));
  key_ctrl.connect_key_released(glib::clone!(@strong vnc_sender => move |_, keyval, _keycode, _state| {
      let keysym = keyval.into_glib();
      vnc_sender.send(VncEvent::KeyEvent { keysym, down: false }).ok();
      glib::Propagation::Stop
  }));
  widget.add_controller(key_ctrl);
  ```
- Motion & Pointer:
  ```rust
  let motion_ctrl = gtk4::EventControllerMotion::new();
  motion_ctrl.connect_motion(glib::clone!(@strong vnc_sender => move |_, x, y| {
      let (fb_x, fb_y) = map_coords(x, y, widget_w, widget_h, fb_w, fb_h);
      vnc_sender.send(VncEvent::PointerEvent { x: fb_x, y: fb_y, mask: current_mask }).ok();
  }));
  widget.add_controller(motion_ctrl);
  ```
- Mouse Click:
  ```rust
  let click_ctrl = gtk4::GestureClick::new();
  click_ctrl.connect_pressed(glib::clone!(@strong widget, @strong vnc_sender => move |gesture, _n, x, y| {
      widget.grab_focus();
      let btn = gesture.current_button();
      let mask = update_button_mask(btn, true);
      let (fb_x, fb_y) = map_coords(x, y, ...);
      vnc_sender.send(VncEvent::PointerEvent { x: fb_x, y: fb_y, mask }).ok();
  }));
  ```

### 4.5 Subprocess Spawning (`std::process::Command`)
- RDP Launcher (`xfreerdp3`):
  ```rust
  use std::process::{Command, Stdio};

  pub fn launch_rdp(conn: &Connection, password: Option<&str>) -> anyhow::Result<()> {
      let bin = find_rdp_binary().ok_or_else(|| anyhow::anyhow!("FreeRDP binary not found"))?;
      let mut cmd = Command::new(bin);
      cmd.arg(format!("/v:{}:{}", conn.host, conn.port));
      if !conn.username.is_empty() {
          cmd.arg(format!("/u:{}", conn.username));
      }
      if let Some(pass) = password {
          cmd.arg(format!("/p:{}", pass));
      }
      cmd.args(&["/cert:ignore", "/dynamic-resolution"]);
      
      if conn.advanced_settings.clipboard_sharing {
          cmd.arg("+clipboard");
      } else {
          cmd.arg("-clipboard");
      }
      if conn.advanced_settings.rdp_multimon { cmd.arg("/multimon"); }
      if conn.advanced_settings.rdp_fullscreen { cmd.arg("/f"); }
      if conn.advanced_settings.rdp_audio { cmd.arg("/sound"); }

      cmd.stdin(Stdio::null())
         .stdout(Stdio::null())
         .stderr(Stdio::null())
         .spawn()?;
      Ok(())
  }
  ```
- SSH External Terminal Launcher:
  ```rust
  pub fn launch_ssh(conn: &Connection) -> anyhow::Result<()> {
      let mut ssh_args = vec![];
      if conn.port != 22 && conn.port != 0 {
          ssh_args.push("-p".to_string());
          ssh_args.push(conn.port.to_string());
      }
      let target = if conn.username.is_empty() {
          conn.host.clone()
      } else {
          format!("{}@{}", conn.username, conn.host)
      };
      ssh_args.push(target);

      let terminals = [
          ("ptyxis", vec!["--"]),
          ("kgx", vec!["-e"]),
          ("gnome-terminal", vec!["--"]),
          ("konsole", vec!["-e"]),
          ("xfce4-terminal", vec!["-e"]),
          ("kitty", vec![]),
          ("alacritty", vec!["-e"]),
          ("xterm", vec!["-e"]),
      ];

      for (term, prefix) in terminals {
          if which::which(term).is_ok() {
              let mut cmd = Command::new(term);
              cmd.args(prefix);
              cmd.arg("ssh");
              cmd.args(&ssh_args);
              cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
              cmd.spawn()?;
              return Ok(());
          }
      }

      // Direct fallback
      let mut cmd = Command::new("ssh");
      cmd.args(&ssh_args);
      cmd.spawn()?;
      Ok(())
  }
  ```

---

## 5. Summary Table & Architectural Comparison

| Component | Existing Python + C Implementation | Proposed Rust Implementation |
|---|---|---|
| **VNC Client Engine** | Custom C extension (`vnc_ext.c`) wrapping `libvncclient` | Pure Rust `vnc` crate (v0.4.0) |
| **VNC Threading** | POSIX pthread inside `vnc_ext.c` | Tokio task / OS thread with `glib::MainContext::channel` |
| **GTK Display** | `Gtk.Picture` + `Gdk.MemoryTexture` (B8G8R8X8) | `gtk4::Picture` + `gdk::MemoryTexture` (B8g8r8x8) |
| **Input Controllers** | `EventControllerKey`, `EventControllerMotion`, `GestureClick` | `gtk4::EventControllerKey`, `gtk4::EventControllerMotion`, `gtk4::GestureClick` |
| **Coordinate Mapping** | Custom Python ratio calculation (`_map_coords`) | Equivalent Rust letterbox/pillarbox math helper |
| **RDP Subprocess** | `subprocess.Popen` running `xfreerdp3` / `xfreerdp` | `std::process::Command` running `xfreerdp3` / `xfreerdp` |
| **SSH Subprocess** | `subprocess.Popen` with terminal emulator fallback list | `std::process::Command` with terminal emulator fallback list |
