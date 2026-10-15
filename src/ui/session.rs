use gtk::glib;
use gtk::prelude::*;
use libadwaita as adw;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use super::sidebar::set_row_active_badge;
use super::window::AppWindowState;
use crate::launcher;
use crate::models::{Connection, Protocol};
use crate::storage;

#[derive(Clone)]
pub struct ActiveSession {
    pub conn_id: String,
    pub conn_name: String,
    pub child: Arc<Mutex<Option<std::process::Child>>>,
    pub log_buffer: gtk::TextBuffer,
    pub is_running: Arc<AtomicBool>,
}

pub enum ExternalSessionEvent {
    Log(String),
    Exit(bool),
}

pub fn show_external_session(
    container: &gtk::Box,
    stack: &gtk::Stack,
    state: Rc<RefCell<AppWindowState>>,
    list_box: &gtk::ListBox,
    conn_id: &str,
) {
    let session_opt = state.borrow().active_sessions.get(conn_id).cloned();
    if let Some(session) = session_opt {
        while let Some(c) = container.first_child() {
            container.remove(&c);
        }

        let title = gtk::Label::builder()
            .label(format!("External Session Active: {}", session.conn_name))
            .css_classes(vec!["title-2"])
            .margin_top(48)
            .margin_bottom(12)
            .build();

        let button_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(12)
            .halign(gtk::Align::Center)
            .margin_bottom(24)
            .build();

        let btn_disconnect = gtk::Button::builder()
            .label("Disconnect")
            .css_classes(vec!["destructive-action", "pill"])
            .build();

        let btn_view_settings = gtk::Button::builder()
            .label("View Settings")
            .css_classes(vec!["flat", "pill"])
            .build();

        button_box.append(&btn_disconnect);
        button_box.append(&btn_view_settings);

        let log_view = gtk::TextView::builder()
            .buffer(&session.log_buffer)
            .editable(false)
            .cursor_visible(false)
            .monospace(true)
            .css_classes(vec!["card", "view"])
            .left_margin(12)
            .right_margin(12)
            .top_margin(12)
            .bottom_margin(12)
            .wrap_mode(gtk::WrapMode::WordChar)
            .build();

        let scroll = gtk::ScrolledWindow::builder()
            .child(&log_view)
            .vexpand(true)
            .hexpand(true)
            .min_content_height(300)
            .margin_start(24)
            .margin_end(24)
            .margin_bottom(24)
            .build();

        let child_arc = session.child.clone();
        let is_running = session.is_running.clone();
        let stack_for_btn = stack.clone();
        let state_for_btn = state.clone();
        let list_box_for_btn = list_box.clone();
        let id_for_btn = conn_id.to_string();

        btn_disconnect.connect_clicked(move |_| {
            is_running.store(false, Ordering::SeqCst);
            let mut opt = child_arc.lock().unwrap();
            if let Some(mut c) = opt.take() {
                std::thread::spawn(move || {
                    let pid = c.id();
                    #[cfg(unix)]
                    {
                        if pid > 1 && c.try_wait().ok().flatten().is_none() {
                            unsafe {
                                libc::kill(-(pid as i32), libc::SIGTERM);
                            }
                        }
                    }

                    let mut exited = false;
                    for _ in 0..20 {
                        if let Ok(Some(_)) = c.try_wait() {
                            exited = true;
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }

                    if !exited {
                        #[cfg(unix)]
                        {
                            if pid > 1 && c.try_wait().ok().flatten().is_none() {
                                unsafe {
                                    libc::kill(-(pid as i32), libc::SIGKILL);
                                }
                            }
                        }
                        let _ = c.kill();
                        let _ = c.wait();
                    }
                });
            }
            state_for_btn
                .borrow_mut()
                .active_sessions
                .remove(&id_for_btn);
            set_row_active_badge(&list_box_for_btn, &id_for_btn, false);
            stack_for_btn.set_visible_child_name("editor");
        });

        let stack_for_settings = stack.clone();
        btn_view_settings.connect_clicked(move |_| {
            stack_for_settings.set_visible_child_name("editor");
        });

        container.append(&title);
        container.append(&button_box);
        container.append(&scroll);
        stack.set_visible_child_name("external_session");
    }
}

pub fn track_external_session(
    mut child: std::process::Child,
    conn: Connection,
    state: Rc<RefCell<AppWindowState>>,
    list_box: &gtk::ListBox,
    stack: &gtk::Stack,
    container: &gtk::Box,
) {
    let conn_id = conn.id.clone();
    let conn_name = conn.name.clone();
    let log_buffer = gtk::TextBuffer::new(None);
    let is_running = Arc::new(AtomicBool::new(true));

    let stdout_pipe = child.stdout.take();
    let stderr_pipe = child.stderr.take();
    let child_arc = Arc::new(Mutex::new(Some(child)));

    let active_session = ActiveSession {
        conn_id: conn_id.clone(),
        conn_name: conn_name.clone(),
        child: child_arc.clone(),
        log_buffer: log_buffer.clone(),
        is_running: is_running.clone(),
    };

    state
        .borrow_mut()
        .active_sessions
        .insert(conn_id.clone(), active_session);

    set_row_active_badge(list_box, &conn_id, true);

    let (tx, rx) = async_channel::unbounded::<ExternalSessionEvent>();

    let tx_out = tx.clone();
    if let Some(stdout) = stdout_pipe {
        std::thread::spawn(move || {
            use std::io::{BufRead, BufReader};
            let reader = BufReader::new(stdout);
            for l in reader.lines().map_while(Result::ok) {
                let _ = tx_out.send_blocking(ExternalSessionEvent::Log(l));
            }
        });
    }

    let tx_err = tx.clone();
    if let Some(stderr) = stderr_pipe {
        std::thread::spawn(move || {
            use std::io::{BufRead, BufReader};
            let reader = BufReader::new(stderr);
            for l in reader.lines().map_while(Result::ok) {
                let _ = tx_err.send_blocking(ExternalSessionEvent::Log(l));
            }
        });
    }

    let child_arc_for_wait = child_arc.clone();
    let is_running_for_wait = is_running.clone();
    let tx_exit = tx.clone();

    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(500));
        if !is_running_for_wait.load(Ordering::SeqCst) {
            break;
        }
        let mut opt = child_arc_for_wait.lock().unwrap();
        if let Some(ref mut c) = *opt {
            if let Ok(Some(status)) = c.try_wait() {
                let success = status.success();
                is_running_for_wait.store(false, Ordering::SeqCst);
                let _ = tx_exit.send_blocking(ExternalSessionEvent::Exit(success));
                break;
            }
        } else {
            is_running_for_wait.store(false, Ordering::SeqCst);
            break;
        }
    });

    let buffer = log_buffer.clone();
    let list_box_for_exit = list_box.clone();
    let state_for_exit = state.clone();
    let stack_for_exit = stack.clone();
    let conn_id_for_exit = conn_id.clone();

    glib::MainContext::default().spawn_local(async move {
        while let Ok(event) = rx.recv().await {
            match event {
                ExternalSessionEvent::Log(msg) => {
                    let mut iter = buffer.end_iter();
                    buffer.insert(&mut iter, &format!("{}\n", msg));
                }
                ExternalSessionEvent::Exit(success) => {
                    state_for_exit
                        .borrow_mut()
                        .active_sessions
                        .remove(&conn_id_for_exit);
                    set_row_active_badge(&list_box_for_exit, &conn_id_for_exit, false);

                    let is_currently_selected = state_for_exit
                        .borrow()
                        .selected_id
                        .as_ref()
                        .map(|id| id == &conn_id_for_exit)
                        .unwrap_or(false);

                    if is_currently_selected && success {
                        stack_for_exit.set_visible_child_name("editor");
                    }
                }
            }
        }
    });

    show_external_session(container, stack, state, list_box, &conn_id);
}

