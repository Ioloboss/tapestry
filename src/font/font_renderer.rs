use mircalla_types::{units::{Pixels}, vectors::{Alignment, Alignments, Colour, Position, Size}};
use wgpu::util::DeviceExt;
use winit::window::Window;
use std::{sync::{Arc, Mutex}};

use crate::font::{ToPixelsSize};

use super::{Font, FontUnits};


#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VertexRaw {
	pub position: [f32; 2],
	pub uv_coords: [f32; 2],
	pub colour: [f32; 3],
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
				},
				wgpu::VertexAttribute {
					offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
					shader_location: 2,
					format: wgpu::VertexFormat::Float32x3,
				},
			]
		}
	}
}

pub trait ToRawTriangles {
	fn to_raw(&self, font: &Font, pixels_per_font_unit: f32, screen_size: Size<Pixels<f32>>, position: Position<Pixels<f32>>, vertices_start: usize, colour: Colour) -> (Vec<VertexRaw>, Vec<u32>, Vec<u32>, Vec<u32>);
}

/* 
impl ToRawTriangles for char {
	fn to_raw(&self, font: &Font, pixels_per_font_unit: f32, offset_x: i32, offset_y: i32, screen_width: f32, screen_height: f32, vertices_start: usize) -> (Vec<VertexRaw>, Vec<u32>, Vec<u32>, Vec<u32>) {
		let character_glyph_id = font.mappings[0].get_glyph_id(self.clone() as u64);
		match character_glyph_id {
			Some(glyph_index) => GlyphIndex(glyph_index).to_raw(font, pixels_per_font_unit, offset_x, offset_y, screen_width, screen_height, vertices_start),
			None => GlyphIndex(0).to_raw(font, pixels_per_font_unit, offset_x, offset_y, screen_width, screen_height, vertices_start),
		}
	}
}


impl ToRawTriangles for GlyphIndex {
	fn to_raw(&self, font: &Font, pixels_per_font_unit: f32, offset_x: i32, offset_y: i32, screen_width: f32, screen_height: f32, vertices_start: usize) -> (Vec<VertexRaw>, Vec<u32>, Vec<u32>, Vec<u32>) {
		let character_glyph: &Glyph = &font.glyphs[self.0 as usize];
		character_glyph.to_raw(font, pixels_per_font_unit, (0, 0).into(), (screen_width, screen_height).into(), (offset_x as f32, offset_y as f32).into(), vertices_start)
	}
}
*/

/* 
impl ToRawTriangles for Mutex<String> {
	fn to_raw(&self, font: &Font, pixels_per_font_unit: f32, screen_size: Size<Pixels<f32>>, position: Position<Pixels<f32>>, vertices_start: usize, colour: Colour) -> (Vec<VertexRaw>, Vec<u32>, Vec<u32>, Vec<u32>) {
		let mut advance_offset: FontUnits<i32> = 0.into();
		let mut vertices_raw: Vec<VertexRaw> = Vec::new();
		let mut indices: Vec<u32> = Vec::new();
		let mut convex_bezier_indices: Vec<u32> = Vec::new();
		let mut concave_bezier_indices: Vec<u32> = Vec::new();
		let string = self.lock().unwrap();
		for character in string.chars() {
			let character_glyph_id = font.mappings[0].get_glyph_id(character as u64).unwrap_or(0);
			let glyph = &font.glyphs[character_glyph_id as usize];
			let (mut vertices_raw_character, mut indices_character, mut convex_bezier_indices_character, mut concave_bezier_indices_character) = glyph.to_raw(font, pixels_per_font_unit, (advance_offset, 0.into()).into(), screen_size, position, vertices_raw.len() + vertices_start, colour);
			advance_offset += glyph.advance_width;
			vertices_raw.append(&mut vertices_raw_character);
			indices.append(&mut indices_character);
			convex_bezier_indices.append(&mut convex_bezier_indices_character);
			concave_bezier_indices.append(&mut concave_bezier_indices_character);
		}
		drop(string);
		(vertices_raw, indices, convex_bezier_indices, concave_bezier_indices)
	}
} */

fn to_linear_rgb(color_chanel: u8) -> f32 {
	let value = color_chanel as f32 / 255.0;
	if value > 0.04045 {
		((value + 0.055) / 1.055).powf(2.4)
	} else {
		value / 12.92
	}
}

