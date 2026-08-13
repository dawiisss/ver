## 2026-08-12T17:39:30Z
You are explorer_m3_1. Your working directory is /home/dawiisss/Documents/antigravity/beautiful-goodall/.agents/explorer_m3_1.
Mandatory requirements:
1. Read /home/dawiisss/Documents/antigravity/beautiful-goodall/.agents/ORIGINAL_REQUEST.md
2. Read /home/dawiisss/Documents/antigravity/beautiful-goodall/.agents/orchestrator/PROJECT.md

Your mission:
Investigate and design the native VNC async client engine (`src/vnc/client.rs`):
1. Examine `Cargo.toml` dependency `vnc = "0.4.0"` (or `vnc-rs` API). Determine how `vnc::Client` performs RFB protocol handshake, password auth (`VncAuth`), framebuffer updates (`Rect` decoding), and thread/task dispatch.
2. Design the async VNC client worker thread loop (`VncSession`) that runs RFB event processing in background Tokio tasks and streams framebuffer update messages (`VncFrameUpdate`) to the GTK main loop via `glib::MainContext::channel` or `tokio::sync::mpsc`.
3. Design pixel format conversion from RFB pixel formats (e.g. RGB888/BGR888) to `gdk::MemoryTexture`-compatible `gdk::MemoryFormat::B8g8r8a8Premultiplied` or `R8g8b8a8`.
4. Formulate a concrete, step-by-step implementation blueprint for `src/vnc/client.rs`. Do NOT edit source code files yourself (you are read-only).
Write your findings and evidence report into /home/dawiisss/Documents/antigravity/beautiful-goodall/.agents/explorer_m3_1/handoff.md and report back via send_message.
