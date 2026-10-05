//! Lazy inline span parsing and packed representation.
//!
//! Follows Section 4.2 of LEKHNI_ARCHITECTURE:
//! Packed u32 = offset delta (24 bits) + 4-bit style + 4-bit class.

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// 4-bit style categories.
pub mod style {
    pub const PLAIN: u8 = 0;
    pub const BOLD: u8 = 1;
    pub const ITALIC: u8 = 2;
    pub const CODE: u8 = 3;
    pub const LINK: u8 = 4;
    pub const STRIKE: u8 = 5;
    pub const HEADING: u8 = 6;
    pub const QUOTE: u8 = 7;
}

/// Packed 32-bit inline span descriptor:
/// - 24 bits: offset delta from block start (up to 16MB)
/// - 4 bits: style tag
/// - 4 bits: class tag / flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct PackedSpan(pub u32);

impl PackedSpan {
    #[inline(always)]
    pub const fn new(offset_delta: u32, style: u8, class: u8) -> Self {
        let val = (offset_delta & 0x00FF_FFFF)
            | (((style & 0x0F) as u32) << 24)
            | (((class & 0x0F) as u32) << 28);
        Self(val)
    }

    #[inline(always)]
    pub const fn offset_delta(&self) -> u32 {
        self.0 & 0x00FF_FFFF
    }

    #[inline(always)]
    pub const fn style(&self) -> u8 {
        ((self.0 >> 24) & 0x0F) as u8
    }

    #[inline(always)]
    pub const fn class(&self) -> u8 {
        ((self.0 >> 28) & 0x0F) as u8
    }
}

/// Inline span parser parsing text for visible blocks on-demand.
#[cfg(feature = "alloc")]
pub struct InlineParser;

#[cfg(feature = "alloc")]
impl InlineParser {
    /// Parses inline styles for a block's text slice, returning packed spans.
    pub fn parse_spans(text: &[u8], out_spans: &mut Vec<PackedSpan>) {
        out_spans.clear();
        let mut i = 0;
        let len = text.len();

        let mut in_code = false;
        let mut code_start = 0;

        let mut in_bold = false;
        let mut bold_start = 0;

        let mut in_italic = false;
        let mut italic_start = 0;

        let mut in_strike = false;
        let mut strike_start = 0;

        while i < len {
            let b = text[i];

            // Inline code `...` takes precedence over all other markup
            if b == b'`' {
                if in_code {
                    out_spans.push(PackedSpan::new(code_start as u32, style::CODE, 0));
                    out_spans.push(PackedSpan::new((i + 1) as u32, style::PLAIN, 0));
                    in_code = false;
                } else {
                    in_code = true;
                    code_start = i;
                }
                i += 1;
                continue;
            }

            if in_code {
                i += 1;
                continue;
            }

            // Strikethrough ~~...~~
            if b == b'~' && i + 1 < len && text[i + 1] == b'~' {
                if in_strike {
                    out_spans.push(PackedSpan::new(strike_start as u32, style::STRIKE, 0));
                    out_spans.push(PackedSpan::new((i + 2) as u32, style::PLAIN, 0));
                    in_strike = false;
                } else {
                    in_strike = true;
                    strike_start = i;
                }
                i += 2;
                continue;
            }

            if in_strike {
                i += 1;
                continue;
            }

            // Wikilinks [[target]] or [[target|label]]
            if b == b'[' && i + 1 < len && text[i + 1] == b'[' {
                let mut j = i + 2;
                while j + 1 < len && !(text[j] == b']' && text[j + 1] == b']') && text[j] != b'\n' {
                    j += 1;
                }
                if j + 1 < len && text[j] == b']' && text[j + 1] == b']' {
                    out_spans.push(PackedSpan::new(i as u32, style::LINK, 1));
                    out_spans.push(PackedSpan::new((j + 2) as u32, style::PLAIN, 0));
                    i = j + 2;
                    continue;
                }
            }

            // Links [label](url)
            if b == b'[' {
                let mut j = i + 1;
                while j < len && text[j] != b']' && text[j] != b'\n' {
                    j += 1;
                }
                if j < len && text[j] == b']' && j + 1 < len && text[j + 1] == b'(' {
                    let mut k = j + 2;
                    while k < len && text[k] != b')' && text[k] != b'\n' {
                        k += 1;
                    }
                    if k < len && text[k] == b')' {
                        out_spans.push(PackedSpan::new(i as u32, style::LINK, 0));
                        out_spans.push(PackedSpan::new((k + 1) as u32, style::PLAIN, 0));
                        i = k + 1;
                        continue;
                    }
                }
            }

            // Bold **...**
            if b == b'*' && i + 1 < len && text[i + 1] == b'*' {
                if in_bold {
                    out_spans.push(PackedSpan::new(bold_start as u32, style::BOLD, 0));
                    out_spans.push(PackedSpan::new((i + 2) as u32, style::PLAIN, 0));
                    in_bold = false;
                } else {
                    in_bold = true;
                    bold_start = i;
                }
                i += 2;
                continue;
            }

            // Italic *...*
            if b == b'*' {
                if in_italic {
                    out_spans.push(PackedSpan::new(italic_start as u32, style::ITALIC, 0));
                    out_spans.push(PackedSpan::new((i + 1) as u32, style::PLAIN, 0));
                    in_italic = false;
                } else {
                    in_italic = true;
                    italic_start = i;
                }
                i += 1;
                continue;
            }

            i += 1;
        }
    }
}
