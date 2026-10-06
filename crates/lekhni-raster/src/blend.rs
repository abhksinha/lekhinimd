//! Integer premultiplied RGBA8 blending operations.
//!
//! Follows Section 8 of LEKHNI_ARCHITECTURE:
//! Integer blend using bit-shift arithmetic without floating-point divisions.

/// Blends a premultiplied RGBA source pixel over a destination pixel in place.
/// Pixels are packed 32-bit values: `0xAARRGGBB` or `0xRRGGBBAA`.
/// Here we use standard 0xAARRGGBB premultiplied representation.
#[inline(always)]
pub fn blend_pixel_premul(src: u32, dst: u32) -> u32 {
    let src_a = (src >> 24) & 0xFF;
    if src_a == 255 {
        return src;
    }
    if src_a == 0 {
        return dst;
    }

    let inv_a = 255 - src_a;

    let dst_a = (dst >> 24) & 0xFF;
    let dst_r = (dst >> 16) & 0xFF;
    let dst_g = (dst >> 8) & 0xFF;
    let dst_b = dst & 0xFF;

    let src_r = (src >> 16) & 0xFF;
    let src_g = (src >> 8) & 0xFF;
    let src_b = src & 0xFF;

    // Fast integer approximation: (c_src + (c_dst * inv_a + 127) / 255)
    let out_a = src_a + ((dst_a * inv_a + 127) / 255);
    let out_r = src_r + ((dst_r * inv_a + 127) / 255);
    let out_g = src_g + ((dst_g * inv_a + 127) / 255);
    let out_b = src_b + ((dst_b * inv_a + 127) / 255);

    (out_a << 24) | (out_r << 16) | (out_g << 8) | out_b
}

/// Blends an 8-bit glyph alpha mask against a text color over a destination pixel.
#[inline(always)]
pub fn blend_glyph_mask(text_color: u32, coverage: u8, dst: u32) -> u32 {
    if coverage == 0 {
        return dst;
    }
    let cov = coverage as u32;
    let text_a = (text_color >> 24) & 0xFF;
    let eff_a = (text_a * cov + 127) / 255;
    if eff_a == 0 {
        return dst;
    }

    let inv_a = 255 - eff_a;
    let dst_a = (dst >> 24) & 0xFF;
    let dst_r = (dst >> 16) & 0xFF;
    let dst_g = (dst >> 8) & 0xFF;
    let dst_b = dst & 0xFF;

    let text_r = (text_color >> 16) & 0xFF;
    let text_g = (text_color >> 8) & 0xFF;
    let text_b = text_color & 0xFF;

    let eff_r = (text_r * cov + 127) / 255;
    let eff_g = (text_g * cov + 127) / 255;
    let eff_b = (text_b * cov + 127) / 255;

    let out_a = eff_a + ((dst_a * inv_a + 127) / 255);
    let out_r = eff_r + ((dst_r * inv_a + 127) / 255);
    let out_g = eff_g + ((dst_g * inv_a + 127) / 255);
    let out_b = eff_b + ((dst_b * inv_a + 127) / 255);

    (out_a << 24) | (out_r << 16) | (out_g << 8) | out_b
}