#[derive(Clone, Debug)]
pub enum NewRendererStateError {
	RequestAdapterError(wgpu::RequestAdapterError),
	RequestDeviceError(wgpu::RequestDeviceError),
}

impl From<wgpu::RequestAdapterError> for NewRendererStateError {
	fn from(value: wgpu::RequestAdapterError) -> Self {
		Self::RequestAdapterError(value)
	}
}

impl From<wgpu::RequestDeviceError> for NewRendererStateError {
	fn from(value: wgpu::RequestDeviceError) -> Self {
	    Self::RequestDeviceError(value)
	}
}

pub struct FontRenderer {
	device: Arc<wgpu::Device>,
	is_surface_configured: bool,
	render_pipeline: wgpu::RenderPipeline,
	vertex_buffer: wgpu::Buffer,
	index_buffer: wgpu::Buffer,
	pub window: Arc<Window>,
	number_of_indices: usize,
	convex_bezier_indices_start: usize,
	concave_bezier_indices_start: usize,
	mode_bind_group_layout: wgpu::BindGroupLayout,
	pub text_boxes: Vec<TextBox>,
}

impl FontRenderer {
	pub async fn new(window: Arc<Window>, device: Arc<wgpu::Device>, config: &wgpu::SurfaceConfiguration) -> Result<Self, NewRendererStateError> {
		let convex_bezier_indices_start = 0;
		let concave_bezier_indices_start = 0;

		let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
			label: Some("Tapestry Shader"),
			source: wgpu::ShaderSource::Wgsl(include_str!("../triangle_shader.wgsl").into()),
		});

		let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
			label: Some("Vertex Buffer"),
			size: 0,
			usage: wgpu::BufferUsages::VERTEX,
			mapped_at_creation: false,
		});

		/* let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Vertex Buffer"),
			contents: bytemuck::cast_slice(&vertices),
			usage: wgpu::BufferUsages::VERTEX,
		}); */

		let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
			label: Some("Index Buffer"),
			size: 0,
			usage: wgpu::BufferUsages::INDEX,
			mapped_at_creation: false,
		});

		/* let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Index Buffer"),
			contents: bytemuck::cast_slice(&indices),
			usage: wgpu::BufferUsages::INDEX,
		}); */

		let mode_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
			label: Some("mode_bind_group_layout"),
			entries: &[
				wgpu::BindGroupLayoutEntry {
					binding: 0,
					visibility: wgpu::ShaderStages::FRAGMENT,
					ty: wgpu::BindingType::Buffer {
						ty: wgpu::BufferBindingType::Uniform,
						has_dynamic_offset: false,
						min_binding_size: None,
					},
					count: None,
				}
			],
		});

		let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
			label: Some("Render Pipeline Layout"),
			bind_group_layouts: &[
				&mode_bind_group_layout
			],
			push_constant_ranges: &[],
		});

		let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
			label: Some("Render Pipeline"),
			layout: Some(&render_pipeline_layout),
			vertex: wgpu::VertexState {
				module: &shader,
				entry_point: Some("vs_main"),
				buffers: &[
					VertexRaw::desc(),
				],
				compilation_options: wgpu::PipelineCompilationOptions::default(),
			},
			fragment: Some(wgpu::FragmentState {
				module: &shader,
				entry_point: Some("fs_main"),
				targets: &[Some(wgpu::ColorTargetState {
					format: config.format,
					blend: Some(wgpu::BlendState::ALPHA_BLENDING),
					write_mask: wgpu::ColorWrites::ALL,
				})],
				compilation_options: wgpu::PipelineCompilationOptions::default(),
			}),
			primitive: wgpu::PrimitiveState {
				topology: wgpu::PrimitiveTopology::TriangleList,
				strip_index_format: None,
				front_face: wgpu::FrontFace::Ccw,
				//cull_mode: None,
				cull_mode: Some(wgpu::Face::Back),
				polygon_mode: wgpu::PolygonMode::Fill,
				unclipped_depth: false,
				conservative: false,
			},
			depth_stencil: None,
			multisample: wgpu::MultisampleState {
				count: 1, // SET TO 4 FOR MSAA
				mask: !0,
				alpha_to_coverage_enabled: false,
			},
			multiview: None,
			cache: None,
		});

		let text_boxes: Vec<TextBox> = Vec::new();

		Ok(Self {
			device,
			is_surface_configured: false,
			render_pipeline,
			vertex_buffer,
			index_buffer,
			window,
			number_of_indices: 0,
			convex_bezier_indices_start,
			concave_bezier_indices_start,
			mode_bind_group_layout,
			text_boxes,
		})
	}

	pub fn resize(&mut self, size: Size<Pixels<i32>>) {
		if size.width.value > 0 && size.height.value > 0 {
			self.is_surface_configured = true;
		}
	}

	pub fn update(&mut self) {
		let size = self.window.inner_size().to_pixels_size();

		let mut vertices: Vec<VertexRaw> = Vec::new();
		let mut indices: Vec<u32> = Vec::new();
		let mut convex_bezier_indices: Vec<u32> = Vec::new();
		let mut concave_bezier_indices: Vec<u32> = Vec::new();

		for text_box in self.text_boxes.iter() {
			let (mut vertices_text_box, mut indices_text_box, mut convex_bezier_indices_text_box, mut concave_bezier_indices_text_box) = text_box.to_raw(size.into(), vertices.len());
			vertices.append(&mut vertices_text_box);
			indices.append(&mut indices_text_box);
			convex_bezier_indices.append(&mut convex_bezier_indices_text_box);
			concave_bezier_indices.append(&mut concave_bezier_indices_text_box);
		}

		let convex_bezier_indices_start = indices.len();
		indices.extend(convex_bezier_indices);
		let concave_bezier_indices_start = indices.len();
		indices.extend(concave_bezier_indices);

		let vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Vertex Buffer"),
			contents: bytemuck::cast_slice(&vertices),
			usage: wgpu::BufferUsages::VERTEX,
		});

		let index_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Index Buffer"),
			contents: bytemuck::cast_slice(&indices),
			usage: wgpu::BufferUsages::INDEX,
		});

		self.number_of_indices = indices.len();
		self.vertex_buffer = vertex_buffer;
		self.index_buffer = index_buffer;
		self.convex_bezier_indices_start = convex_bezier_indices_start;
		self.concave_bezier_indices_start = concave_bezier_indices_start;
	}


