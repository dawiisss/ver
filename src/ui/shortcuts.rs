use gtk::prelude::*;

const SHORTCUTS_UI: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<interface>
  <object class="GtkShortcutsWindow" id="shortcuts_window">
    <property name="modal">1</property>
    <child>
      <object class="GtkShortcutsSection">
        <property name="section-name">shortcuts</property>
        <property name="max-height">12</property>
        <child>
          <object class="GtkShortcutsGroup">
            <property name="title">General</property>
            <child>
              <object class="GtkShortcutsShortcut">
                <property name="title">Quick Connect</property>
                <property name="accelerator">&lt;Ctrl&gt;k</property>
              </object>
            </child>
            <child>
              <object class="GtkShortcutsShortcut">
                <property name="title">New Connection</property>
                <property name="accelerator">&lt;Ctrl&gt;n</property>
              </object>
            </child>
            <child>
              <object class="GtkShortcutsShortcut">
                <property name="title">Search Connections</property>
                <property name="accelerator">&lt;Ctrl&gt;f</property>
              </object>
            </child>
            <child>
              <object class="GtkShortcutsShortcut">
                <property name="title">Preferences</property>
                <property name="accelerator">&lt;Ctrl&gt;comma</property>
              </object>
            </child>
            <child>
              <object class="GtkShortcutsShortcut">
                <property name="title">Network Discovery</property>
                <property name="accelerator">&lt;Ctrl&gt;d</property>
              </object>
            </child>
            <child>
              <object class="GtkShortcutsShortcut">
                <property name="title">Import Connections</property>
                <property name="accelerator">&lt;Ctrl&gt;i</property>
              </object>
            </child>
            <child>
              <object class="GtkShortcutsShortcut">
                <property name="title">Export Connections</property>
                <property name="accelerator">&lt;Ctrl&gt;e</property>
              </object>
            </child>
            <child>
              <object class="GtkShortcutsShortcut">
                <property name="title">Refresh Reachability / Status</property>
                <property name="accelerator">F5</property>
              </object>
            </child>
            <child>
              <object class="GtkShortcutsShortcut">
                <property name="title">Quit VER</property>
                <property name="accelerator">&lt;Ctrl&gt;q</property>
              </object>
            </child>
          </object>
        </child>
        <child>
          <object class="GtkShortcutsGroup">
            <property name="title">Connection Actions</property>
            <child>
              <object class="GtkShortcutsShortcut">
                <property name="title">Launch Selected Connection</property>
                <property name="accelerator">Return</property>
              </object>
            </child>
            <child>
              <object class="GtkShortcutsShortcut">
                <property name="title">Delete Selected Connection</property>
                <property name="accelerator">Delete</property>
              </object>
            </child>
            <child>
              <object class="GtkShortcutsShortcut">
                <property name="title">Keyboard Shortcuts Cheat Sheet</property>
                <property name="accelerator">&lt;Ctrl&gt;question</property>
              </object>
            </child>
          </object>
        </child>
      </object>
    </child>
  </object>
</interface>
"#;

pub struct ShortcutsDialog;

impl ShortcutsDialog {
    pub fn show(parent: &impl IsA<gtk::Window>) {
        let builder = gtk::Builder::from_string(SHORTCUTS_UI);
        if let Some(shortcuts_win) = builder.object::<gtk::ShortcutsWindow>("shortcuts_window") {
            shortcuts_win.set_transient_for(Some(parent));
            shortcuts_win.present();
        }
    }
}
