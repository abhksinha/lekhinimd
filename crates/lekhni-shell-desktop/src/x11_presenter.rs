//! Pure Rust zero-dependency X11 window presenter.
//!
//! Follows Section 8 & 14 of LEKHNI_ARCHITECTURE:
//! Direct native presentation over the X11 Unix domain socket (/tmp/.X11-unix/X0)
//! without depending on C libraries or heavy external runtimes.

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;

pub struct X11Window {
    stream: UnixStream,
    pub window_id: u32,
    pub gc_id: u32,
    pub width: u16,
    pub height: u16,
}

impl X11Window {
    /// Connects to the local X11 display socket and opens a native window.
    pub fn open(title: &str, width: u16, height: u16) -> Result<Self, String> {
        let display = std::env::var("DISPLAY").unwrap_or_else(|_| ":0".into());
        let socket_num = display.trim_start_matches(':').split('.').next().unwrap_or("0");
        let socket_path = format!("/tmp/.X11-unix/X{}", socket_num);

        let mut stream = UnixStream::connect(&socket_path)
            .map_err(|e| format!("Failed to connect to X11 socket {}: {}", socket_path, e))?;

        // 1. Handshake setup
        let (auth_name, auth_data) = read_xauthority().unwrap_or_default();
        let name_pad = (4 - (auth_name.len() % 4)) % 4;
        let data_pad = (4 - (auth_data.len() % 4)) % 4;

        let mut setup_req = Vec::new();
        setup_req.push(b'l'); // Little-endian
        setup_req.push(0); // Unused
        setup_req.extend_from_slice(&11u16.to_le_bytes()); // Major version 11
        setup_req.extend_from_slice(&0u16.to_le_bytes()); // Minor version 0
        setup_req.extend_from_slice(&(auth_name.len() as u16).to_le_bytes());
        setup_req.extend_from_slice(&(auth_data.len() as u16).to_le_bytes());
        setup_req.extend_from_slice(&[0, 0]); // Unused
        setup_req.extend_from_slice(&auth_name);
        setup_req.extend_from_slice(&vec![0u8; name_pad]);
        setup_req.extend_from_slice(&auth_data);
        setup_req.extend_from_slice(&vec![0u8; data_pad]);

        stream.write_all(&setup_req).map_err(|e| e.to_string())?;

        // Read setup reply
        let mut reply_hdr = [0u8; 8];
        stream.read_exact(&mut reply_hdr).map_err(|e| e.to_string())?;
        if reply_hdr[0] != 1 {
            return Err("X11 connection rejected by server".into());
        }

        let additional_len = u16::from_le_bytes([reply_hdr[6], reply_hdr[7]]) as usize * 4;
        let mut reply_body = vec![0u8; additional_len];
        stream.read_exact(&mut reply_body).map_err(|e| e.to_string())?;

        // Parse resource ID base and mask
        let res_id_base = u32::from_le_bytes(reply_body[4..8].try_into().unwrap());
        let vendor_len = u16::from_le_bytes([reply_body[16], reply_body[17]]) as usize;
        let num_formats = reply_body[21] as usize;
        let vendor_pad = (4 - (vendor_len % 4)) % 4;

        let screen_offset = 32 + vendor_len + vendor_pad + (num_formats * 8);
        let root_window = u32::from_le_bytes(reply_body[screen_offset..screen_offset + 4].try_into().unwrap());
        let root_visual = u32::from_le_bytes(reply_body[screen_offset + 32..screen_offset + 36].try_into().unwrap());

        let window_id = res_id_base | 1;
        let gc_id = res_id_base | 2;

        // 2. CreateWindow (Opcode 1)
        // Request length = 8 (header + fields) + 2 (value mask entries: background + event-mask)
        let mut create_win = Vec::new();
        create_win.push(1u8); // CreateWindow opcode
        create_win.push(0u8); // Depth = copy from parent
        create_win.extend_from_slice(&10u16.to_le_bytes()); // 10 * 4 bytes total
        create_win.extend_from_slice(&window_id.to_le_bytes());
        create_win.extend_from_slice(&root_window.to_le_bytes());
        create_win.extend_from_slice(&0i16.to_le_bytes()); // X
        create_win.extend_from_slice(&0i16.to_le_bytes()); // Y
        create_win.extend_from_slice(&width.to_le_bytes());
        create_win.extend_from_slice(&height.to_le_bytes());
        create_win.extend_from_slice(&0u16.to_le_bytes()); // Border width
        create_win.extend_from_slice(&1u16.to_le_bytes()); // InputOutput class
        create_win.extend_from_slice(&root_visual.to_le_bytes());
        // Value mask: CWBackPixel (0x02) | CWEventMask (0x800) = 0x802
        create_win.extend_from_slice(&0x802u32.to_le_bytes());
        create_win.extend_from_slice(&0xFF_1E_1E_1Eu32.to_le_bytes()); // Background color
        // Event mask: KeyPress (0x01) | ButtonPress (0x04) | ButtonRelease (0x08) | PointerMotion (0x40) | Exposure (0x8000) | StructureNotify (0x20000)
        create_win.extend_from_slice(&0x2804Du32.to_le_bytes());

        stream.write_all(&create_win).map_err(|e| e.to_string())?;

        // 3. Set Window Title (ChangeProperty opcode 18: WM_NAME)
        let mut title_req = Vec::new();
        let title_bytes = title.as_bytes();
        let title_pad = (4 - (title_bytes.len() % 4)) % 4;
        let title_units = 6 + (title_bytes.len() + title_pad) / 4;
        title_req.push(18u8); // ChangeProperty
        title_req.push(0u8); // Replace mode
        title_req.extend_from_slice(&(title_units as u16).to_le_bytes());
        title_req.extend_from_slice(&window_id.to_le_bytes());
        title_req.extend_from_slice(&39u32.to_le_bytes()); // WM_NAME atom (39)
        title_req.extend_from_slice(&31u32.to_le_bytes()); // STRING atom (31)
        title_req.push(8); // 8-bit format
        title_req.extend_from_slice(&[0, 0, 0]); // Unused
        title_req.extend_from_slice(&(title_bytes.len() as u32).to_le_bytes());
        title_req.extend_from_slice(title_bytes);
        title_req.extend_from_slice(&vec![0u8; title_pad]);

        stream.write_all(&title_req).map_err(|e| e.to_string())?;

        // 4. CreateGC (Opcode 55)
        let mut gc_req = Vec::new();
        gc_req.push(55u8); // CreateGC
        gc_req.push(0);
        gc_req.extend_from_slice(&4u16.to_le_bytes()); // 4 * 4 bytes
        gc_req.extend_from_slice(&gc_id.to_le_bytes());
        gc_req.extend_from_slice(&window_id.to_le_bytes());
        gc_req.extend_from_slice(&0u32.to_le_bytes()); // Value mask = 0

        stream.write_all(&gc_req).map_err(|e| e.to_string())?;

        // 5. MapWindow (Opcode 8)
        let mut map_req = Vec::new();
        map_req.push(8u8); // MapWindow
        map_req.push(0);
        map_req.extend_from_slice(&2u16.to_le_bytes()); // 2 * 4 bytes
        map_req.extend_from_slice(&window_id.to_le_bytes());
        stream.write_all(&map_req).map_err(|e| e.to_string())?;

        // 6. RaiseWindow: ConfigureWindow (Opcode 12) with stack_mode = Above (0)
        let mut raise_req = Vec::new();
        raise_req.push(12u8); // ConfigureWindow
        raise_req.push(0);
        raise_req.extend_from_slice(&4u16.to_le_bytes()); // 4 * 4 = 16 bytes
        raise_req.extend_from_slice(&window_id.to_le_bytes());
        raise_req.extend_from_slice(&0x0040u16.to_le_bytes()); // value-mask: CWStackMode (0x0040)
        raise_req.extend_from_slice(&[0, 0]); // pad
        raise_req.extend_from_slice(&0u32.to_le_bytes()); // Above = 0
        stream.write_all(&raise_req).map_err(|e| e.to_string())?;

        // 7. SetInputFocus (Opcode 42)
        let mut focus_req = Vec::new();
        focus_req.push(42u8); // SetInputFocus
        focus_req.push(2u8); // RevertToParent
        focus_req.extend_from_slice(&3u16.to_le_bytes()); // 3 * 4 = 12 bytes
        focus_req.extend_from_slice(&window_id.to_le_bytes());
        focus_req.extend_from_slice(&0u32.to_le_bytes()); // CurrentTime = 0
        stream.write_all(&focus_req).map_err(|e| e.to_string())?;
        stream.flush().map_err(|e| e.to_string())?;

        Ok(Self {
            stream,
            window_id,
            gc_id,
            width,
            height,
        })
    }

