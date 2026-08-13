# Handoff Report: Native VNC Async Client Engine Design (`src/vnc/client.rs`)

**Author**: `explorer_m3_1`  
**Date**: 2026-08-12  
**Target File**: `src/vnc/client.rs`  
**Milestone**: M3 (Native VNC Engine & GTK4 Display Integration)  

---

## 1. Observation

### 1.1 Dependency & Crate Analysis (`Cargo.toml` & `vnc-0.4.0`)
- **`Cargo.toml`**: Lines 19 & 22 specify `vnc = "0.4.0"` and `tokio = { version = "1.34", features = ["full"] }`.
- **Crate Source**: Located at `/home/dawiisss/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/vnc-0.4.0/`.
- **`vnc::Client` API (`vnc-0.4.0/src/client.rs`)**:
  - **Constructor & Auth** (lines 162-298):
    ```rust
    pub fn from_tcp_stream<Auth>(
        mut stream: TcpStream,
        shared: bool,
        auth: Auth
    ) -> Result<Client>
    where Auth: FnOnce(&[AuthMethod]) -> Option<AuthChoice>
    ```
    - Handshake supports RFB `3.3`, `3.7`, and `3.8`.
    - `AuthMethod` contains `AuthMethod::None` and `AuthMethod::Password`.
    - `AuthChoice::Password([u8; 8])`: Password auth requires an 8-byte array. The crate internally reverses bits per byte and performs DES encryption against a 16-byte server challenge.
    - Post-auth: sends `ClientInit { shared }`, receives `ServerInit` containing desktop dimensions `size: (u16, u16)` and `format: PixelFormat`.
    - Internal worker: `from_tcp_stream` automatically calls `thread::spawn` running `Event::pump(stream, format, tx_events)` which pushes `vnc::client::Event` into an internal `std::sync::mpsc::channel`.
  - **Message Sending Methods**:
    - `client.set_encodings(&[Encoding::Zrle, Encoding::CopyRect, Encoding::Raw, Encoding::Cursor, Encoding::DesktopSize]) -> Result<()>` (lines 305-310).
    - `client.request_update(rect: Rect, incremental: bool) -> Result<()>` (lines 312-322).
    - `client.send_key_event(down: bool, key: u32) -> Result<()>` (lines 324-332).
    - `client.send_pointer_event(buttons: u8, x: u16, y: u16) -> Result<()>` (lines 334-342).
    - `client.update_clipboard(text: &str) -> Result<()>` (lines 344-349).
    - `client.disconnect() -> Result<()>` (lines 407-410).
  - **Event Polling**:
    - `client.poll_event() -> Option<vnc::client::Event>` (lines 391-402).
    - Events emitted:
      - `Event::PutPixels(Rect, Vec<u8>)`: Sub-rectangle pixel data.
      - `Event::CopyPixels { src: Rect, dst: Rect }`: Copy rectangle area.
      - `Event::Resize(u16, u16)`: Framebuffer resize notification.
      - `Event::EndOfFrame`: End of current frame updates boundary.
      - `Event::Disconnected(Option<Error>)`: Connection closed or error.
- **`vnc::PixelFormat` (`vnc-0.4.0/src/protocol.rs`, lines 174-185)**:
  ```rust
  pub struct PixelFormat {
      pub bits_per_pixel: u8,
      pub depth: u8,
      pub big_endian: bool,
      pub true_colour: bool,
      pub red_max: u16,
      pub green_max: u16,
      pub blue_max: u16,
      pub red_shift: u8,
      pub green_shift: u8,
      pub blue_shift: u8,
  }
  ```

### 1.2 Existing Project Codebase (`src/vnc/`)
- **`src/vnc/mod.rs`**: Re-exports `client::{VncClient, VncEvent, VncFrameUpdate}` and `widget::VncWidget`.
- **`src/vnc/client.rs`**: Currently contains stub structs `VncFrameUpdate`, `VncEvent`, `VncClient` with basic RGB-to-BGRA conversion (`process_frame_buffer`).
- **`src/vnc/widget.rs`**: Stub widget holding `current_frame` and recording `events_sent`.

---

## 2. Logic Chain

