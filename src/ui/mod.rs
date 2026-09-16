pub mod discovery;
pub mod editor;
pub mod export_dialog;
pub mod import_dialog;
pub mod preferences;
pub mod quick_connect;
pub mod session;
pub mod shortcuts;
pub mod sidebar;
pub mod window;

pub use discovery::{DiscoveredService, DiscoveryDialog};
pub use editor::ConnectionEditor;
pub use export_dialog::ExportDialog;
pub use import_dialog::ImportDialog;
pub use preferences::{apply_theme, PreferencesWindow};
pub use quick_connect::{parse_quick_connect, QuickConnectDialog};
pub use session::ActiveSession;
pub use shortcuts::ShortcutsDialog;
pub use sidebar::{
    create_connection_row, set_row_active_badge, set_row_status_dot, STATUS_DOT_CSS,
};
pub use window::MainWindow;