/* 	pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {

		if self.number_of_indices == 0 {
			return Ok(());
		}

		if !self.is_surface_configured {
			return Ok(());
		}

		let output = self.surface.get_current_texture()?;

		let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

		let size = self.window.inner_size();

		let multisample_texture = self.device.create_texture(&wgpu::TextureDescriptor{
			label: Some("Multisample texture"),
			size: wgpu::Extent3d{ width: size.width, height: size.height, depth_or_array_layers: 1},
			sample_count: 4,
			dimension: wgpu::TextureDimension::D2,
			usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
			mip_level_count: 1,
			format: wgpu::TextureFormat::Bgra8UnormSrgb,
			view_formats: &[],
		});

		let multisample_view = multisample_texture.create_view(&wgpu::TextureViewDescriptor::default());

		let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
			label: Some("Render Encoder"),
		});

		{
			let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
				label: Some("Render Pass"),
				color_attachments: &[
					Some(wgpu::RenderPassColorAttachment {
						view: &multisample_view,
						resolve_target: Some(&view),
						ops: wgpu::Operations {
							// load: wgpu::LoadOp::Clear(
							// 	wgpu::Color {
							// 		r: to_linear_rgb(16) as f64,
							// 		g: to_linear_rgb(16) as f64,
							// 		b: to_linear_rgb(16) as f64,
							// 		a: 1.0,
							// 	}
							//),
							load: wgpu::LoadOp::Load,
							store: wgpu::StoreOp::Store,
						},
						depth_slice: None, // NOT IN TUTORIAL SO MIGHT NOT WORK.
					})
				],
				depth_stencil_attachment: None,
				occlusion_query_set: None,
				timestamp_writes: None,
			});

			render_pass.set_pipeline(&self.render_pipeline);
			render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
			render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
			let convex_bezier_indices_start = self.convex_bezier_indices_start;
			let concave_bezier_indices_start = self.concave_bezier_indices_start;
			let number_of_indices = self.number_of_indices;
			if convex_bezier_indices_start - 0 > 0 {
				let mode: u32 = 0;
				let mode_buffer = self.device.create_buffer_init(
					&wgpu::util::BufferInitDescriptor {
						label: Some("Mode Buffer"),
						contents: bytemuck::cast_slice(&[mode]),
						usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
					}
				);
				let mode_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
					layout: &self.mode_bind_group_layout,
					entries: &[
						wgpu::BindGroupEntry {
							binding: 0,
							resource: mode_buffer.as_entire_binding(),
						}
						],
						label: Some("mode_bind_group"),
					}
				);
				render_pass.set_bind_group(0, &mode_bind_group, &[]);
				render_pass.draw_indexed(0..convex_bezier_indices_start as _, 0, 0..1 as _);
			}

			if concave_bezier_indices_start - convex_bezier_indices_start > 0 {
				let mode: u32 = 1;
				let mode_buffer = self.device.create_buffer_init(
					&wgpu::util::BufferInitDescriptor {
						label: Some("Camera Buffer"),
						contents: bytemuck::cast_slice(&[mode]),
						usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
					}
				);
				let mode_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
					layout: &self.mode_bind_group_layout,
					entries: &[
						wgpu::BindGroupEntry {
							binding: 0,
							resource: mode_buffer.as_entire_binding(),
						}
						],
						label: Some("mode_bind_group"),
					}
				);
				render_pass.set_bind_group(0, &mode_bind_group, &[]);
				render_pass.draw_indexed(convex_bezier_indices_start as _..concave_bezier_indices_start as _, 0, 0..1 as _);
			}

			if number_of_indices - concave_bezier_indices_start > 0 {
				let mode: u32 = 2;
				let mode_buffer = self.device.create_buffer_init(
					&wgpu::util::BufferInitDescriptor {
						label: Some("Camera Buffer"),
						contents: bytemuck::cast_slice(&[mode]),
						usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
					}
				);
				let mode_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
					layout: &self.mode_bind_group_layout,
					entries: &[
						wgpu::BindGroupEntry {
							binding: 0,
							resource: mode_buffer.as_entire_binding(),
						}
						],
						label: Some("mode_bind_group"),
					}
				);
				render_pass.set_bind_group(0, &mode_bind_group, &[]);
				render_pass.draw_indexed(concave_bezier_indices_start as _..number_of_indices as _, 0, 0..1 as _);
			}
		}

		self.queue.submit(std::iter::once(encoder.finish()));
		self.window.pre_present_notify();
		output.present();

		Ok(())
	}	 */

	pub fn request_redraw(&self) {
		self.window.request_redraw();
	}

	pub fn add_text_box(&mut self, text_box: TextBox) {
		self.text_boxes.push(text_box);
	}

	pub fn draw_text(&self, queue: &wgpu::Queue, view: &wgpu::TextureView) {

		/* let view = output.texture.create_view(&wgpu::TextureViewDescriptor {
			label: Some("Tapestry TextureView"),
			..Default::default()
		}); */

		if self.number_of_indices == 0 {
			return
		}

		let size = self.window.inner_size();

		let multisample_texture = self.device.create_texture(&wgpu::TextureDescriptor{
			label: Some("Multisample texture"),
			size: wgpu::Extent3d{ width: size.width, height: size.height, depth_or_array_layers: 1},
			sample_count: 4,
			dimension: wgpu::TextureDimension::D2,
			usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
			mip_level_count: 1,
			format: wgpu::TextureFormat::Bgra8UnormSrgb,
			view_formats: &[],
		});

		let multisample_view = multisample_texture.create_view(&wgpu::TextureViewDescriptor::default());

		let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
			label: Some("Render Encoder"),
		});

		{
			let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
				label: Some("Render Pass"),
				color_attachments: &[
					Some(wgpu::RenderPassColorAttachment {
						view: &view, // SET TO &multisample_view FOR MSAA
						resolve_target: None, // SET TO Some(&view) FOR MSAA
						ops: wgpu::Operations {
							// load: wgpu::LoadOp::Clear(
							// 	wgpu::Color {
							// 		r: to_linear_rgb(16) as f64,
							// 		g: to_linear_rgb(16) as f64,
							// 		b: to_linear_rgb(16) as f64,
							// 		a: 1.0,
							// 	}
							//),
							load: wgpu::LoadOp::Load,
							store: wgpu::StoreOp::Store,
						},
						depth_slice: None, // NOT IN TUTORIAL SO MIGHT NOT WORK.
					})
				],
				depth_stencil_attachment: None,
				occlusion_query_set: None,
				timestamp_writes: None,
			});

			render_pass.set_pipeline(&self.render_pipeline);
			render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
			render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
			let convex_bezier_indices_start = self.convex_bezier_indices_start;
			let concave_bezier_indices_start = self.concave_bezier_indices_start;
			let number_of_indices = self.number_of_indices;
			if convex_bezier_indices_start - 0 > 0 {
				let mode: u32 = 0;
				let mode_buffer = self.device.create_buffer_init(
					&wgpu::util::BufferInitDescriptor {
						label: Some("Mode Buffer"),
						contents: bytemuck::cast_slice(&[mode]),
						usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
					}
				);
				let mode_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
					layout: &self.mode_bind_group_layout,
					entries: &[
						wgpu::BindGroupEntry {
							binding: 0,
							resource: mode_buffer.as_entire_binding(),
						}
						],
						label: Some("mode_bind_group"),
					}
				);
				render_pass.set_bind_group(0, &mode_bind_group, &[]);
				render_pass.draw_indexed(0..convex_bezier_indices_start as _, 0, 0..1 as _);
			}

			if concave_bezier_indices_start - convex_bezier_indices_start > 0 {
				let mode: u32 = 1;
				let mode_buffer = self.device.create_buffer_init(
					&wgpu::util::BufferInitDescriptor {
						label: Some("Camera Buffer"),
						contents: bytemuck::cast_slice(&[mode]),
						usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
					}
				);
				let mode_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
					layout: &self.mode_bind_group_layout,
					entries: &[
						wgpu::BindGroupEntry {
							binding: 0,
							resource: mode_buffer.as_entire_binding(),
						}
						],
						label: Some("mode_bind_group"),
					}
				);
				render_pass.set_bind_group(0, &mode_bind_group, &[]);
				render_pass.draw_indexed(convex_bezier_indices_start as _..concave_bezier_indices_start as _, 0, 0..1 as _);
			}

			if number_of_indices - concave_bezier_indices_start > 0 {
				let mode: u32 = 2;
				let mode_buffer = self.device.create_buffer_init(
					&wgpu::util::BufferInitDescriptor {
						label: Some("Camera Buffer"),
						contents: bytemuck::cast_slice(&[mode]),
						usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
					}
				);
				let mode_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
					layout: &self.mode_bind_group_layout,
					entries: &[
						wgpu::BindGroupEntry {
							binding: 0,
							resource: mode_buffer.as_entire_binding(),
						}
						],
						label: Some("mode_bind_group"),
					}
				);
				render_pass.set_bind_group(0, &mode_bind_group, &[]);
				render_pass.draw_indexed(concave_bezier_indices_start as _..number_of_indices as _, 0, 0..1 as _);
			}
		}

		queue.submit(std::iter::once(encoder.finish()));
	}
}

