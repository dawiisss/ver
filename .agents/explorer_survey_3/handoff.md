# Handoff Report: VNC, C Extension, RDP & SSH Investigation

## 1. Observation

### 1.1 Existing VNC C Extension (`src/core/ext/vnc_ext.c`)
- **File path**: `src/core/ext/vnc_ext.c`
- **Dependencies**: `#include <rfb/rfbclient.h>`, `#include <pthread.h>` (lines 1-2).
- **Context structure**:
  ```c
  typedef struct {
      rfbClient* client;
      pthread_t thread;
      int is_running;
      char* password;
      framebuffer_cb_t cb;
  } VncContext;
  ```
- **Connection & Protocol Setup** (lines 66-95):
  - Instantiates `rfbClient* cl = rfbGetClient(8, 3, 4);` (32 bpp, 24 depth, trueColour=1, redShift=16, greenShift=8, blueShift=0).
  - Encodings: `cl->appData.encodingsString = "tight zrle hextile raw";`.
  - Password callback: `cl->GetPassword = get_password;`.
  - Framebuffer update callback: `cl->GotFrameBufferUpdate = update_framebuffer;` which passes full `client->frameBuffer` pointer to C callback `ctx->cb(client->width, client->height, stride, client->frameBuffer)`.
  - Spawns background thread running `vnc_thread`.
- **Worker Thread Loop** (lines 37-64):
  - Calls `rfbInitClient(ctx->client, NULL, NULL)`.
  - Loop: `WaitForMessage(ctx->client, 50000)` (50ms timeout) and `HandleRFBServerMessage(ctx->client)`.
- **Input Handling**:
  - `vnc_send_key(ctx, keysym, down)` -> `SendKeyEvent(ctx->client, keysym, down)` (lines 106-110).
  - `vnc_send_pointer(ctx, x, y, button_mask)` -> `SendPointerEvent(ctx->client, x, y, button_mask)` (lines 112-116).

### 1.2 Python GTK VNC Widget (`src/ui/vnc_widget.py`)
- **File path**: `src/ui/vnc_widget.py`
- **Widget class**: `class VncWidget(Gtk.Picture):` (line 48). Sets `self.set_focusable(True)` (line 51).
- **C Extension Binding**: Uses `ctypes` to load `vnc_ext.so` and connect callback `_on_framebuffer_update` (lines 31-72).
- **Rendering Loop**: `GLib.timeout_add(16, self._render_frame)` (~60 FPS) (line 74). In `_render_frame` (lines 108-127):
  ```python
  size = self._fb_stride * self._fb_h
  buffer = ctypes.string_at(self._fb_ptr, size)
  bytes_glib = GLib.Bytes.new(buffer)
  texture = Gdk.MemoryTexture.new(self._fb_w, self._fb_h, Gdk.MemoryFormat.B8G8R8X8, bytes_glib, self._fb_stride)
  self.set_paintable(texture)
  ```
- **Coordinate Transformation**: `_map_coords(x, y)` (lines 129-158) maps event coordinates `(x, y)` in widget space to framebuffer pixel space `[0, fb_w-1] x [0, fb_h-1]`, taking letterboxing/pillarboxing offset and scaling into account.
- **Event Controllers**:
  - `Gtk.EventControllerKey` (lines 77-80): `key-pressed` and `key-released` invoke `vnc_send_key(ctx, keyval, 1 or 0)`.
  - `Gtk.EventControllerMotion` (lines 82-84): `motion` maps coords and invokes `vnc_send_pointer(ctx, x, y, button_mask)`.
  - `Gtk.GestureClick` (lines 86-89): `pressed` and `released` update button mask (`1` -> bit 0, `2` -> bit 1, `3` -> bit 2) and invoke `vnc_send_pointer`.

### 1.3 RDP & SSH Session Launching (`src/core/launcher.py` & `src/core/rdp_client.py`)
- **File paths**: `src/core/launcher.py`, `src/core/rdp_client.py`, `src/ui/terminal.py`.
- **RDP Discovery & Args**:
  - Tries `xfreerdp3`, fallback `xfreerdp`, `wlfreerdp`.
  - Arguments: `/v:<host>:<port>`, `/u:<user>`, `/p:<pass>`, `/cert:ignore`, `/dynamic-resolution`, `+clipboard`/`-clipboard`, `/bpp:<depth>`, `/multimon`, `/f`, `/sound`, `/parent-window:<xid>`.
  - Process launch: `subprocess.Popen(args, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)`.
- **SSH Launcher & Terminal Discovery**:
  - Base command: `["ssh", "-p", "<port>", "<user>@<host>"]`.
  - Terminal fallback list: `ptyxis`, `kgx`, `gnome-terminal`, `konsole`, `xfce4-terminal`, `kitty`, `alacritty`, `xterm`.
  - Process launch: `subprocess.Popen(full_cmd, stdin=DEVNULL, stdout=DEVNULL, stderr=DEVNULL)`.