    /// Blits 32-bit ARGB/XRGB pixels into the X11 window.
    pub fn present_framebuffer(&mut self, pixels: &[u32], width: u16, height: u16) -> Result<(), String> {
        let chunk_h = 32u16;
        let mut y = 0u16;

        while y < height {
            let current_h = chunk_h.min(height - y);
            let start_pixel = (y as usize) * (width as usize);
            let pixel_count = (current_h as usize) * (width as usize);
            let slice = &pixels[start_pixel..start_pixel + pixel_count];

            let pixel_bytes: &[u8] = unsafe {
                std::slice::from_raw_parts(slice.as_ptr() as *const u8, pixel_count * 4)
            };

            let total_words = 6 + (pixel_bytes.len() / 4);
            let mut req = Vec::with_capacity(24 + pixel_bytes.len());
            req.push(72u8); // PutImage
            req.push(2u8); // ZPixmap format
            req.extend_from_slice(&(total_words as u16).to_le_bytes());
            req.extend_from_slice(&self.window_id.to_le_bytes());
            req.extend_from_slice(&self.gc_id.to_le_bytes());
            req.extend_from_slice(&width.to_le_bytes());
            req.extend_from_slice(&current_h.to_le_bytes());
            req.extend_from_slice(&0i16.to_le_bytes()); // dst X
            req.extend_from_slice(&(y as i16).to_le_bytes()); // dst Y
            req.push(0); // Left pad
            req.push(24); // Depth 24-bit
            req.extend_from_slice(&[0, 0]); // Unused
            req.extend_from_slice(pixel_bytes);

            self.stream.write_all(&req).map_err(|e| e.to_string())?;
            y += current_h;
        }

        self.stream.flush().map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Reads the next X11 event packet (blocking with zero CPU idle usage).
    pub fn wait_event(&mut self) -> Result<[u8; 32], std::io::Error> {
        self.stream.set_nonblocking(false)?;
        let mut event_buf = [0u8; 32];
        self.stream.read_exact(&mut event_buf)?;
        Ok(event_buf)
    }

    /// Polls for incoming X11 event packets (non-blocking).
    pub fn poll_event(&mut self) -> Option<u8> {
        self.stream.set_nonblocking(true).ok()?;
        let mut event_buf = [0u8; 32];
        match self.stream.read_exact(&mut event_buf) {
            Ok(_) => Some(event_buf[0] & 0x7F), // Mask off high bit (synthetic flag)
            Err(_) => None,
        }
    }
}

/// Reads X11 MIT-MAGIC-COOKIE-1 authentication credentials from ~/.Xauthority.
fn read_xauthority() -> Option<(Vec<u8>, Vec<u8>)> {
    let home = std::env::var("HOME").ok()?;
    let path = format!("{}/.Xauthority", home);
    let bytes = std::fs::read(&path).ok()?;

    let mut i = 0;
    while i + 4 < bytes.len() {
        // Family (2 bytes BE)
        i += 2;
        if i >= bytes.len() { break; }
        // Address
        let addr_len = u16::from_be_bytes([bytes[i], bytes[i + 1]]) as usize;
        i += 2 + addr_len;
        if i >= bytes.len() { break; }
        // Number
        let num_len = u16::from_be_bytes([bytes[i], bytes[i + 1]]) as usize;
        i += 2 + num_len;
        if i >= bytes.len() { break; }
        // Auth name
        let name_len = u16::from_be_bytes([bytes[i], bytes[i + 1]]) as usize;
        i += 2;
        let name = bytes[i..i + name_len].to_vec();
        i += name_len;
        if i >= bytes.len() { break; }
        // Auth data
        let data_len = u16::from_be_bytes([bytes[i], bytes[i + 1]]) as usize;
        i += 2;
        let data = bytes[i..i + data_len].to_vec();
        i += data_len;

        if name == b"MIT-MAGIC-COOKIE-1" {
            return Some((name, data));
        }
    }
    None
}