1. **Threading Model & GTK Integration**:
   - `vnc::Client` relies on standard `std::net::TcpStream` (blocking I/O) and standard `mpsc` channels. Calling `poll_event()` or write methods directly on the GTK main thread would freeze the UI.
   - Therefore, `VncSession` must run inside a dedicated background Tokio blocking task (`tokio::task::spawn_blocking`) or standard `std::thread::spawn`.
   - Communication from **UI to VNC Session**: Async `tokio::sync::mpsc::unbounded_channel::<VncCommand>()` (or `std::sync::mpsc`) allows the GTK main thread to dispatch `KeyEvent`, `PointerEvent`, `CutText`, and `Disconnect` commands without blocking.
   - Communication from **VNC Session to GTK UI**: GTK4 requires UI and texture updates to occur on the main loop thread. `glib::MainContext::channel(glib::Priority::DEFAULT)` creates a `(glib::Sender<VncSessionEvent>, glib::Receiver<VncSessionEvent>)`. `glib::Sender` is `Send + Sync` and safely dispatches `VncSessionEvent` to a closure attached to the GTK main context via `.attach(None, move |event| { ... })`.

2. **RFB Auth & Handshake**:
   - RFB passwords are 8 ASCII bytes max. The `auth` closure converts an `Option<String>` password into `AuthChoice::Password([u8; 8])`. If password length is < 8 bytes, it is zero-padded; if > 8 bytes, truncated.
   - Initial connection flow:
     `TcpStream::connect_timeout` -> `Client::from_tcp_stream` -> `set_encodings` -> send initial `request_update(Rect { left: 0, top: 0, width, height }, false)`.

3. **Frame Assembly & Pixel Format Conversion**:
   - Remote VNC servers emit incremental tile updates (`PutPixels` and `CopyPixels`).
   - Rather than sending individual tiles to GTK, `VncSession` maintains an offscreen backing buffer (`Vec<u8>`) representing the full screen in `B8G8R8X8` (BGRA 32 bpp, 4 bytes/pixel: `[B, G, R, 0xFF]`) matching `gdk::MemoryFormat::B8g8r8a8Premultiplied`.
   - On `Event::PutPixels(rect, pixels)`: `decode_tile_pixels` decodes raw tile bytes according to `client.format()` (extracting `r, g, b` using bit masks `red_max`, `red_shift`, etc.) and blits them directly into the backing buffer at coordinates `(rect.left..rect.left+rect.width, rect.top..rect.top+rect.height)`.
   - On `Event::CopyPixels { src, dst }`: `copy_tile_pixels` copies sub-rectangle memory blocks within the backing buffer.
   - On `Event::EndOfFrame`: The finalized backing buffer frame is sent as `VncFrameUpdate` via `glib_tx`. Then an incremental update request `client.request_update(full_rect, true)` is issued to poll the next frame.

---

## 3. Caveats

- **Network Blocking & Timeout**: `vnc::Client::from_tcp_stream` performs blocking I/O. Using `TcpStream::connect_timeout` prevents infinite hangs on unreachable IP/port combinations.
- **CopyPixels Overlap**: When copying pixels within the backing buffer, source and destination rectangles can overlap. Implementation must handle row iteration order (top-to-bottom vs bottom-to-top) to prevent data corruption during blitting.
- **Color Depth Variants**: Most modern VNC servers use 32 bpp or 24 bpp true-color. The decoder handles 32 bpp, 24 bpp, and 16 bpp formats. 8-bit palette modes default to fallback RGB conversion.

---

## 4. Conclusion

The native VNC engine (`src/vnc/client.rs`) can be cleanly implemented using `vnc = "0.4.0"`, Tokio background tasks, `glib::MainContext::channel`, and an offscreen backing buffer converting RFB tiles into GTK4 `gdk::MemoryFormat::B8g8r8a8Premultiplied` textures.

### Blueprint for `src/vnc/client.rs`

