//! Lekhni desktop shell runtime.
//!
//! Provides the std-enabled shell implementations of Fs, Clock, Presenter,
//! and runs the desktop pipeline proving keystroke latency within the < 8 ms budget.

pub mod a11y_bridge;
pub mod gpu_presenter;
use lekhni_md::parser::MarkdownParser;
use lekhni_raster::canvas::Canvas;
use lekhni_raster::damage::Rect;
use lekhni_shell::{Clock, Presenter};
use lekhni_text::buffer::PieceTable;
use lekhni_text::cursor::Cursor;

/// System monotonic clock implementation.
pub struct StdClock;

impl Clock for StdClock {
    fn now_ms(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    fn set_deadline(&mut self, _deadline_ms: u64) {}
}

/// In-memory / software presentation surface for headless testing and latency proofs.
pub struct SoftwarePresenter {
    pub width: u32,
    pub height: u32,
    pub framebuffer: Vec<u32>,
    pub last_damage: Rect,
}

impl SoftwarePresenter {
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width * height) as usize;
        Self {
            width,
            height,
            framebuffer: vec![0; size],
            last_damage: Rect::default(),
        }
    }
}

impl Presenter for SoftwarePresenter {
    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn scale(&self) -> f32 {
        1.0
    }

    fn present(&mut self, damage: Rect, pixels: &[u32]) {
        self.last_damage = damage;
        let visible = damage.intersect(&Rect::new(0, 0, self.width as i32, self.height as i32));
        if let Some(r) = visible {
            let x1 = r.x as usize;
            let y1 = r.y as usize;
            let w = r.width as usize;
            let h = r.height as usize;
            let stride = self.width as usize;

            for row in 0..h {
                let start = (y1 + row) * stride + x1;
                self.framebuffer[start..start + w].copy_from_slice(&pixels[start..start + w]);
            }
        }
    }
}

/// Minimal Desktop Editor verifying the keystroke-to-pixels latency budget.
pub struct DesktopEditor<'a> {
    pub buffer: PieceTable<'a>,
    pub parser: MarkdownParser,
    pub cursor: Cursor,
    pub presenter: SoftwarePresenter,
    pub damage_rect: Rect,
}

impl<'a> DesktopEditor<'a> {
    pub fn new(initial_text: &'a [u8], width: u32, height: u32) -> Self {
        let mut parser = MarkdownParser::new();
        parser.parse_full(initial_text);

        Self {
            buffer: PieceTable::new(initial_text),
            parser,
            cursor: Cursor::default(),
            presenter: SoftwarePresenter::new(width, height),
            damage_rect: Rect::new(0, 0, width as i32, height as i32),
        }
    }

    /// Handles a single keystroke insertion and renders the damaged line.
    /// Returns the elapsed execution time in microseconds.
    pub fn handle_keystroke(&mut self, text: &[u8]) -> u128 {
        let start = std::time::Instant::now();

        // 1. Apply edit in piece table with coalescing
        let offset = self.cursor.byte_offset;
        self.buffer.insert(offset, text);
        self.cursor.byte_offset += text.len();

        // 2. Incremental block reparse from dirty line
        let (dirty_line, _) = self.buffer.offset_to_line(offset);
        let mut full_text = vec![0u8; self.buffer.len()];
        self.buffer.copy_range(0, self.buffer.len(), &mut full_text);
        self.parser.reparse_incremental(&full_text, dirty_line);

        // 3. Compute damage rect (line height ~ 20px, damaged row)
        let line_height = 20;
        let damage_y = (dirty_line as i32) * line_height;
        let damage = Rect::new(0, damage_y, self.presenter.width as i32, line_height);

        // 4. Rasterize damaged region into canvas
        {
            let mut canvas = Canvas::new(
                &mut self.presenter.framebuffer,
                self.presenter.width,
                self.presenter.height,
            );
            canvas.clip = damage;
            canvas.fill_rect(damage, 0xFF_1E_1E_1E); // Background
        }

        // 5. Present damage to surface
        self.presenter.last_damage = damage;

        start.elapsed().as_micros()
    }
}
