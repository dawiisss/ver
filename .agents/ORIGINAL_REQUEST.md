# Original User Request

## Initial Request — 2026-08-12T12:35:19Z

Rewrite the "beautiful-goodall" (VER - Very Easy Remote) connection manager application entirely in Rust. The application currently uses Python, GTK4, Libadwaita, and a custom C extension for VNC. The new Rust version must achieve full feature parity, utilizing `gtk4-rs` and `libadwaita` for a functional GTK4 prototype, and implement a native embedded VNC client using a pure Rust VNC crate (`vnc-rs`). You should use Rust-native UI patterns rather than strictly copying the Python architecture.

Working directory: /home/dawiisss/Documents/antigravity/beautiful-goodall
Integrity mode: development

## Requirements

### R1. Rust Project Skeleton & Data Models
The existing Python source files should be replaced with a Cargo project. The connection data model must parse and serialize the existing connections JSON file perfectly using `serde`. 

### R2. Connection Manager UI
Implement a functional GTK4/Libadwaita interface that allows viewing, editing, and initiating connections. It should feel robust and idiomatically Rust-based.

### R3. Native VNC Client
Implement the VNC client directly in Rust using the `vnc` crate. It should connect, decode frames (Tight/ZRLE), and render raw pixels directly into a `gtk4::Picture` or `gtk4::DrawingArea` efficiently, handling mouse and keyboard events.

### R4. RDP and SSH Integration
Retain the ability to launch RDP sessions (via `xfreerdp3`) and SSH sessions.

## Acceptance Criteria

### Compilation
- [ ] `cargo build` completes successfully with zero compilation errors.

### Data Management
- [ ] The app successfully reads the existing connections JSON file on startup.
- [ ] Modifying a connection in the UI correctly updates and saves the JSON file.

### UI & VNC Verification
- [ ] Launching the app produces a functional GTK4 window.
- [ ] Selecting a VNC connection successfully initializes the `vnc-rs` client and renders the remote framebuffer without crashing.
- [ ] Keyboard and mouse inputs are successfully propagated to the VNC server.
