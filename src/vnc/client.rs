use std::net::SocketAddr;
use std::time::Duration;
use anyhow::{anyhow, Result};
use gtk::glib;

use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use vnc::{VncConnector, Rect, PixelFormat, VncEvent};

use crate::models::VncScaling;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VncFrameUpdate {
    pub width: u32,
    pub height: u32,
    pub stride: usize,
    pub pixels: Vec<u8>, // B8G8R8X8 format for gdk::MemoryTexture
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VncEventLocal {
    Key { keysym: u32, down: bool },
    Pointer { x: u16, y: u16, mask: u8 },
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
    pub scaling: VncScaling,
    pub encoding: crate::models::VncEncodingOption,
}

impl VncClient {
    pub fn new(host: String, port: u16, scaling: VncScaling, encoding: crate::models::VncEncodingOption) -> Self {
        Self { host, port, scaling, encoding }
    }

    pub fn process_frame_buffer(&self, raw_rgb: &[u8], width: u32, height: u32) -> VncFrameUpdate {
        let stride = (width * 4) as usize;
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        
        // Convert RGB to BGRA / B8G8R8X8 format for GdkMemoryTexture
        let total_pixels = (width * height) as usize;
        for i in 0..total_pixels {
            let src_idx = i * 3;
            let dst_idx = i * 4;
            if src_idx + 2 < raw_rgb.len() {
                pixels[dst_idx] = raw_rgb[src_idx + 2];     // B
                pixels[dst_idx + 1] = raw_rgb[src_idx + 1]; // G
                pixels[dst_idx + 2] = raw_rgb[src_idx];     // R
                pixels[dst_idx + 3] = 0xFF;                 // X/A
            }
        }

        VncFrameUpdate {
            width,
            height,
            stride,
            pixels,
        }
    }

    pub fn start_session(
        &self,
        password: Option<String>,
        glib_tx: glib::Sender<VncSessionEvent>,
    ) -> UnboundedSender<VncCommand> {
        let (cmd_tx, cmd_rx) = unbounded_channel();
        let host = self.host.clone();
        let port = self.port;

        let encoding = self.encoding.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                let mut session = VncSession::new(host, port, password, glib_tx, cmd_rx, encoding);
                if let Err(e) = session.run().await {
                    let _ = session.glib_tx.send(VncSessionEvent::Error(e.to_string()));
                }
            });
        });

        cmd_tx
    }
}

struct VncSession {
    host: String,
    port: u16,
    password: Option<String>,
    glib_tx: glib::Sender<VncSessionEvent>,
    cmd_rx: UnboundedReceiver<VncCommand>,
    encoding: crate::models::VncEncodingOption,
    backing_buffer: Vec<u8>,
    width: u32,
    height: u32,
    pixel_format: Option<PixelFormat>,
}

impl VncSession {
    fn new(
        host: String,
        port: u16,
        password: Option<String>,
        glib_tx: glib::Sender<VncSessionEvent>,
        cmd_rx: UnboundedReceiver<VncCommand>,
        encoding: crate::models::VncEncodingOption,
    ) -> Self {
        Self {
            host,
            port,
            password,
            glib_tx,
            cmd_rx,
            encoding,
            backing_buffer: Vec::new(),
            width: 0,
            height: 0,
            pixel_format: None,
        }
    }