#[derive(Debug, Clone, Copy)]
pub struct WrapOptions {
	pub wrap_on: WrapOn,
}

#[derive(Debug, Clone, Copy)]
pub enum WrapOn {
	Character,
	Whitespace,
}

#[derive(Clone)]
pub struct TextBox {
	pub font: Arc<Font>,
	pub text: Arc<Mutex<String>>,
	pub pixels_per_em: Pixels<f32>,
	pub position: Position<Pixels<i32>>,
	pub text_box_size: Size<Pixels<i32>>,
	pub bounds: (Position<Pixels<i32>>, Position<Pixels<i32>>),
	pub colour: Colour,
	pub wrap_options: WrapOptions,
	pub alignment: Alignment,
}

impl TextBox {
	pub fn get_ideal_width(&self) -> Pixels<i32> { // Change when type are unified.
		let mut width: FontUnits<u32> = 0.into();
		let text_lock = self.text.lock().unwrap();
		for line in text_lock.lines() {
			let mut line_width: FontUnits<u32> = 0.into();
			for character in line.chars() {
				let character_glyph_id = self.font.mappings[0].get_glyph_id(character as u64).unwrap_or(0);
				let glyph = &self.font.get_glyph(character_glyph_id as usize);
				line_width += glyph.advance_width;
			}
			if line_width > width {
				width = line_width;
			}
		};
		
		drop(text_lock);
		(width.to_pixels_em(self.pixels_per_em, self.font.units_per_em).value.ceil() as i32).into()
	}

