# Architecture & UI Survey of VER (Very Easy Remote) Python Application

**Date**: 2026-08-12  
**Target Repository**: `/home/dawiisss/Documents/antigravity/beautiful-goodall`  
**Author**: `explorer_survey_1`  

---

## Executive Summary

VER ("Very Easy Remote", `com.example.ver`) is a GTK4 / Libadwaita desktop remote connection manager written in Python. It supports three primary protocols: **RDP**, **VNC**, and **SSH**. Connection definitions are persisted in a JSON file (`~/.config/ver/connections.json`), while sensitive credentials (passwords) are securely offloaded to the system keyring using the Python `keyring` library (`ver_remote_connection_manager`).

The application features:
1. A single-window **Sidebar/Content** UI design using Libadwaita widgets (`Adw.ApplicationWindow`, `Adw.HeaderBar`, `Adw.ActionRow`, `Adw.EntryRow`, `Adw.ComboRow`, `Adw.SwitchRow`).
2. An embedded **VNC client** powered by a native C shared library (`src/core/ext/vnc_ext.c` built as `vnc_ext.so` using `libvncclient` and `pthreads`), rendered into a custom GTK4 widget (`VncWidget` / `Gtk.Picture`).
3. External process execution for **RDP** (via `xfreerdp3` / `xfreerdp`) and **SSH** (spawning external terminal emulators like `ptyxis`, `kgx`, `gnome-terminal`, `konsole`, etc., or embedded `Vte.Terminal`).
4. **Local Network Discovery** via Zeroconf (`_ssh._tcp.local.`, `_rfb._tcp.local.`).
5. **Wake-on-LAN (WoL)** magic packet broadcasting via UDP.
6. **System Tray Integration** using a dedicated subprocess (`src/tray_daemon.py` with `pystray`).

---

## 1. System Architecture & Module Map

### 1.1 Architecture Diagram

```
+-----------------------------------------------------------------------------------+
|                                 VerApplication                                    |
|                             (src/app.py - Adw.App)                                |
|                                       |                                           |
|         +-----------------------------+-----------------------------+             |
|         |                                                           |             |
|         v                                                           v             |
|    MainWindow                                                  Tray Daemon        |
| (ui/window.py)                                             (src/tray_daemon.py)   |
|         |                                                           | (stdout)    |
|   +-----+-----------------------+                                   v             |
|   |                             |                           (SHOW / QUIT Pipe)    |
|   v                             v                                                 |
| Sidebar                    Content Area                                           |
| - ListBox (Groups/Rows)    - QuickConnect SearchEntry                             |
| - Header Actions           - ConnectionEditor (ui/editor.py)                       |
|                             +--- General Settings (Name, Proto, Host, Port, User) |
|                             +--- Network & WoL (MAC Address)                      |
|                             +--- Advanced Protocol Settings                       |
|                             +--- Action Buttons (Save, Connect, Delete, Dup, Wake)|
+---------+-----------------------+-------------------------------------------------+
          |                       |
          v                       v
+-------------------+   +--------------------+   +----------------------------------+
| Storage & Secrets |   | Protocols/Launcher |   | UI Dialogs & Widgets             |
| (core/storage.py) |   | (core/launcher.py) |   | - PreferencesWindow (preferences)|
| - connections.json|   | - FreeRDP Launcher |   | - DiscoveryDialog (discovery.py) |
| (core/secrets.py) |   | - Terminal/SSH     |   | - VncWidget (vnc_widget.py)      |
| - Keyring storage |   | - core/network.py  |   |   +-- vnc_ext.so (C / libvncclient)|
| (core/config.py)  |   |   (Wake-on-LAN)    |   | - TerminalView (terminal.py)     |
| - config.json     |   | - core/rdp_client  |   |   +-- VTE Terminal               |
+-------------------+   +--------------------+   +----------------------------------+
```

### 1.2 Source File Structure

