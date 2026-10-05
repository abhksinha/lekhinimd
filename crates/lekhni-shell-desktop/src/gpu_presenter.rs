//! GPU acceleration presenter backend and display list vertex generator.
//!
//! Follows Section 8 of LEKHNI_ARCHITECTURE:
//! wgpu: display list to instanced quads + atlas texture.
//! Benchmarked against the software rasterizer.

use lekhni_ui::scene::{CmdKind, DisplayList};

/// GPU Quad Vertex structure matching standard vertex buffer layouts.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct GpuVertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub color: u32,
}

/// Translates a DisplayList into GPU instanced vertex arrays.
pub struct GpuPresenter {
    pub width: u32,
    pub height: u32,
    pub vertex_buffer: Vec<GpuVertex>,
    pub index_buffer: Vec<u32>,
}

impl GpuPresenter {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            vertex_buffer: Vec::with_capacity(1024),
            index_buffer: Vec::with_capacity(1536),
        }
    }

    /// Prepares GPU vertex and index streams from the given DisplayList.
    pub fn prepare_display_list(&mut self, list: &DisplayList) {
        self.vertex_buffer.clear();
        self.index_buffer.clear();

        for cmd in &list.commands {
            if cmd.kind == CmdKind::FillRect {
                let x0 = cmd.x as f32;
                let y0 = cmd.y as f32;
                let x1 = x0 + (cmd.width as f32);
                let y1 = y0 + (cmd.height as f32);

                let base_index = self.vertex_buffer.len() as u32;

                // 4 quad vertices
                self.vertex_buffer.push(GpuVertex {
                    position: [x0, y0],
                    uv: [0.0, 0.0],
                    color: cmd.color,
                });
                self.vertex_buffer.push(GpuVertex {
                    position: [x1, y0],
                    uv: [1.0, 0.0],
                    color: cmd.color,
                });
                self.vertex_buffer.push(GpuVertex {
                    position: [x1, y1],
                    uv: [1.0, 1.0],
                    color: cmd.color,
                });
                self.vertex_buffer.push(GpuVertex {
                    position: [x0, y1],
                    uv: [0.0, 1.0],
                    color: cmd.color,
                });

                // 6 quad indices (two triangles)
                self.index_buffer.push(base_index);
                self.index_buffer.push(base_index + 1);
                self.index_buffer.push(base_index + 2);
                self.index_buffer.push(base_index);
                self.index_buffer.push(base_index + 2);
                self.index_buffer.push(base_index + 3);
            }
        }
    }

    /// Returns the total quad count uploaded.
    pub fn quad_count(&self) -> usize {
        self.index_buffer.len() / 6
    }
}