	pub fn get_height_offset(&self) -> Pixels<i32> {
		(self.font.typographic_ascender + self.font.typographic_descender).to_pixels_rounded(self.get_pixels_per_font_unit())
	}

	pub fn get_pixels_per_font_unit(&self) -> f32 {
		self.pixels_per_em.value / self.font.units_per_em.value as f32
	}

	pub fn get_height(&self, width: Pixels<i32>) -> Pixels<i32> {
		let mut advance_offset: FontUnits<i32> = 0.into();
		let mut vertical_offset: FontUnits<i32> = 0.into();
		let string = self.text.lock().unwrap();

		let mut first_line = true;

		for line in string.lines() {
			if !first_line {
				advance_offset = 0.into();
				vertical_offset += self.font.line_spacing;
			}
			match self.wrap_options.wrap_on {
				WrapOn::Character => {
					for character in line.chars() {
						let character_glyph_id = self.font.mappings[0].get_glyph_id(character as u64).unwrap_or(0);
						let glyph = &self.font.get_glyph(character_glyph_id as usize);

						let future_advance_offset = (advance_offset + glyph.advance_width).to_pixels(self.get_pixels_per_font_unit());
						if future_advance_offset > width.into() {

							advance_offset = 0.into();
							vertical_offset += self.font.line_spacing;
						}

						advance_offset += glyph.advance_width;
					}
				},
				WrapOn::Whitespace => {
					let space_glyph_id = self.font.mappings[0].get_glyph_id(' ' as u64).unwrap_or(0);
					let space_advance_width =  self.font.get_glyph(space_glyph_id as usize).advance_width;
					let mut add_space = false;
					let mut first_word = true;
					for word in line.split_whitespace() {
						let mut word_advance_width: FontUnits<i32> = if add_space {space_advance_width.into()} else {0.into()};
						for character in word.chars() {
							let character_glyph_id = self.font.mappings[0].get_glyph_id(character as u64).unwrap_or(0);
							let glyph = &self.font.get_glyph(character_glyph_id as usize);

							word_advance_width += glyph.advance_width;
						}

						let future_advance_offset = (advance_offset + word_advance_width).to_pixels(self.get_pixels_per_font_unit());
						if future_advance_offset > width.into() {
							advance_offset = 0.into();
							vertical_offset += self.font.line_spacing;
							add_space = false;
						} else if !first_word {
							add_space = true;
						}

						if add_space {
							let character_glyph_id = self.font.mappings[0].get_glyph_id(' ' as u64).unwrap_or(0);
							let glyph = &self.font.get_glyph(character_glyph_id as usize);

							advance_offset += glyph.advance_width;
						}

						for character in word.chars() {
							let character_glyph_id = self.font.mappings[0].get_glyph_id(character as u64).unwrap_or(0);
							let glyph = &self.font.get_glyph(character_glyph_id as usize);

							advance_offset += glyph.advance_width;
						}

						first_word = false;

					}
				}
			}
			first_line = false;
		}
		drop(string);


		((vertical_offset + self.font.typographic_ascender + self.font.typographic_descender).to_pixels_em(self.pixels_per_em, self.font.units_per_em).value.ceil() as i32).into()
	}

