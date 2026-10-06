use lekhni_raster::atlas::GlyphAtlas;
use lekhni_raster::blend::blend_pixel_premul;
use lekhni_raster::canvas::Canvas;
use lekhni_raster::damage::Rect;

#[test]
fn test_blend_premul() {
    let opaque_red = 0xFF_FF_00_00;
    let opaque_blue = 0xFF_00_00_FF;
    let result = blend_pixel_premul(opaque_red, opaque_blue);
    assert_eq!(result, opaque_red); // Opaque source completely replaces destination

    let transparent = 0x00_00_00_00;
    let result = blend_pixel_premul(transparent, opaque_blue);
    assert_eq!(result, opaque_blue);
}

#[test]
fn test_rect_clip_and_canvas() {
    let mut fb = [0u32; 100 * 100];
    let mut canvas = Canvas::new(&mut fb, 100, 100);

    let red = 0xFF_FF_00_00;
    canvas.fill_rect(Rect::new(10, 10, 20, 20), red);

    // Check pixel inside filled rect
    let inside_idx = 15 * 100 + 15;
    assert_eq!(canvas.buffer[inside_idx], red);

    // Check pixel outside filled rect
    let outside_idx = 5 * 100 + 5;
    assert_eq!(canvas.buffer[outside_idx], 0);
}

#[test]
fn test_glyph_atlas_packing() {
    let mut atlas = GlyphAtlas::new(64, 64);
    let g1 = atlas.allocate(16, 16).expect("should allocate");
    assert_eq!(g1.x, 0);
    assert_eq!(g1.y, 0);

    let g2 = atlas.allocate(16, 16).expect("should allocate");
    assert_eq!(g2.x, 16);
    assert_eq!(g2.y, 0);
}

#[test]
fn test_vector_font_collection() {
    use lekhni_raster::vector_font::FontCollection;

    let fonts = FontCollection::load_default().expect("load default vector fonts");
    assert!(fonts.ui.line_height > 10);
    assert!(fonts.editor.line_height > 10);
    assert!(fonts.h1.line_height > fonts.ui.line_height);

    let (w, h) = fonts.ui.measure_text("Hello Lekhni");
    assert!(w > 40);
    assert!(h > 10);

    let mut fb = [0u32; 200 * 50];
    let mut canvas = Canvas::new(&mut fb, 200, 50);
    let adv = fonts.ui.draw_text(&mut canvas, "Hello", 10, 10, 0xFF_FF_FF_FF);
    assert!(adv > 20);
    // Verify some pixels were shaded
    let has_drawn_pixel = canvas.buffer.iter().any(|&px| px != 0);
    assert!(has_drawn_pixel);
}