pub fn launch_session(
    conn: Connection,
    pass: String,
    state: Rc<RefCell<AppWindowState>>,
    toast_overlay: &adw::ToastOverlay,
    list_box: &gtk::ListBox,
    stack: &gtk::Stack,
    container: &gtk::Box,
) {
    if state.borrow().active_sessions.contains_key(&conn.id) {
        show_external_session(container, stack, state, list_box, &conn.id);
        toast_overlay.add_toast(adw::Toast::new(&format!(
            "Session for '{}' is already active",
            conn.name
        )));
        return;
    }

    state.borrow_mut().config.last_connected_id = Some(conn.id.clone());
    let _ = storage::save_config(&state.borrow().config);

    let launch_result = match conn.protocol {
        Protocol::Rdp | Protocol::Xrdp => {
            let pass_opt = if pass.is_empty() {
                None
            } else {
                Some(pass.as_str())
            };
            launcher::launch_rdp(&conn, pass_opt)
        }
        Protocol::Ssh => launcher::launch_ssh(&conn),
        Protocol::Spice => {
            let pass_opt = if pass.is_empty() {
                None
            } else {
                Some(pass.as_str())
            };
            launcher::launch_spice(&conn, pass_opt)
        }
        Protocol::Vnc => {
            let pass_opt = if pass.is_empty() {
                None
            } else {
                Some(pass.as_str())
            };
            launcher::launch_vnc(&conn, pass_opt)
        }
    };

    match launch_result {
        Ok(child) => {
            track_external_session(child, conn, state, list_box, stack, container);
        }
        Err(err) => {
            toast_overlay.add_toast(adw::Toast::new(&format!("Connection failed: {}", err)));
        }
    }
}
