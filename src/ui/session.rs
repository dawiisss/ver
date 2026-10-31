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
use crate::tray;

#[derive(Clone)]
pub struct ActiveSession {
    pub conn_id: String,
    pub conn_name: String,
    pub child: Arc<Mutex<Option<std::process::Child>>>,
    pub log_buffer: gtk::TextBuffer,
    pub is_running: Arc<AtomicBool>,
    /// SSH tunnel the viewer connects through, closed when the session ends.
    pub tunnel: Arc<Mutex<Option<std::process::Child>>>,
}

fn close_tunnel(tunnel: &Mutex<Option<std::process::Child>>) {
    if let Some(child) = tunnel.lock().unwrap().take() {
        std::thread::spawn(move || launcher::close_tunnel(child));
    }
}

pub enum ExternalSessionEvent {
    Log(String),
    /// `handed_off`: an SSH terminal launcher exited at once after passing the session to an
    /// already running terminal server, so the session can no longer be tracked.
    Exit {
        success: bool,
        handed_off: bool,
    },
}

/// Terminal launchers that hand off to a server process exit within this time.
const TERMINAL_HANDOFF_WINDOW: std::time::Duration = std::time::Duration::from_secs(3);

const MAX_LOG_CHUNK_BYTES: u64 = 8192;
const MAX_LOG_CHARS: i32 = 256 * 1024;
const MAX_LOG_LINES: i32 = 2000;

fn stream_output(reader: impl std::io::Read, tx: async_channel::Sender<ExternalSessionEvent>) {
    use std::io::{BufRead, BufReader, Read};
    let mut reader = BufReader::new(reader);
    loop {
        // Limit even output with no newline, rather than allocating an entire line.
        let mut chunk = Vec::new();
        match reader
            .by_ref()
            .take(MAX_LOG_CHUNK_BYTES)
            .read_until(b'\n', &mut chunk)
        {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                let text = String::from_utf8_lossy(&chunk).into_owned();
                if tx.send_blocking(ExternalSessionEvent::Log(text)).is_err() {
                    break;
                }
            }
        }
    }
}

