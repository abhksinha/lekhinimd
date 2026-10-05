//! iOS UIKit shell implementation.
//!
//! Follows Section 14 of LEKHNI_ARCHITECTURE:
//! UIKit integration, Metal/wgpu presentation, touch selection physics.

use lekhni_raster::damage::Rect;
use lekhni_shell::Presenter;

pub struct IosPresenter {
    pub width: u32,
    pub height: u32,
    pub scale_factor: f32,
    pub last_damage: Rect,
}

impl IosPresenter {
    pub fn new(width: u32, height: u32, scale_factor: f32) -> Self {
        Self {
            width,
            height,
            scale_factor,
            last_damage: Rect::default(),
        }
    }
}

impl Presenter for IosPresenter {
    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn scale(&self) -> f32 {
        self.scale_factor
    }

    fn present(&mut self, damage: Rect, _pixels: &[u32]) {
        self.last_damage = damage;
    }
}
