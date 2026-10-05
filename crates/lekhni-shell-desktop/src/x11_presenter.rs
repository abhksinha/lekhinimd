//! Pure Rust zero-dependency X11 window presenter.
//!
//! Follows Section 8 & 14 of LEKHNI_ARCHITECTURE:
//! Direct native presentation over the X11 Unix domain socket (/tmp/.X11-unix/X0)
//! without depending on C libraries or heavy external runtimes.

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyAction {
    Char(char),
    Backspace,
    Delete,
    Return,
    Tab,
    Escape,
    Left { shift: bool, ctrl: bool },
    Right { shift: bool, ctrl: bool },
    Up { shift: bool },
    Down { shift: bool },
    Home { shift: bool },
    End { shift: bool },
    PageUp,
    PageDown,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    Save,
    SelectAll,
    NewNotebook,
    NewPage,
    Find,
    Rename,
    DeleteNote,
    ToggleOutline,
    SortNotes,
    Reload,
    F1,
    F2,
    F3,
    F4,
    None,
}

pub struct X11Window {
    stream: UnixStream,
    pub window_id: u32,
    pub gc_id: u32,
    pub width: u16,
    pub height: u16,
    pub min_keycode: u8,
    pub max_keycode: u8,
    pub keysyms_per_keycode: u8,
    pub keysyms: Vec<u32>,
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
        // Event mask: KeyPress (0x01) | ButtonPress (0x04) | ButtonRelease (0x08) | PointerMotion (0x40) | Exposure (0x8000) | StructureNotify (0x20000) | FocusChange (0x200000)
        create_win.extend_from_slice(&0x22804Du32.to_le_bytes());

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

        let min_keycode = reply_body[26];
        let max_keycode = reply_body[27];

        // Query X11 keyboard mapping (Opcode 101: GetKeyboardMapping)
        let count = max_keycode.saturating_sub(min_keycode) + 1;
        let mut keymap_req = Vec::with_capacity(8);
        keymap_req.push(101u8); // GetKeyboardMapping opcode
        keymap_req.push(0);
        keymap_req.extend_from_slice(&2u16.to_le_bytes()); // 2 * 4 = 8 bytes
        keymap_req.push(min_keycode);
        keymap_req.push(count);
        keymap_req.extend_from_slice(&[0, 0]);

        stream.write_all(&keymap_req).map_err(|e| e.to_string())?;
        stream.flush().map_err(|e| e.to_string())?;

        let mut km_hdr = [0u8; 32];
        stream.read_exact(&mut km_hdr).map_err(|e| e.to_string())?;
        let keysyms_per_keycode = km_hdr[1];
        let km_words = u32::from_le_bytes(km_hdr[4..8].try_into().unwrap()) as usize;
        let mut km_bytes = vec![0u8; km_words * 4];
        stream.read_exact(&mut km_bytes).map_err(|e| e.to_string())?;

        let mut keysyms = Vec::with_capacity(km_bytes.len() / 4);
        for chunk in km_bytes.chunks_exact(4) {
            keysyms.push(u32::from_le_bytes(chunk.try_into().unwrap()));
        }