	pub fn get_text_size(&self, width: Pixels<i32>) -> Size<Pixels<i32>> {
		let mut advance_offset: FontUnits<i32> = 0.into();
		let mut vertical_offset: FontUnits<i32> = 0.into();
		let mut max_advance_offset: FontUnits<i32> = 0.into();
		let string = self.text.lock().unwrap();

		let mut first_line = true;

		for line in string.lines() {
			if !first_line {
				advance_offset = 0.into();
				vertical_offset += self.font.line_spacing;
			}
			match self.wrap_options.wrap_on {
				WrapOn::Character => {
					for character in line.chars() {
						let character_glyph_id = self.font.mappings[0].get_glyph_id(character as u64).unwrap_or(0);
						let glyph = &self.font.get_glyph(character_glyph_id as usize);

						let future_advance_offset = (advance_offset + glyph.advance_width).to_pixels(self.get_pixels_per_font_unit());
						if future_advance_offset > width.into() {

							advance_offset = 0.into();
							vertical_offset += self.font.line_spacing;
						}

						advance_offset += glyph.advance_width;
						if advance_offset > max_advance_offset {
							max_advance_offset = advance_offset;
						}
					}
				},
				WrapOn::Whitespace => {
					let space_glyph_id = self.font.mappings[0].get_glyph_id(' ' as u64).unwrap_or(0);
					let space_advance_width =  self.font.get_glyph(space_glyph_id as usize).advance_width;
					let mut add_space = false;
					let mut first_word = true;
					for word in line.split_whitespace() {
						let mut word_advance_width: FontUnits<i32> = if add_space {space_advance_width.into()} else {0.into()};
						for character in word.chars() {
							let character_glyph_id = self.font.mappings[0].get_glyph_id(character as u64).unwrap_or(0);
							let glyph = &self.font.get_glyph(character_glyph_id as usize);

							word_advance_width += glyph.advance_width;
						}

						let future_advance_offset = (advance_offset + word_advance_width).to_pixels(self.get_pixels_per_font_unit());
						if future_advance_offset > width.into() {
							advance_offset = 0.into();
							vertical_offset += self.font.line_spacing;
							add_space = false;
						} else if !first_word {
							add_space = true;
						}

						if add_space {
							let character_glyph_id = self.font.mappings[0].get_glyph_id(' ' as u64).unwrap_or(0);
							let glyph = &self.font.get_glyph(character_glyph_id as usize);

							advance_offset += glyph.advance_width;
						}

						for character in word.chars() {
							let character_glyph_id = self.font.mappings[0].get_glyph_id(character as u64).unwrap_or(0);
							let glyph = &self.font.get_glyph(character_glyph_id as usize);

							advance_offset += glyph.advance_width;
							if advance_offset > max_advance_offset {
								max_advance_offset = advance_offset;
							}
						}

						first_word = false;

					}
				}
			}
			first_line = false;
		}
		drop(string);


		(max_advance_offset.to_pixels_em(self.pixels_per_em, self.font.units_per_em).value.ceil() as i32, (vertical_offset + self.font.typographic_ascender + self.font.typographic_descender).to_pixels_em(self.pixels_per_em, self.font.units_per_em).value.ceil() as i32).into()
	}
}