    async fn run(&mut self) -> Result<()> {
        let addr_str = format!("{}:{}", self.host, self.port);
        let socket_addr: SocketAddr = addr_str.parse()
            .map_err(|e| anyhow!("Invalid VNC host/port address {}: {}", addr_str, e))?;

        let stream = tokio::time::timeout(
            Duration::from_secs(10),
            tokio::net::TcpStream::connect(&socket_addr)
        ).await.map_err(|_| anyhow!("Connection timed out"))?
         .map_err(|e| anyhow!("VNC TCP connection failed: {}", e))?;

        let pwd_opt = self.password.clone();
        let auth_callback = async move {
            Ok(pwd_opt.unwrap_or_else(|| String::new()))
        };
        let mut connector = VncConnector::new(stream)
            .set_auth_method(auth_callback)
            .set_pixel_format(vnc::PixelFormat::bgra());
        
        match self.encoding {
            crate::models::VncEncodingOption::Tight => {
                connector = connector.add_encoding(vnc::VncEncoding::Tight);
            }
            crate::models::VncEncodingOption::Zrle => {
                connector = connector.add_encoding(vnc::VncEncoding::Zrle);
            }
            crate::models::VncEncodingOption::Raw => {
                connector = connector.add_encoding(vnc::VncEncoding::Raw);
            }
            crate::models::VncEncodingOption::Auto => {
                connector = connector
                    .add_encoding(vnc::VncEncoding::Tight)
                    .add_encoding(vnc::VncEncoding::Zrle)
                    .add_encoding(vnc::VncEncoding::CopyRect)
                    .add_encoding(vnc::VncEncoding::Raw);
            }
        }
        
        // ALWAYS add CursorPseudo and DesktopSizePseudo so we don't get ghost cursors
        connector = connector
            .add_encoding(vnc::VncEncoding::CursorPseudo)
            .add_encoding(vnc::VncEncoding::DesktopSizePseudo);
        
        let client = connector.build()
            .map_err(|e| anyhow!("RFB build failed: {}", e))?
            .try_start().await
            .map_err(|e| anyhow!("RFB handshake failed: {}", e))?
            .finish()
            .map_err(|e| anyhow!("RFB finish failed: {}", e))?;

        let mut running = true;
        let mut refresh_interval = tokio::time::interval(Duration::from_millis(16)); // ~60 fps
        let mut needs_redraw = false;
        
        while running {
            tokio::select! {
                _ = refresh_interval.tick() => {
                    if needs_redraw {
                        self.dispatch_frame();
                        needs_redraw = false;
                    }
                    let _ = client.input(vnc::X11Event::Refresh).await;
                }
                cmd_opt = self.cmd_rx.recv() => {
                    if let Some(cmd) = cmd_opt {
                        match cmd {
                            VncCommand::KeyEvent { keysym, down } => {
                                let _ = client.input(vnc::X11Event::KeyEvent(vnc::ClientKeyEvent {
                                    keycode: keysym,
                                    down,
                                })).await;
                            }
                            VncCommand::PointerEvent { x, y, mask } => {
                                let _ = client.input(vnc::X11Event::PointerEvent(vnc::ClientMouseEvent {
                                    position_x: x,
                                    position_y: y,
                                    bottons: mask,
                                })).await;
                            }
                            VncCommand::CutText(text) => {
                                let _ = client.input(vnc::X11Event::CopyText(text)).await;
                            }
                            VncCommand::SetScaling(_) => {}
                            VncCommand::Disconnect => {
                                break;
                            }
                        }
                    } else {
                        break;
                    }
                }
                event_res = client.recv_event() => {
                    match event_res {
                        Ok(event) => {
                            match event {
                                VncEvent::SetResolution(screen) => {
                                    self.width = screen.width as u32;
                                    self.height = screen.height as u32;
                                    self.backing_buffer = vec![0u8; (self.width * self.height * 4) as usize];
                                    let _ = self.glib_tx.send(VncSessionEvent::Connected {
                                        width: self.width,
                                        height: self.height,
                                        name: "VNC Session".to_string(), // vnc-rs might not expose name easily, hardcode for now
                                    });
                                    // Send FullRefresh immediately after resolution is known
                                    let _ = client.input(vnc::X11Event::FullRefresh).await;
                                }
                                VncEvent::SetPixelFormat(format) => {
                                    self.pixel_format = Some(format);
                                }
                                VncEvent::RawImage(rect, pixels) => {
                                    // We explicitly forced the server to use BGRA in the connector!
                                    // So we must decode using BGRA, not the server's native format from SetPixelFormat.
                                    self.decode_tile(&rect, &pixels, &vnc::PixelFormat::bgra());
                                    needs_redraw = true;
                                }
                                VncEvent::Copy(dst, src) => {
                                    self.copy_tile(&src, &dst);
                                    needs_redraw = true;
                                }
                                VncEvent::Error(err) => {
                                    let _ = self.glib_tx.send(VncSessionEvent::Error(err));
                                    running = false;
                                }
                                _ => {}
                            }
                        }
                        Err(e) => {
                            let _ = self.glib_tx.send(VncSessionEvent::Disconnected(e.to_string()));
                            running = false;
                        }
                    }
                }
            }
        }
        
        let _ = client.close().await;
        Ok(())
    }