        Ok(Self {
            stream,
            window_id,
            gc_id,
            width,
            height,
            min_keycode,
            max_keycode,
            keysyms_per_keycode,
            keysyms,
        })
    }

    /// Translates raw X11 keycode and modifier state into a typed Unicode char or editor navigation action.
    pub fn translate_key(&self, keycode: u8, state: u16) -> KeyAction {
        if keycode < self.min_keycode || keycode > self.max_keycode || self.keysyms_per_keycode == 0 {
            return KeyAction::None;
        }

        let shift = (state & 0x0001) != 0;
        let lock = (state & 0x0002) != 0;
        let ctrl = (state & 0x0004) != 0;

        let base_idx = (keycode - self.min_keycode) as usize * self.keysyms_per_keycode as usize;
        let ks_unmod = self.keysyms.get(base_idx).copied().unwrap_or(0);
        let ks_shift = if self.keysyms_per_keycode > 1 && self.keysyms.get(base_idx + 1).copied().unwrap_or(0) != 0 {
            self.keysyms[base_idx + 1]
        } else if (0x61..=0x7A).contains(&ks_unmod) {
            ks_unmod - 0x20
        } else {
            ks_unmod
        };

        // Determine active keysym based on shift modifier
        let ks = if shift { ks_shift } else { ks_unmod };

        // Handle Control shortcuts (Ctrl+Z, Ctrl+Y, Ctrl+C, Ctrl+X, Ctrl+V, Ctrl+S, Ctrl+A, Ctrl+N, Ctrl+P, Ctrl+F, Ctrl+R, Ctrl+D, Ctrl+O)
        if ctrl {
            let base_char = match ks_unmod {
                0x0041..=0x005A => char::from_u32(ks_unmod + 32),
                0x0061..=0x007A => char::from_u32(ks_unmod),
                _ => None,
            };
            if let Some(c) = base_char {
                match c {
                    'z' => return if shift { KeyAction::Redo } else { KeyAction::Undo },
                    'y' => return KeyAction::Redo,
                    'c' => return KeyAction::Copy,
                    'x' => return KeyAction::Cut,
                    'v' => return KeyAction::Paste,
                    's' => return KeyAction::Save,
                    'a' => return KeyAction::SelectAll,
                    'n' => return KeyAction::NewNotebook,
                    'p' => return KeyAction::NewPage,
                    'f' => return KeyAction::Find,
                    'r' => return if shift { KeyAction::Reload } else { KeyAction::Rename },
                    'd' => return KeyAction::DeleteNote,
                    'o' => return KeyAction::ToggleOutline,
                    _ => {}
                }
            }
        }

        // Standard X11 keysym dispatch
        match ks {
            0xFF08 => KeyAction::Backspace,
            0xFF09 => KeyAction::Tab,
            0xFF0D | 0xFF8D => KeyAction::Return,
            0xFF1B => KeyAction::Escape,
            0xFFFF => KeyAction::Delete,
            0xFF50 | 0xFF95 => KeyAction::Home { shift },
            0xFF51 | 0xFF96 => KeyAction::Left { shift, ctrl },
            0xFF52 | 0xFF97 => KeyAction::Up { shift },
            0xFF53 | 0xFF98 => KeyAction::Right { shift, ctrl },
            0xFF54 | 0xFF99 => KeyAction::Down { shift },
            0xFF55 | 0xFF9A => KeyAction::PageUp,
            0xFF56 | 0xFF9B => KeyAction::PageDown,
            0xFF57 | 0xFF9C => KeyAction::End { shift },
            0xFFBE => KeyAction::F1,
            0xFFBF => KeyAction::F2,
            0xFFC0 => KeyAction::F3,
            0xFFC1 => KeyAction::F4,
            // Latin-1 / ASCII printable range
            0x0020..=0x007E | 0x00A0..=0x00FF => {
                let mut ch = char::from_u32(ks).unwrap_or(' ');
                if lock && ch.is_ascii_lowercase() {
                    ch = ch.to_ascii_uppercase();
                }
                KeyAction::Char(ch)
            }
            // Direct Unicode keysyms (ISO 10646 standard: 0x01000000..=0x0110FFFF)
            0x01000000..=0x0110FFFF => {
                if let Some(ch) = char::from_u32(ks - 0x01000000) {
                    KeyAction::Char(ch)
                } else {
                    KeyAction::None
                }
            }
            // Keypad numbers (0xFFB0..=0xFFB9)
            0xFFB0..=0xFFB9 => {
                let digit = (ks - 0xFFB0) as u8;
                KeyAction::Char((b'0' + digit) as char)
            }
            _ => KeyAction::None,
        }
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
        self.stream.set_read_timeout(None)?;
        let mut event_buf = [0u8; 32];
        self.stream.read_exact(&mut event_buf)?;
        Ok(event_buf)
    }

    /// Waits for the next X11 event packet with a timeout (for autosave and timers).
    pub fn wait_event_timeout(&mut self, timeout: std::time::Duration) -> Result<Option<[u8; 32]>, std::io::Error> {
        self.stream.set_nonblocking(false)?;
        self.stream.set_read_timeout(Some(timeout))?;
        let mut event_buf = [0u8; 32];
        match self.stream.read_exact(&mut event_buf) {
            Ok(_) => Ok(Some(event_buf)),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => Ok(None),
            Err(e) => Err(e),
        }
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