impl TextBox {
	fn to_raw(&self, screen_size: Size<Pixels<i32>>, vertices_start: usize) -> (Vec<VertexRaw>, Vec<u32>, Vec<u32>, Vec<u32>) {
		let mut advance_offset: FontUnits<i32> = 0.into();
		let mut vertical_offset: FontUnits<i32> = 0.into();
		let mut vertices_raw: Vec<VertexRaw> = Vec::new();
		let mut indices: Vec<u32> = Vec::new();
		let mut convex_bezier_indices: Vec<u32> = Vec::new();
		let mut concave_bezier_indices: Vec<u32> = Vec::new();

		let text_size = self.get_text_size(self.text_box_size.width);

		let string = self.text.lock().unwrap();

		let mut first_line = true;

		let mut position: Position<Pixels<i32>> = (0, 0).into();

		position.x = match self.alignment.x {
			Alignments::Start => self.position.x,
			Alignments::Centre => {
				self.position.x + ((self.text_box_size.width - text_size.width) / 2)
			},
			Alignments::End => {
				self.position.x + (self.text_box_size.width - text_size.width)
			},
		};

		position.y = match self.alignment.y {
			Alignments::Start => self.position.y,
			Alignments::Centre => {
				self.position.y - ((self.text_box_size.height - text_size.height) / 2)
			},
			Alignments::End => {
				self.position.y - (self.text_box_size.height - text_size.height)
			},
		};

		for line in string.lines() {
			if !first_line {
				advance_offset = 0.into();
				vertical_offset -= self.font.line_spacing;
			}
			match self.wrap_options.wrap_on {
				WrapOn::Character => {
					for character in line.chars() {
						let character_glyph_id = self.font.mappings[0].get_glyph_id(character as u64).unwrap_or(0);
						let glyph = &self.font.get_glyph(character_glyph_id as usize);

						let future_advance_offset = (advance_offset + glyph.advance_width).to_pixels(self.get_pixels_per_font_unit());
						if future_advance_offset > self.text_box_size.width.into() {
							advance_offset = 0.into();
							vertical_offset -= self.font.line_spacing;
						}

						let (mut vertices_raw_character, mut indices_character, mut convex_bezier_indices_character, mut concave_bezier_indices_character) = glyph.to_raw(&*self.font, self.get_pixels_per_font_unit(), (advance_offset, vertical_offset).into(), screen_size, position.into(), vertices_raw.len() + vertices_start, self.colour, self.bounds);
						advance_offset += glyph.advance_width;
						vertices_raw.append(&mut vertices_raw_character);
						indices.append(&mut indices_character);
						convex_bezier_indices.append(&mut convex_bezier_indices_character);
						concave_bezier_indices.append(&mut concave_bezier_indices_character);
					}
				},
				WrapOn::Whitespace => {
					let space_glyph_id = self.font.mappings[0].get_glyph_id(' ' as u64).unwrap_or(0);
					let space_advance_width =  self.font.get_glyph(space_glyph_id as usize).advance_width;
					let mut add_space = false;
					let mut first_word = true;
					for word in line.split_whitespace() {
						let mut word_advance_width: FontUnits<i32> = if add_space {space_advance_width.into()} else {0.into()};
						for character in word.chars() {
							let character_glyph_id = self.font.mappings[0].get_glyph_id(character as u64).unwrap_or(0);
							let glyph = &self.font.get_glyph(character_glyph_id as usize);

							word_advance_width += glyph.advance_width;
						}

						let future_advance_offset = (advance_offset + word_advance_width).to_pixels(self.get_pixels_per_font_unit());
						if future_advance_offset > self.text_box_size.width.into() {
							advance_offset = 0.into();
							vertical_offset -= self.font.line_spacing;
							add_space = false;
						} else if !first_word {
							add_space = true;
						}

						if add_space {
							let character_glyph_id = self.font.mappings[0].get_glyph_id(' ' as u64).unwrap_or(0);
							let glyph = &self.font.get_glyph(character_glyph_id as usize);

							let (mut vertices_raw_character, mut indices_character, mut convex_bezier_indices_character, mut concave_bezier_indices_character) = glyph.to_raw(&*self.font, self.get_pixels_per_font_unit(), (advance_offset, vertical_offset).into(), screen_size, position.into(), vertices_raw.len() + vertices_start, self.colour, self.bounds);
							advance_offset += glyph.advance_width;
							vertices_raw.append(&mut vertices_raw_character);
							indices.append(&mut indices_character);
							convex_bezier_indices.append(&mut convex_bezier_indices_character);
							concave_bezier_indices.append(&mut concave_bezier_indices_character);
						}

						for character in word.chars() {
							let character_glyph_id = self.font.mappings[0].get_glyph_id(character as u64).unwrap_or(0);
							let glyph = &self.font.get_glyph(character_glyph_id as usize);

							let (mut vertices_raw_character, mut indices_character, mut convex_bezier_indices_character, mut concave_bezier_indices_character) = glyph.to_raw(&*self.font, self.get_pixels_per_font_unit(), (advance_offset, vertical_offset).into(), screen_size, position.into(), vertices_raw.len() + vertices_start, self.colour, self.bounds);
							advance_offset += glyph.advance_width;
							vertices_raw.append(&mut vertices_raw_character);
							indices.append(&mut indices_character);
							convex_bezier_indices.append(&mut convex_bezier_indices_character);
							concave_bezier_indices.append(&mut concave_bezier_indices_character);
						}

						first_word = false;
					}
				}
			}
			first_line = false;
		}
		drop(string);
		(vertices_raw, indices, convex_bezier_indices, concave_bezier_indices)
	}
}

impl TextBox {
	pub fn new(font: Arc<Font>, text: Arc<Mutex<String>>, pixels_per_em: Pixels<f32>, colour: Colour, wrap_options: WrapOptions) -> TextBox {
		TextBox {
			font,
			text,
			pixels_per_em,
			position: (0, 0).into(),
			text_box_size: (0, 0).into(),
			bounds: ((0, 0).into(), (i32::MAX, i32::MAX).into()),
			colour,
			wrap_options,
			alignment: Alignment { x: mircalla_types::vectors::Alignments::Start, y: mircalla_types::vectors::Alignments::Start }
		}
	}

	pub fn alignment(mut self, alignment: Alignment) -> TextBox {
		self.alignment = alignment;
		self
	}
}