| File | Subsystem / Role | Key Classes & Functions |
|------|------------------|-------------------------|
| `src/app.py` | Application Entrypoint | `VerApplication(Adw.Application)`: theme loading, tray listener thread, window activation. |
| `src/main.py` | Deprecated Stub | Replaced by `app.py` & `ui/` architecture. |
| `src/models.py` | Data Model | `Connection` dataclass: serialization/deserialization to/from dictionary. |
| `src/tray_daemon.py` | System Tray Daemon | Standalone `pystray` icon process sending "SHOW" / "QUIT" over stdout pipe. |
| `src/core/config.py` | Global Configuration | `load_app_config()`, `save_app_config()` (`~/.config/ver/config.json`). |
| `src/core/storage.py` | Connection Persistence | `load_connections()`, `save_connections()` (`~/.config/ver/connections.json`). |
| `src/core/secrets.py` | Credential Security | `save_password()`, `get_password()`, `delete_password()` via `keyring` (`ver_remote_connection_manager`). |
| `src/core/launcher.py` | Connection Dispatcher | `launch_connection()`: spawns `xfreerdp`, `gnome-connections`/`krdc`/`vncviewer`, or terminal SSH commands. |
| `src/core/rdp_client.py` | FreeRDP Command Builder | `RdpClientManager`: command string construction and child process management. |
| `src/core/network.py` | Network Utilities | `send_wol(mac_address)`: constructs & broadcasts 102-byte WoL UDP magic packet. |
| `src/core/ext/vnc_ext.c` | C VNC Client Extension | `vnc_connect()`, `vnc_disconnect()`, `vnc_send_key()`, `vnc_send_pointer()`. Multi-threaded RFB event loop via `libvncclient`. |
| `src/ui/window.py` | Main Window | `MainWindow(Adw.ApplicationWindow)`: Sidebar with ListBox, QuickConnect search entry, connection switching. |
| `src/ui/editor.py` | Connection Editor View | `ConnectionEditor(Gtk.Box)`: Libadwaita forms for protocol settings, action buttons. |
| `src/ui/preferences.py` | Preferences Modal | `PreferencesWindow(Adw.PreferencesWindow)`: appearance theme configuration & connection data export stub. |
| `src/ui/discovery.py` | mDNS/Zeroconf Scanner | `DiscoveryDialog(Adw.Window)`: local network service discovery for `_ssh._tcp.local.` and `_rfb._tcp.local.`. |
| `src/ui/vnc_widget.py` | VNC Rendering & Input | `VncWidget(Gtk.Picture)`: `ctypes` bindings to `vnc_ext.so`, 60 FPS texture updating, coordinate aspect-ratio mapping & input events. |
| `src/ui/terminal.py` | Embedded SSH View | `TerminalView(Gtk.Box)`: embedded terminal emulator wrapping `Vte.Terminal`. |

---

## 2. Data Models & File Formats

### 2.1 Connection Model (`src/models.py`)

The `Connection` data structure contains:

```python
@dataclass
class Connection:
    id: str = field(default_factory=lambda: str(uuid.uuid4()))
    name: str = "New Connection"
    protocol: str = "rdp"          # "rdp", "vnc", or "ssh"
    host: str = ""
    port: int = 3389
    username: str = ""
    mac_address: str = ""          # MAC address for WoL
    group: str = "Default"         # Used for sidebar category grouping
    advanced_settings: dict = field(default_factory=dict)
```

### 2.2 Advanced Settings Fields

`advanced_settings` is a key-value dictionary containing:
- `rdp_multimon` (bool): Pass `/multimon` to FreeRDP.
- `rdp_fullscreen` (bool): Pass `/f` to FreeRDP.
- `rdp_audio` (bool): Pass `/sound` to FreeRDP.
- `vnc_viewonly` (bool): View-only mode for VNC sessions.
- `vnc_shared` (bool): Shared session mode for VNC sessions.
- `clipboard_sharing` (bool): Enable/disable clipboard sharing (`+clipboard`/`-clipboard` for RDP; `-AcceptClipboard`/`-SendClipboard` for external VNC).
- `color_depth` (int): Color depth code (`0`: Auto, `32`: 32-bit, `24`: 24-bit, `16`: 16-bit, `8`: 8-bit).
- `vnc_scaling` (str): Scaling mode (`"Original Size"`, `"Fit to Window"`, `"Stretch"`).

### 2.3 JSON Storage Format (`~/.config/ver/connections.json`)

```json
[
    {
        "id": "c1f7a8b4-92e1-4c10-b982-123456789abc",
        "name": "Office Workstation",
        "protocol": "rdp",
        "host": "192.168.1.100",
        "port": 3389,
        "username": "admin",
        "mac_address": "AA:BB:CC:DD:EE:FF",
        "group": "Work",
        "advanced_settings": {
            "rdp_multimon": true,
            "rdp_fullscreen": false,
            "rdp_audio": true,
            "vnc_viewonly": false,
            "vnc_shared": false,
            "clipboard_sharing": true,
            "color_depth": 32,
            "vnc_scaling": "Fit to Window"
        }
    }
]
```

*Note: Passwords are NEVER written to `connections.json`. They are stored in system secret storage via `keyring.set_password("ver_remote_connection_manager", connection_id, password)`.*

---

## 3. GTK4 / Libadwaita UI Layout & Component Hierarchy

### 3.1 Main Window Hierarchy (`src/ui/window.py`)