fn append_log(buffer: &gtk::TextBuffer, message: &str) {
    buffer.insert(&mut buffer.end_iter(), message);
    let excess = buffer.char_count() - MAX_LOG_CHARS;
    if excess > 0 {
        buffer.delete(&mut buffer.start_iter(), &mut buffer.iter_at_offset(excess));
    }
    if buffer.line_count() > MAX_LOG_LINES {
        if let Some(mut end) = buffer.iter_at_line(buffer.line_count() - MAX_LOG_LINES) {
            buffer.delete(&mut buffer.start_iter(), &mut end);
        }
    }
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
        let tunnel = session.tunnel.clone();
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
            close_tunnel(&tunnel);
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

#[allow(clippy::too_many_arguments)]
pub fn track_external_session(
    mut child: std::process::Child,
    mut tunnel: Option<std::process::Child>,
    conn: Connection,
    state: Rc<RefCell<AppWindowState>>,
    list_box: &gtk::ListBox,
    stack: &gtk::Stack,
    container: &gtk::Box,
    toast_overlay: &adw::ToastOverlay,
) {
    let conn_id = conn.id.clone();
    let is_ssh = conn.protocol == Protocol::Ssh;
    let conn_name = conn.name.clone();
    let log_buffer = gtk::TextBuffer::new(None);
    log_buffer.set_enable_undo(false);
    let is_running = Arc::new(AtomicBool::new(true));

    let stdout_pipe = child.stdout.take();
    let stderr_pipe = child.stderr.take();
    let tunnel_stderr = tunnel.as_mut().and_then(|t| t.stderr.take());
    let child_arc = Arc::new(Mutex::new(Some(child)));
    let tunnel_arc = Arc::new(Mutex::new(tunnel));

    let active_session = ActiveSession {
        conn_id: conn_id.clone(),
        conn_name: conn_name.clone(),
        child: child_arc.clone(),
        log_buffer: log_buffer.clone(),
        is_running: is_running.clone(),
        tunnel: tunnel_arc.clone(),
    };

    state
        .borrow_mut()
        .active_sessions
        .insert(conn_id.clone(), active_session);

    set_row_active_badge(list_box, &conn_id, true);

    let (tx, rx) = async_channel::bounded::<ExternalSessionEvent>(256);

    let tx_out = tx.clone();
    if let Some(stdout) = stdout_pipe {
        std::thread::spawn(move || stream_output(stdout, tx_out));
    }

    let tx_err = tx.clone();
    if let Some(stderr) = stderr_pipe {
        std::thread::spawn(move || stream_output(stderr, tx_err));
    }

    let tx_tunnel = tx.clone();
    if let Some(stderr) = tunnel_stderr {
        std::thread::spawn(move || stream_output(stderr, tx_tunnel));
    }

    let child_arc_for_wait = child_arc.clone();
    let is_running_for_wait = is_running.clone();
    let tx_exit = tx.clone();
    let started = std::time::Instant::now();

    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(500));
        if !is_running_for_wait.load(Ordering::SeqCst) {
            break;
        }
        let exit = {
            let mut opt = child_arc_for_wait.lock().unwrap();
            match opt.as_mut() {
                Some(child) => match child.try_wait() {
                    Ok(Some(status)) => Some(status.success()),
                    _ => None,
                },
                None => break,
            }
        };
        if let Some(success) = exit {
            is_running_for_wait.store(false, Ordering::SeqCst);
            close_tunnel(&tunnel_arc);
            // Never hold the child mutex while a bounded log queue may be full.
            let handed_off = is_ssh && success && started.elapsed() < TERMINAL_HANDOFF_WINDOW;
            let _ = tx_exit.send_blocking(ExternalSessionEvent::Exit {
                success,
                handed_off,
            });
            break;
        }
    });

    let buffer = log_buffer.clone();
    let list_box_for_exit = list_box.clone();
    let state_for_exit = state.clone();
    let stack_for_exit = stack.clone();
    let conn_id_for_exit = conn_id.clone();
    let child_for_exit = child_arc.clone();
    let toast_for_exit = toast_overlay.clone();

    glib::MainContext::default().spawn_local(async move {
        let mut processed = 0;
        while let Ok(event) = rx.recv().await {
            match event {
                ExternalSessionEvent::Log(msg) => {
                    append_log(&buffer, &msg);
                }
                ExternalSessionEvent::Exit {
                    success,
                    handed_off,
                } => {
                    let current = state_for_exit
                        .borrow()
                        .active_sessions
                        .get(&conn_id_for_exit)
                        .map(|session| Arc::ptr_eq(&session.child, &child_for_exit))
                        .unwrap_or(false);
                    if !current {
                        continue;
                    }
                    state_for_exit
                        .borrow_mut()
                        .active_sessions
                        .remove(&conn_id_for_exit);
                    set_row_active_badge(&list_box_for_exit, &conn_id_for_exit, false);
                    if handed_off {
                        toast_for_exit.add_toast(adw::Toast::new(
                            "SSH opened in an existing terminal window; VER cannot track or disconnect it",
                        ));
                    }

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
            processed += 1;
            if processed % 32 == 0 {
                glib::timeout_future(std::time::Duration::from_millis(1)).await;
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

    let mut config = state.borrow().config.clone();
    config.record_recent(&conn.id);
    match storage::save_config(&config) {
        Ok(()) => {
            tray::update_launch_menu(&config, &state.borrow().connections);
            state.borrow_mut().config = config;
        }
        Err(error) => toast_overlay.add_toast(adw::Toast::new(&format!(
            "Could not save last connection: {:#}",
            error
        ))),
    }

    if conn.uses_ssh_tunnel() {
        launch_through_tunnel(conn, pass, state, toast_overlay, list_box, stack, container);
        return;
    }

    match launch_viewer(&conn, &pass) {
        Ok(child) => {
            track_external_session(
                child,
                None,
                conn,
                state,
                list_box,
                stack,
                container,
                toast_overlay,
            );
        }
        Err(err) => {
            toast_overlay.add_toast(adw::Toast::new(&format!("Connection failed: {}", err)));
        }
    }
}

fn launch_viewer(conn: &Connection, pass: &str) -> Result<std::process::Child, String> {
    let pass_opt = Some(pass).filter(|p| !p.is_empty());
    match conn.protocol {
        Protocol::Rdp | Protocol::Xrdp => launcher::launch_rdp(conn, pass_opt),
        Protocol::Ssh => launcher::launch_ssh(conn),
        Protocol::Spice => launcher::launch_spice(conn, pass_opt),
        Protocol::Vnc => launcher::launch_vnc(conn, pass_opt),
    }
}

/// Open the SSH tunnel on a worker thread, then start the viewer against its local port.
fn launch_through_tunnel(
    conn: Connection,
    pass: String,
    state: Rc<RefCell<AppWindowState>>,
    toast_overlay: &adw::ToastOverlay,
    list_box: &gtk::ListBox,
    stack: &gtk::Stack,
    container: &gtk::Box,
) {
    if !state.borrow_mut().pending_tunnels.insert(conn.id.clone()) {
        toast_overlay.add_toast(adw::Toast::new(&format!(
            "The SSH tunnel for '{}' is still opening",
            conn.name
        )));
        return;
    }
    toast_overlay.add_toast(adw::Toast::new(&format!(
        "Opening SSH tunnel through {}…",
        conn.advanced_settings.ssh_tunnel_host.trim()
    )));

    let (tx, rx) = async_channel::bounded(1);
    let tunnel_conn = conn.clone();
    std::thread::spawn(move || {
        let _ = tx.send_blocking(launcher::open_ssh_tunnel(&tunnel_conn));
    });

    let toast_overlay = toast_overlay.clone();
    let list_box = list_box.clone();
    let stack = stack.clone();
    let container = container.clone();
    glib::MainContext::default().spawn_local(async move {
        let result = rx
            .recv()
            .await
            .unwrap_or_else(|_| Err("The SSH tunnel worker stopped unexpectedly".into()));
        state.borrow_mut().pending_tunnels.remove(&conn.id);
        let tunnel = match result {
            Ok(tunnel) => tunnel,
            Err(err) => {
                toast_overlay.add_toast(adw::Toast::new(&format!("Connection failed: {}", err)));
                return;
            }
        };
        match launch_viewer(&launcher::through_tunnel(&conn, tunnel.local_port), &pass) {
            Ok(child) => track_external_session(
                child,
                Some(tunnel.child),
                conn,
                state,
                &list_box,
                &stack,
                &container,
                &toast_overlay,
            ),
            Err(err) => {
                std::thread::spawn(move || launcher::close_tunnel(tunnel.child));
                toast_overlay.add_toast(adw::Toast::new(&format!("Connection failed: {}", err)));
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retained_log_text_is_limited_by_characters_and_lines() {
        // TextBuffer is a non-display GObject and its Default constructor works headlessly.
        let buffer = gtk::TextBuffer::default();
        buffer.set_enable_undo(false);
        append_log(&buffer, &"é".repeat(MAX_LOG_CHARS as usize + 100));
        assert_eq!(buffer.char_count(), MAX_LOG_CHARS);
        for number in 0..MAX_LOG_LINES + 100 {
            append_log(&buffer, &format!("line {}\n", number));
        }
        assert!(buffer.line_count() <= MAX_LOG_LINES);
        assert!(buffer.char_count() <= MAX_LOG_CHARS);
        assert!(buffer
            .text(&buffer.start_iter(), &buffer.end_iter(), false)
            .ends_with("line 2099\n"));
        assert!(!buffer.can_undo());
    }

    #[test]
    fn output_without_newlines_is_streamed_in_bounded_chunks() {
        let input = vec![b'x'; 2 * 1024 * 1024];
        let size = input.len();
        let (tx, rx) = async_channel::bounded(2);
        let worker = std::thread::spawn(move || stream_output(std::io::Cursor::new(input), tx));
        let mut received = 0;
        while let Ok(ExternalSessionEvent::Log(message)) = rx.recv_blocking() {
            assert!(message.len() <= MAX_LOG_CHUNK_BYTES as usize);
            received += message.len();
        }
        worker.join().unwrap();
        assert_eq!(received, size);
    }

    #[test]
    fn invalid_utf8_does_not_discard_subsequent_output() {
        let (tx, rx) = async_channel::bounded(2);
        stream_output(std::io::Cursor::new(b"\xff\nnext line\n"), tx);
        let ExternalSessionEvent::Log(first) = rx.recv_blocking().unwrap() else {
            panic!()
        };
        let ExternalSessionEvent::Log(next) = rx.recv_blocking().unwrap() else {
            panic!()
        };
        assert!(first.contains('�'));
        assert_eq!(next, "next line\n");
    }
}