### 1.4 Rust Environment (`Cargo.toml`)
- **File path**: `Cargo.toml`
- Contains: `gtk4` (v0.7), `libadwaita` (v0.5), `vnc` (v0.4.0), `serde` (v1.0), `serde_json` (v1.0), `tokio` (v1.34), `oo7` (v0.3), `anyhow` (v1.0).

---

## 2. Logic Chain

1. **VNC Protocol & C Extension Replacement**:
   - The C extension (`vnc_ext.c`) uses `libvncclient` to handle RFB version negotiation, VNC authentication, Tight/ZRLE frame decoding, and event packet generation (`SendKeyEvent`, `SendPointerEvent`).
   - The `vnc` crate (v0.4.0) in `Cargo.toml` provides native Pure Rust implementations of these exact RFB operations, eliminating the need for `libvncclient` and C FFI (`vnc_ext.c`).
   - Running `vnc-rs` in an async `tokio` task or background thread allows decoupling RFB network I/O from GTK main thread operations. Frame updates can be transmitted to GTK using `glib::MainContext::channel`.

2. **GTK4 Widget & Rendering**:
   - Python's `VncWidget` inherits from `Gtk.Picture` and paints frames using `Gdk.MemoryTexture.new(w, h, B8G8R8X8, bytes, stride)`.
   - In `gtk4-rs`, `gtk4::Picture` with `gdk::MemoryTexture::new` provides identical zero-copy GPU texture rendering.
   - Coordinate transformation math (`_map_coords`) must be preserved in Rust to convert mouse events inside letterboxed/pillarboxed `gtk4::Picture` widgets into remote framebuffer pixel space `[0, width-1] x [0, height-1]`.

3. **Input Propagation**:
   - GTK4 event controllers (`gtk4::EventControllerKey`, `gtk4::EventControllerMotion`, `gtk4::GestureClick`) attach directly to `gtk4::Picture`.
   - GDK `keyval` converts directly to `u32` keysyms via `.into_glib()`, matching standard RFB key event values expected by `vnc-rs`.
   - Pointer buttons match standard bitmask representation: Bit 0 (`0x01`) = Left, Bit 1 (`0x02`) = Middle, Bit 2 (`0x04`) = Right.

4. **Process Management**:
   - `std::process::Command` with `Stdio::null()` for stdin/stdout/stderr replaces `subprocess.Popen(..., stdin=DEVNULL, ...)` cleanly in Rust.
   - Executable discovery can use the `which` crate or manual path search.

---

## 3. Caveats

- **libvncclient vs vnc-rs Encoding Support**:
  - `libvncclient` supports Tight, ZRLE, Hextile, Raw. `vnc` crate (v0.4.0) supports Raw, Tight, ZRLE. Ensure server encoding request string specifies encodings supported by `vnc-rs`.
- **Secret Storage (`oo7`)**:
  - The Python code uses `core/secrets.py` (libsecret / keyring). `Cargo.toml` includes `oo7 = "0.3"` which is the modern pure Rust client for Secret Service / Keyring API.
- **Embedded Terminal Widget (`Vte`)**:
  - `src/ui/terminal.py` uses `Vte.Terminal`. For external SSH spawning, `std::process::Command` launching an external terminal (e.g. `ptyxis`, `kgx`, `gnome-terminal`) is fully supported without extra VTE C bindings.

---

## 4. Conclusion

Replacing the C extension and Python background logic with pure Rust is completely viable and direct.
1. The `vnc` crate replaces `vnc_ext.c` and `libvncclient`.
2. `gtk4::Picture` with `gdk::MemoryTexture` provides high-performance rendering matching the existing GTK4 architecture.
3. GTK4 event controllers in `gtk4-rs` propagate keysyms and pointer bitmasks seamlessly.
4. `std::process::Command` handles detached child process spawning for `xfreerdp3` and SSH external terminals.

---

## 5. Verification Method

To verify these findings and eventual implementer work:
1. Inspect C extension and Python widget source files:
   - `view_file /home/dawiisss/Documents/antigravity/beautiful-goodall/src/core/ext/vnc_ext.c`
   - `view_file /home/dawiisss/Documents/antigravity/beautiful-goodall/src/ui/vnc_widget.py`
   - `view_file /home/dawiisss/Documents/antigravity/beautiful-goodall/src/core/launcher.py`
2. Inspect Rust workspace configuration:
   - `view_file /home/dawiisss/Documents/antigravity/beautiful-goodall/Cargo.toml`
3. Verify Cargo build environment:
   - Run `cargo check` or `cargo build` in `/home/dawiisss/Documents/antigravity/beautiful-goodall`.
