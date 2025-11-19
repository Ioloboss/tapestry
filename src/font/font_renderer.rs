use super::{Font, Glyph, GlyphIndex};


#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VertexRaw {
	pub position: [f32; 2],
	pub uv_coords: [f32; 2],
}

impl VertexRaw {
	pub fn desc() -> wgpu::VertexBufferLayout<'static> {
		wgpu::VertexBufferLayout {
			array_stride: std::mem::size_of::<VertexRaw>() as wgpu::BufferAddress,
			step_mode: wgpu::VertexStepMode::Vertex,
			attributes: &[
				wgpu::VertexAttribute {
					offset: 0,
					shader_location: 0,
					format: wgpu::VertexFormat::Float32x2,
				},
				wgpu::VertexAttribute {
					offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
					shader_location: 1,
					format: wgpu::VertexFormat::Float32x2,
				}
			]
		}
	}
}

pub trait ToRawTriangles {
	fn to_raw(&self, font: &Font, pixels_per_font_unit: f32, offset_x: i32, offest_y: i32, screen_width: f32, screen_height: f32, vertices_start: usize) -> (Vec<VertexRaw>, Vec<u32>, Vec<u32>, Vec<u32>);
}

impl ToRawTriangles for char {
	fn to_raw(&self, font: &Font, pixels_per_font_unit: f32, offset_x: i32, offest_y: i32, screen_width: f32, screen_height: f32, vertices_start: usize) -> (Vec<VertexRaw>, Vec<u32>, Vec<u32>, Vec<u32>) {
		let character_glyph_id = font.mappings[0].get_glyph_id(self.clone() as u64);
		match character_glyph_id {
			Some(glyph_index) => GlyphIndex(glyph_index).to_raw(font, pixels_per_font_unit, offset_x, offest_y, screen_width, screen_height, vertices_start),
			None => GlyphIndex(0).to_raw(font, pixels_per_font_unit, offset_x, offest_y, screen_width, screen_height, vertices_start),
		}
	}
}


impl ToRawTriangles for GlyphIndex {
	fn to_raw(&self, font: &Font, pixels_per_font_unit: f32, offset_x: i32, offest_y: i32, screen_width: f32, screen_height: f32, vertices_start: usize) -> (Vec<VertexRaw>, Vec<u32>, Vec<u32>, Vec<u32>) {
		let character_glyph: &Glyph = &font.glyphs[self.0 as usize];
		character_glyph.to_raw(font, pixels_per_font_unit, (0, 0).into(), (screen_width, screen_height).into(), (offset_x as f32, offest_y as f32).into(), vertices_start)
	}
}