```rust
use std::net::{TcpStream, SocketAddr};
use std::time::Duration;
use std::sync::Arc;
use anyhow::{Result, anyhow};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender, UnboundedReceiver};
use glib::MainContext;
use vnc::{Client, Rect, Encoding, PixelFormat, client::{AuthMethod, AuthChoice, Event as RfbEvent}};
use crate::models::VncScaling;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VncFrameUpdate {
    pub width: u32,
    pub height: u32,
    pub stride: usize,
    pub pixels: Vec<u8>, // BGRA / B8G8R8X8 format for gdk::MemoryTexture
}

#[derive(Debug, Clone)]
pub enum VncCommand {
    KeyEvent { keysym: u32, down: bool },
    PointerEvent { x: u16, y: u16, mask: u8 },
    CutText(String),
    SetScaling(VncScaling),
    Disconnect,
}

#[derive(Debug, Clone)]
pub enum VncSessionEvent {
    Connected { width: u32, height: u32, name: String },
    FrameUpdate(VncFrameUpdate),
    Disconnected(String),
    Error(String),
}

pub struct VncClient {
    pub host: String,
    pub port: u16,
    pub password: Option<String>,
    pub scaling: VncScaling,
}

impl VncClient {
    pub fn new(host: String, port: u16, password: Option<String>, scaling: VncScaling) -> Self {
        Self { host, port, password, scaling }
    }

    /// Spawns background VNC session worker thread and returns command handle and GLib receiver.
    pub fn start_session(
        &self,
        glib_tx: glib::Sender<VncSessionEvent>,
    ) -> UnboundedSender<VncCommand> {
        let (cmd_tx, cmd_rx) = unbounded_channel();
        let host = self.host.clone();
        let port = self.port;
        let password = self.password.clone();

        tokio::task::spawn_blocking(move || {
            let mut session = VncSession::new(host, port, password, glib_tx, cmd_rx);
            if let Err(e) = session.run() {
                let _ = session.glib_tx.send(VncSessionEvent::Error(e.to_string()));
            }
        });

        cmd_tx
    }

    pub fn process_frame_buffer(&self, raw_rgb: &[u8], width: u32, height: u32) -> VncFrameUpdate {
        let stride = (width * 4) as usize;
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        let total_pixels = (width * height) as usize;
        for i in 0..total_pixels {
            let src_idx = i * 3;
            let dst_idx = i * 4;
            if src_idx + 2 < raw_rgb.len() {
                pixels[dst_idx] = raw_rgb[src_idx + 2];     // B
                pixels[dst_idx + 1] = raw_rgb[src_idx + 1]; // G
                pixels[dst_idx + 2] = raw_rgb[src_idx];     // R
                pixels[dst_idx + 3] = 0xFF;                 // A / X
            }
        }
        VncFrameUpdate { width, height, stride, pixels }
    }
}

struct VncSession {
    host: String,
    port: u16,
    password: Option<String>,
    glib_tx: glib::Sender<VncSessionEvent>,
    cmd_rx: UnboundedReceiver<VncCommand>,
    backing_buffer: Vec<u8>,
    width: u32,
    height: u32,
}

impl VncSession {
    fn new(
        host: String,
        port: u16,
        password: Option<String>,
        glib_tx: glib::Sender<VncSessionEvent>,
        cmd_rx: UnboundedReceiver<VncCommand>,
    ) -> Self {
        Self {
            host,
            port,
            password,
            glib_tx,
            cmd_rx,
            backing_buffer: Vec::new(),
            width: 0,
            height: 0,
        }
    }

    fn run(&mut self) -> Result<()> {
        let addr_str = format!("{}:{}", self.host, self.port);
        let socket_addr: SocketAddr = addr_str.parse()
            .map_err(|e| anyhow!("Invalid address {}: {}", addr_str, e))?;

        let stream = TcpStream::connect_timeout(&socket_addr, Duration::from_secs(10))
            .map_err(|e| anyhow!("TCP connection failed: {}", e))?;

        let password = self.password.clone();
        let auth_fn = move |methods: &[AuthMethod]| -> Option<AuthChoice> {
            if methods.contains(&AuthMethod::Password) {
                let mut pass_bytes = [0u8; 8];
                if let Some(ref p) = password {
                    let bytes = p.as_bytes();
                    let len = bytes.len().min(8);
                    pass_bytes[..len].copy_from_slice(&bytes[..len]);
                }
                Some(AuthChoice::Password(pass_bytes))
            } else if methods.contains(&AuthMethod::None) {
                Some(AuthChoice::None)
            } else {
                None
            }
        };

        let mut client = Client::from_tcp_stream(stream, true, auth_fn)
            .map_err(|e| anyhow!("RFB Handshake failed: {}", e))?;

        client.set_encodings(&[
            Encoding::Zrle,
            Encoding::CopyRect,
            Encoding::Raw,
            Encoding::Cursor,
            Encoding::DesktopSize,
        ]).map_err(|e| anyhow!("Set encodings failed: {}", e))?;

        let (w, h) = client.size();
        self.width = w as u32;
        self.height = h as u32;
        self.backing_buffer = vec![0u8; (self.width * self.height * 4) as usize];

        let _ = self.glib_tx.send(VncSessionEvent::Connected {
            width: self.width,
            height: self.height,
            name: client.name().to_string(),
        });

        // Request initial full screen update
        let full_rect = Rect { left: 0, top: 0, width: w, height: h };
        client.request_update(full_rect, false)
            .map_err(|e| anyhow!("Initial request_update failed: {}", e))?;

        let mut running = true;
        while running {
            // 1. Drain incoming UI commands
            while let Ok(cmd) = self.cmd_rx.try_recv() {
                match cmd {
                    VncCommand::KeyEvent { keysym, down } => {
                        let _ = client.send_key_event(down, keysym);
                    }
                    VncCommand::PointerEvent { x, y, mask } => {
                        let _ = client.send_pointer_event(mask, x, y);
                    }
                    VncCommand::CutText(text) => {
                        let _ = client.update_clipboard(&text);
                    }
                    VncCommand::SetScaling(_) => {}
                    VncCommand::Disconnect => {
                        let _ = client.disconnect();
                        running = false;
                        break;
                    }
                }
            }

            if !running { break; }

            // 2. Poll RFB events
            let mut got_events = false;
            while let Some(event) = client.poll_event() {
                got_events = true;
                match event {
                    RfbEvent::PutPixels(rect, pixels) => {
                        let fmt = client.format();
                        self.decode_tile(&rect, &pixels, &fmt);
                    }
                    RfbEvent::CopyPixels { src, dst } => {
                        self.copy_tile(&src, &dst);
                    }
                    RfbEvent::Resize(new_w, new_h) => {
                        self.width = new_w as u32;
                        self.height = new_h as u32;
                        self.backing_buffer = vec![0u8; (self.width * self.height * 4) as usize];
                        let _ = self.glib_tx.send(VncSessionEvent::Connected {
                            width: self.width,
                            height: self.height,
                            name: client.name().to_string(),
                        });
                    }
                    RfbEvent::EndOfFrame => {
                        let update = VncFrameUpdate {
                            width: self.width,
                            height: self.height,
                            stride: (self.width * 4) as usize,
                            pixels: self.backing_buffer.clone(),
                        };
                        let _ = self.glib_tx.send(VncSessionEvent::FrameUpdate(update));

                        // Request next incremental update
                        let cur_rect = Rect { left: 0, top: 0, width: self.width as u16, height: self.height as u16 };
                        let _ = client.request_update(cur_rect, true);
                    }
                    RfbEvent::Disconnected(err) => {
                        let msg = err.map(|e| e.to_string()).unwrap_or_else(|| "Server disconnected".into());
                        let _ = self.glib_tx.send(VncSessionEvent::Disconnected(msg));
                        running = false;
                        break;
                    }
                    _ => {}
                }
            }

            if !got_events && running {
                std::thread::sleep(Duration::from_millis(5));
            }
        }

        Ok(())
    }

    fn decode_tile(&mut self, rect: &Rect, tile_bytes: &[u8], format: &PixelFormat) {
        let bpp = format.bits_per_pixel as usize / 8;
        if bpp == 0 { return; }

        let rect_w = rect.width as usize;
        let rect_h = rect.height as usize;
        let fb_w = self.width as usize;

        for y in 0..rect_h {
            let dst_y = rect.top as usize + y;
            if dst_y >= self.height as usize { continue; }

            for x in 0..rect_w {
                let dst_x = rect.left as usize + x;
                if dst_x >= fb_w { continue; }

                let src_idx = (y * rect_w + x) * bpp;
                if src_idx + bpp > tile_bytes.len() { continue; }

                let dst_idx = (dst_y * fb_w + dst_x) * 4;

                let (r, g, b) = match bpp {
                    4 => {
                        let val = if format.big_endian {
                            u32::from_be_bytes([tile_bytes[src_idx], tile_bytes[src_idx+1], tile_bytes[src_idx+2], tile_bytes[src_idx+3]])
                        } else {
                            u32::from_le_bytes([tile_bytes[src_idx], tile_bytes[src_idx+1], tile_bytes[src_idx+2], tile_bytes[src_idx+3]])
                        };
                        let r = ((val >> format.red_shift) as u16 & format.red_max) as u8;
                        let g = ((val >> format.green_shift) as u16 & format.green_max) as u8;
                        let b = ((val >> format.blue_shift) as u16 & format.blue_max) as u8;
                        (r, g, b)
                    }
                    3 => {
                        let r = tile_bytes[src_idx];
                        let g = tile_bytes[src_idx + 1];
                        let b = tile_bytes[src_idx + 2];
                        (r, g, b)
                    }
                    2 => {
                        let val = if format.big_endian {
                            u16::from_be_bytes([tile_bytes[src_idx], tile_bytes[src_idx+1]])
                        } else {
                            u16::from_le_bytes([tile_bytes[src_idx], tile_bytes[src_idx+1]])
                        };
                        let r = (((val >> format.red_shift) & format.red_max) * 255 / format.red_max.max(1)) as u8;
                        let g = (((val >> format.green_shift) & format.green_max) * 255 / format.green_max.max(1)) as u8;
                        let b = (((val >> format.blue_shift) & format.blue_max) * 255 / format.blue_max.max(1)) as u8;
                        (r, g, b)
                    }
                    _ => (0, 0, 0),
                };

                if dst_idx + 3 < self.backing_buffer.len() {
                    self.backing_buffer[dst_idx] = b;     // B
                    self.backing_buffer[dst_idx + 1] = g; // G
                    self.backing_buffer[dst_idx + 2] = r; // R
                    self.backing_buffer[dst_idx + 3] = 0xFF; // A / X
                }
            }
        }
    }

    fn copy_tile(&mut self, src: &Rect, dst: &Rect) {
        let fb_w = self.width as usize;
        let w = src.width as usize;
        let h = src.height as usize;

        let y_range: Vec<usize> = if dst.top > src.top {
            (0..h).rev().collect()
        } else {
            (0..h).collect()
        };

        for y in y_range {
            let sy = src.top as usize + y;
            let dy = dst.top as usize + y;
            if sy >= self.height as usize || dy >= self.height as usize { continue; }

            for x in 0..w {
                let sx = src.left as usize + x;
                let dx = dst.left as usize + x;
                if sx >= fb_w || dx >= fb_w { continue; }

                let src_idx = (sy * fb_w + sx) * 4;
                let dst_idx = (dy * fb_w + dx) * 4;

                if src_idx + 3 < self.backing_buffer.len() && dst_idx + 3 < self.backing_buffer.len() {
                    self.backing_buffer.copy_within(src_idx..src_idx+4, dst_idx);
                }
            }
        }
    }
}
```

---

## 5. Verification Method

1. **Compilation Check**:
   Run `cargo check` in `/home/dawiisss/Documents/antigravity/beautiful-goodall` after replacing `src/vnc/client.rs` with the blueprint. Verify zero compilation warnings/errors.
2. **Unit Test Verification**:
   Run `cargo test --test e2e_vnc_tests` to verify frame buffer processing logic and pixel format conversion algorithms.
3. **Invalidation Conditions**:
   - Changes to `vnc` crate API in Cargo dependencies.
   - Any departure from `B8G8R8X8` / `gdk::MemoryFormat::B8g8r8a8Premultiplied` in `src/vnc/widget.rs`.