    fn dispatch_frame(&self) {
        let update = VncFrameUpdate {
            width: self.width,
            height: self.height,
            stride: (self.width * 4) as usize,
            pixels: self.backing_buffer.clone(),
        };
        let _ = self.glib_tx.send(VncSessionEvent::FrameUpdate(update));
    }

    fn decode_tile(&mut self, rect: &Rect, tile_bytes: &[u8], format: &PixelFormat) {
        decode_tile_raw(&mut self.backing_buffer, self.width, self.height, rect, tile_bytes, format);
    }

    fn copy_tile(&mut self, src: &Rect, dst: &Rect) {
        copy_tile_raw(&mut self.backing_buffer, self.width, self.height, src, dst);
    }
}

pub fn decode_tile_raw(
    backing_buffer: &mut [u8],
    width: u32,
    height: u32,
    rect: &Rect,
    tile_bytes: &[u8],
    format: &PixelFormat,
) {
    let bpp = format.bits_per_pixel as usize / 8;
    if bpp == 0 { return; }

    let rect_w = rect.width as usize;
    let rect_h = rect.height as usize;
    let fb_w = width as usize;

    for y in 0..rect_h {
        let dst_y = rect.y as usize + y;
        if dst_y >= height as usize { continue; }

        for x in 0..rect_w {
            let dst_x = rect.x as usize + x;
            if dst_x >= fb_w { continue; }

            let src_idx = (y * rect_w + x) * bpp;
            if src_idx + bpp > tile_bytes.len() { continue; }

            let dst_idx = (dst_y * fb_w + dst_x) * 4;

            let (r, g, b) = match bpp {
                4 => {
                    let val = if format.big_endian_flag != 0 {
                        u32::from_be_bytes([tile_bytes[src_idx], tile_bytes[src_idx+1], tile_bytes[src_idx+2], tile_bytes[src_idx+3]])
                    } else {
                        u32::from_le_bytes([tile_bytes[src_idx], tile_bytes[src_idx+1], tile_bytes[src_idx+2], tile_bytes[src_idx+3]])
                    };
                    let red_max = format.red_max.max(1);
                    let green_max = format.green_max.max(1);
                    let blue_max = format.blue_max.max(1);
                    let r = ((((val >> format.red_shift) as u16 & format.red_max) as u32 * 255) / red_max as u32) as u8;
                    let g = ((((val >> format.green_shift) as u16 & format.green_max) as u32 * 255) / green_max as u32) as u8;
                    let b = ((((val >> format.blue_shift) as u16 & format.blue_max) as u32 * 255) / blue_max as u32) as u8;
                    (r, g, b)
                }
                3 => {
                    let r = tile_bytes[src_idx];
                    let g = tile_bytes[src_idx + 1];
                    let b = tile_bytes[src_idx + 2];
                    (r, g, b)
                }
                2 => {
                    let val = if format.big_endian_flag != 0 {
                        u16::from_be_bytes([tile_bytes[src_idx], tile_bytes[src_idx+1]])
                    } else {
                        u16::from_le_bytes([tile_bytes[src_idx], tile_bytes[src_idx+1]])
                    };
                    let red_max = format.red_max.max(1);
                    let green_max = format.green_max.max(1);
                    let blue_max = format.blue_max.max(1);
                    let r = ((((val >> format.red_shift) & format.red_max) as u32 * 255) / red_max as u32) as u8;
                    let g = ((((val >> format.green_shift) & format.green_max) as u32 * 255) / green_max as u32) as u8;
                    let b = ((((val >> format.blue_shift) & format.blue_max) as u32 * 255) / blue_max as u32) as u8;
                    (r, g, b)
                }
                _ => (0, 0, 0),
            };

            if dst_idx + 3 < backing_buffer.len() {
                backing_buffer[dst_idx] = b;        // B
                backing_buffer[dst_idx + 1] = g;    // G
                backing_buffer[dst_idx + 2] = r;    // R
                backing_buffer[dst_idx + 3] = 0xFF; // X / A
            }
        }
    }
}

