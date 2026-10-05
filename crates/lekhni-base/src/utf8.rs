//! UTF-8 lookup tables and boundary helpers without branch overhead.

/// Lookup table for the byte length of a UTF-8 sequence given its leading byte.
pub static UTF8_CHAR_WIDTH: [u8; 256] = {
    let mut table = [1u8; 256];
    let mut i = 0x80;
    while i <= 0xBF {
        table[i] = 0; // Continuation bytes are not sequence starters
        i += 1;
    }
    i = 0xC0;
    while i <= 0xDF {
        table[i] = 2;
        i += 1;
    }
    i = 0xE0;
    while i <= 0xEF {
        table[i] = 3;
        i += 1;
    }
    i = 0xF0;
    while i <= 0xF7 {
        table[i] = 4;
        i += 1;
    }
    i = 0xF8;
    while i <= 0xFF {
        table[i] = 0; // Invalid UTF-8
        i += 1;
    }
    table
};

/// Returns true if the byte at the specified index is a UTF-8 character boundary.
#[inline(always)]
pub fn is_char_boundary(b: u8) -> bool {
    // A byte is a char boundary if it is not a continuation byte (0b10xxxxxx)
    (b as i8) >= -0x40
}

/// Returns the byte length of the UTF-8 codepoint starting with `lead_byte`.
#[inline(always)]
pub fn utf8_char_width(lead_byte: u8) -> usize {
    UTF8_CHAR_WIDTH[lead_byte as usize] as usize
}