```
MainWindow (Adw.ApplicationWindow - size: 1050x700)
 └── main_box (Gtk.Box - Horizontal)
      ├── sidebar_box (Gtk.Box - Vertical, width: 280px, CSS: "background")
      │    ├── HeaderBar (Adw.HeaderBar, show_end_title_buttons=False)
      │    │    ├── Pack Start: btn_prefs (Gtk.Button, icon: "open-menu-symbolic")
      │    │    ├── Pack End: btn_add (Gtk.Button, icon: "list-add-symbolic")
      │    │    └── Pack End: btn_scan (Gtk.Button, icon: "network-wireless-symbolic")
      │    └── ScrolledWindow (Gtk.ScrolledWindow, vexpand=True)
      │         └── list_box (Gtk.ListBox, CSS: "navigation-sidebar", single selection)
      │              ├── Group Header Row: ListBoxRow (non-selectable) -> Label ("<b>Group Name</b>")
      │              └── Connection Row: Adw.ActionRow (title=name, subtitle="host:port", name=id)
      │
      ├── Separator (Gtk.Separator - Vertical)
      │
      └── content_box (Gtk.Box - Vertical, hexpand=True)
           ├── content_header (Adw.HeaderBar, show_start_title_buttons=False)
           │    └── Title Widget: quick_connect (Gtk.SearchEntry)
           └── content_scroll (Gtk.ScrolledWindow, vexpand=True)
                └── editor_bin (Gtk.Box, vexpand=True, hexpand=True)
                     └── active_editor (ConnectionEditor widget)
```

### 3.2 Connection Editor Hierarchy (`src/ui/editor.py`)

`ConnectionEditor` is a `Gtk.Box` (Vertical, margins: 24px, spacing: 12px) containing:

1. **General Settings** (`Adw.PreferencesGroup`, title: "General Settings"):
   - `entry_name`: `Adw.EntryRow` (title: "Name")
   - `entry_group`: `Adw.EntryRow` (title: "Group")
   - `entry_proto`: `Adw.ComboRow` (title: "Protocol", choices: `["rdp", "vnc", "ssh"]`)
   - `entry_host`: `Adw.EntryRow` (title: "Host (IP or Domain)")
   - `entry_port`: `Adw.EntryRow` (title: "Port")
   - `entry_user`: `Adw.EntryRow` (title: "Username")
   - `entry_pass`: `Adw.PasswordEntryRow` (title: "Password (RDP only)")

2. **Network & Hardware** (`Adw.PreferencesGroup`, title: "Network & Hardware"):
   - `entry_mac`: `Adw.EntryRow` (title: "MAC Address (For WoL)")

3. **Advanced Protocol Settings** (`Adw.PreferencesGroup`, title: "Advanced Protocol Settings"):
   - `rdp_multimon`: `Adw.SwitchRow` (title: "Multi-Monitor Support", visible when RDP)
   - `rdp_fullscreen`: `Adw.SwitchRow` (title: "Fullscreen Mode", visible when RDP)
   - `rdp_audio`: `Adw.SwitchRow` (title: "Audio Redirection", visible when RDP)
   - `vnc_viewonly`: `Adw.SwitchRow` (title: "View-Only Mode", visible when VNC)
   - `vnc_shared`: `Adw.SwitchRow` (title: "Shared Session", visible when VNC)
   - `clipboard`: `Adw.SwitchRow` (title: "Clipboard Sharing", visible when RDP or VNC)
   - `color_depth`: `Adw.ComboRow` (title: "Color Depth", choices: Auto/32-bit/24-bit/16-bit/8-bit, visible when RDP or VNC)
   - `vnc_scaling`: `Adw.ComboRow` (title: "Scaling Mode", choices: Original Size/Fit to Window/Stretch, visible when VNC)

4. **Action Buttons Box** (`Gtk.Box`, Horizontal, spacing: 12px, centered):
   - `btn_del`: `Gtk.Button` (label: "Delete", style: `destructive-action`)
   - `btn_dup`: `Gtk.Button` (label: "Duplicate")
   - `btn_save`: `Gtk.Button` (label: "Save", style: `suggested-action`)
   - `btn_wol`: `Gtk.Button` (label: "Wake") -> sends WoL magic packet
   - `btn_connect`: `Gtk.Button` (label: "Connect", style: `pill`) -> saves & launches connection

---

## 4. Secondary Windows & Dialogs

### 4.1 Global Preferences Window (`src/ui/preferences.py`)
- Subclasses `Adw.PreferencesWindow` (default size: 500x400).
- **Appearance Page**:
  - `theme_row` (`Adw.ComboRow`, title: "Application Theme", options: System Default, Dark Mode, Light Mode). Dynamically alters `Adw.StyleManager` color scheme and saves to `~/.config/ver/config.json`.
- **Data Management Page**:
  - `export_row` (`Adw.ActionRow`, title: "Export Connections") with an "Export Data" button.

