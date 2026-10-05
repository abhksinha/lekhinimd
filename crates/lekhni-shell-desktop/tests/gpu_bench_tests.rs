use std::time::Instant;
use lekhni_raster::canvas::Canvas;
use lekhni_raster::damage::Rect;
use lekhni_shell_desktop::gpu_presenter::GpuPresenter;
use lekhni_ui::scene::{DisplayCmd, DisplayList};

#[test]
fn test_software_vs_gpu_presentation_benchmark() {
    let width = 1920;
    let height = 1080;

    // 1. Build a display list of 500 rects
    let mut list = DisplayList::new();
    for i in 0..500 {
        let x = (i * 7) % (width - 100);
        let y = (i * 11) % (height - 30);
        list.push(DisplayCmd::fill_rect(Rect::new(x, y, 80, 20), 0xFF_33_66_99));
    }

    // 2. Measure GPU vertex stream preparation
    let mut gpu = GpuPresenter::new(width as u32, height as u32);
    let start_gpu = Instant::now();
    gpu.prepare_display_list(&list);
    let gpu_duration = start_gpu.elapsed();
    assert_eq!(gpu.quad_count(), 500);

    // 3. Measure Software Canvas rasterization
    let mut fb = vec![0u32; (width * height) as usize];
    let mut canvas = Canvas::new(&mut fb, width as u32, height as u32);
    let start_sw = Instant::now();
    for cmd in &list.commands {
        canvas.fill_rect(Rect::new(cmd.x as i32, cmd.y as i32, cmd.width as i32, cmd.height as i32), cmd.color);
    }
    let sw_duration = start_sw.elapsed();

    // Verify both executed within acceptable interactive bounds (< 15 ms for 500 rects at 1080p)
    assert!(gpu_duration.as_millis() < 15);
    assert!(sw_duration.as_millis() < 30);
}