pub fn copy_tile_raw(
    backing_buffer: &mut [u8],
    width: u32,
    height: u32,
    src: &Rect,
    dst: &Rect,
) {
    let fb_w = width as usize;
    let w = src.width as usize;
    let h = src.height as usize;

    let y_range: Vec<usize> = if dst.y > src.y {
        (0..h).rev().collect()
    } else {
        (0..h).collect()
    };

    let x_range: Vec<usize> = if dst.x > src.x {
        (0..w).rev().collect()
    } else {
        (0..w).collect()
    };

    for y in y_range {
        let sy = src.y as usize + y;
        let dy = dst.y as usize + y;
        if sy >= height as usize || dy >= height as usize { continue; }

        for &x in &x_range {
            let sx = src.x as usize + x;
            let dx = dst.x as usize + x;
            if sx >= fb_w || dx >= fb_w { continue; }

            let src_idx = (sy * fb_w + sx) * 4;
            let dst_idx = (dy * fb_w + dx) * 4;

            if src_idx + 3 < backing_buffer.len() && dst_idx + 3 < backing_buffer.len() {
                backing_buffer.copy_within(src_idx..src_idx+4, dst_idx);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vnc::Rect;

    #[test]
    fn test_copy_tile_horizontal_overlap() {
        #[allow(deprecated)]
        let (glib_tx, _) = glib::MainContext::channel(glib::Priority::default());
        let (_, cmd_rx) = unbounded_channel();
        let mut session = VncSession::new("127.0.0.1".into(), 5900, None, glib_tx, cmd_rx);
        session.width = 4;
        session.height = 1;
        // Backing buffer with 4 pixels: P0, P1, P2, P3
        session.backing_buffer = vec![
            10, 10, 10, 255, // P0
            20, 20, 20, 255, // P1
            30, 30, 30, 255, // P2
            0,  0,  0,  255, // P3
        ];

        // Copy 2 pixels from left=0 to left=1 (P0,P1 -> pos 1,2)
        let src = Rect { x: 0, y: 0, width: 2, height: 1 };
        let dst = Rect { x: 1, y: 0, width: 2, height: 1 };
        session.copy_tile(&src, &dst);

        // Expect P1 becomes P0 (10), P2 becomes P1 (20)
        assert_eq!(&session.backing_buffer[4..8], &[10, 10, 10, 255]);
        assert_eq!(&session.backing_buffer[8..12], &[20, 20, 20, 255]);
    }

    #[test]
    fn test_copy_tile_vertical_overlap() {
        #[allow(deprecated)]
        let (glib_tx, _) = glib::MainContext::channel(glib::Priority::default());
        let (_, cmd_rx) = unbounded_channel();
        let mut session = VncSession::new("127.0.0.1".into(), 5900, None, glib_tx, cmd_rx);
        session.width = 1;
        session.height = 4;
        session.backing_buffer = vec![
            10, 10, 10, 255, // R0
            20, 20, 20, 255, // R1
            30, 30, 30, 255, // R2
            0,  0,  0,  255, // R3
        ];

        let src = Rect { x: 0, y: 0, width: 1, height: 2 };
        let dst = Rect { x: 0, y: 1, width: 1, height: 2 };
        session.copy_tile(&src, &dst);

        assert_eq!(&session.backing_buffer[4..8], &[10, 10, 10, 255]);
        assert_eq!(&session.backing_buffer[8..12], &[20, 20, 20, 255]);
    }
}