### 4.2 Local Network Discovery Dialog (`src/ui/discovery.py`)
- Subclasses `Adw.Window` (default size: 400x500).
- Contains an `Adw.HeaderBar`, a `Gtk.Spinner` (animated for 5 seconds), and a `Gtk.ListBox`.
- Uses `zeroconf.ServiceBrowser` listening for `_ssh._tcp.local.` and `_rfb._tcp.local.`.
- Discovered hosts populate `list_box` with `Adw.ActionRow` items (`title=service_name`, `subtitle="ip:port (PROTO)"`).
- Each row features an "Add" button (`suggested-action`) that creates a `Connection` record, invokes `on_add_callback`, and updates UI state.

### 4.3 Embedded VNC Session Window (`src/ui/vnc_widget.py` & `src/ui/window.py`)
- Subclasses `Gtk.Window` (title: `"VNC: {conn.name}"`, default size: 1024x768).
- Holds a single child widget: `VncWidget` (subclass of `Gtk.Picture`).
- **Framebuffer Pipeline**:
  1. `vnc_ext.so` launches a C thread (`vnc_thread`), initializes `rfbClient`, connects to VNC server, and receives RFB framebuffer updates.
  2. C callback `update_framebuffer()` sends pointer `client->frameBuffer`, width, height, and stride to Python callback `_on_framebuffer_update()`.
  3. Python `_render_frame()` (called via `GLib.timeout_add(16)`) constructs a `Gdk.MemoryTexture` (format `B8G8R8X8`) and calls `self.set_paintable(texture)`.
- **Input Forwarding**:
  - `Gtk.EventControllerKey`: maps `key-pressed` and `key-released` keyval to `vnc_send_key()`.
  - `Gtk.EventControllerMotion`: maps cursor movement with aspect-ratio geometric projection (`_map_coords`) to `vnc_send_pointer()`.
  - `Gtk.GestureClick`: maps mouse button bits (Left=1, Middle=2, Right=4) to `vnc_send_pointer()`.

---

## 5. User Interaction Flows & State Transitions

1. **Application Lifecycle**:
   - `VerApplication` initializes GTK4/Adw, loads `config.json` for dark/light theme, creates `MainWindow`, loads `connections.json`, and starts `tray_daemon.py`.
   - Window close button hides the window (`set_visible(False)`). The process remains running in background via system tray.

2. **Quick Connect Flow**:
   - User types URI into top search bar (e.g. `ssh://user@remotehost:2222` or `rdp://10.0.0.5`).
   - On Enter, `_on_quick_connect_activate()` parses the scheme/host/port/user, creates an ephemeral `Connection`, and launches the session immediately.

3. **Connection Management Flow**:
   - Selecting a connection in the sidebar updates `editor_bin` to render details in `ConnectionEditor`.
   - Dynamic UI visibility: Changing protocol combo box dynamically shows/hides protocol-specific switches (RDP multi-mon/audio/fullscreen vs VNC view-only/shared/scaling).
   - Changing protocol updates default port (3389 for RDP, 5900 for VNC, 22 for SSH).

4. **Connection Execution Flow**:
   - Clicking **Connect**:
     - **VNC**: Obtains password from `keyring`, instantiates `VncWidget` connected to `vnc_ext.so`, and opens native embedded `VncWindow`.
     - **RDP**: Invokes `launcher.launch_connection()`, building `xfreerdp3`/`xfreerdp` command line args (`/v:`, `/u:`, `/p:`, `/cert:ignore`, `/dynamic-resolution`, `+clipboard`, `/multimon`, `/f`, `/sound`), launching process detached via `stdin=DEVNULL`.
     - **SSH**: Scans for system terminal emulators (`ptyxis`, `kgx`, `gnome-terminal`, `konsole`, `kitty`, etc.) and executes `terminal -e ssh [-p port] user@host`.

---

## 6. Synthesis for Rust Parity Re-implementation

To achieve 100% feature parity in Rust:
1. **Data Model**: Struct `Connection` with `serde` annotations matching all Python fields (`id`, `name`, `protocol`, `host`, `port`, `username`, `mac_address`, `group`, `advanced_settings`).
2. **Keyring Integration**: Use Rust `oo7` or `keyring` crate to store passwords under service `ver_remote_connection_manager`.
3. **UI Toolkit**: `gtk4-rs` and `libadwaita-rs` using native widgets (`adw::ApplicationWindow`, `adw::HeaderBar`, `adw::PreferencesGroup`, `adw::ActionRow`, `adw::EntryRow`, `adw::ComboRow`, `adw::SwitchRow`).
4. **VNC Engine**: Replace `vnc_ext.c` / `libvncclient` with `vnc` Rust crate (`vnc-rs`), decoding Tight/ZRLE/Raw framebuffers directly into a pixel buffer, creating a `gdk::MemoryTexture` or `gtk4::DrawingArea` / `gtk4::Picture`.
5. **Process Launching**: Standard Rust `std::process::Command` to launch `xfreerdp3` / `xfreerdp` and external terminals for SSH.
