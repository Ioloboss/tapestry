use wgpu::util::DeviceExt;
use winit::window::Window;
use std::sync::{Arc, Mutex};

use crate::font::Vertex;

use super::{Font, Size, Position, FontUnits, Pixels};


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
	fn to_raw(&self, font: &Font, pixels_per_font_unit: f32, screen_size: Size<Pixels<f32>>, position: Position<Pixels<f32>>, vertices_start: usize) -> (Vec<VertexRaw>, Vec<u32>, Vec<u32>, Vec<u32>);
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

impl ToRawTriangles for Mutex<String> {
	fn to_raw(&self, font: &Font, pixels_per_font_unit: f32, screen_size: Size<Pixels<f32>>, position: Position<Pixels<f32>>, vertices_start: usize) -> (Vec<VertexRaw>, Vec<u32>, Vec<u32>, Vec<u32>) {
		let mut advance_offset: FontUnits<i32> = 0.into();
		let mut vertices_raw: Vec<VertexRaw> = Vec::new();
		let mut indices: Vec<u32> = Vec::new();
		let mut convex_bezier_indices: Vec<u32> = Vec::new();
		let mut concave_bezier_indices: Vec<u32> = Vec::new();
		let string = self.lock().unwrap();
		for character in string.chars() {
			let character_glyph_id = font.mappings[0].get_glyph_id(character as u64).unwrap_or(0);
			let glyph = &font.glyphs[character_glyph_id as usize];
			let (mut vertices_raw_character, mut indices_character, mut convex_bezier_indices_character, mut concave_bezier_indices_character) = glyph.to_raw(font, pixels_per_font_unit, (advance_offset, 0.into()).into(), screen_size, position, vertices_raw.len() + vertices_start);
			advance_offset += glyph.advance_width;
			vertices_raw.append(&mut vertices_raw_character);
			indices.append(&mut indices_character);
			convex_bezier_indices.append(&mut convex_bezier_indices_character);
			concave_bezier_indices.append(&mut concave_bezier_indices_character);
		}
		drop(string);
		(vertices_raw, indices, convex_bezier_indices, concave_bezier_indices)
	}
}

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
	surface: wgpu::Surface<'static>,
	device: wgpu::Device,
	queue: wgpu::Queue,
	config: wgpu::SurfaceConfiguration,
	is_surface_configured: bool,
	render_pipeline: wgpu::RenderPipeline,
	vertex_buffer: wgpu::Buffer,
	index_buffer: wgpu::Buffer,
	pub window: Arc<Window>,
	number_of_indices: usize,
	convex_bezier_indices_start: usize,
	concave_bezier_indices_start: usize,
	mode_bind_group_layout: wgpu::BindGroupLayout,
	text_boxes: Vec<TextBox>,
}

impl FontRenderer {
	pub async fn new(window: Arc<Window>) -> Result<Self, NewRendererStateError> {
		println!("New State");
		let size = window.inner_size();

		let convex_bezier_indices_start = 0;
		let concave_bezier_indices_start = 0;

		let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
			backends: wgpu::Backends::PRIMARY,
			..Default::default()
		});

		let surface = instance.create_surface(window.clone()).unwrap();

		let adapter = instance
			.request_adapter(&wgpu::RequestAdapterOptions {
				power_preference: wgpu::PowerPreference::default(), // POTENTIALLY SWITCH TO LOW POWER?
				compatible_surface: Some(&surface),
				force_fallback_adapter: false,
			})
			.await?;

		let (device, queue) = adapter
			.request_device(&wgpu::DeviceDescriptor {
				label: None,
				required_features: wgpu::Features::empty(),
				required_limits: wgpu::Limits::defaults(),
				memory_hints: Default::default(),
				trace: wgpu::Trace::Off,
				experimental_features: wgpu::ExperimentalFeatures::disabled(),
			})
			.await?;

		let surface_caps = surface.get_capabilities(&adapter);

		let surface_format = surface_caps.formats.iter()
			.find(|f| f.is_srgb())
			.copied()
			.unwrap_or(surface_caps.formats[0]);

		let config = wgpu::SurfaceConfiguration {
			usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
			format: surface_format,
			width: size.width,
			height: size.height,
			present_mode: surface_caps.present_modes[0],
			alpha_mode: surface_caps.alpha_modes[0],
			view_formats: vec![],
			desired_maximum_frame_latency: 2,
		};

		let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
			label: Some("Shader"),
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
				count: 4,
				mask: !0,
				alpha_to_coverage_enabled: false,
			},
			multiview: None,
			cache: None,
		});

		let text_boxes: Vec<TextBox> = Vec::new();

		Ok(Self {
			surface,
			device,
			queue,
			config,
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

	pub fn resize(&mut self, width: u32, height: u32) {
		println!("Resized to: {width}x{height}");
		if width > 0 && height > 0 {
			self.config.width = width;
			self.config.height = height;
			self.surface.configure(&self.device, &self.config);
			self.is_surface_configured = true;
			self.update();
		}
	}

	pub fn update(&mut self) {
		let size = self.window.inner_size();

		let mut vertices: Vec<VertexRaw> = Vec::new();
		let mut indices: Vec<u32> = Vec::new();
		let mut convex_bezier_indices: Vec<u32> = Vec::new();
		let mut concave_bezier_indices: Vec<u32> = Vec::new();

		for text_box in self.text_boxes.iter() {
			let (mut vertices_text_box, mut indices_text_box, mut convex_bezier_indices_text_box, mut concave_bezier_indices_text_box) = text_box.text.to_raw(&*text_box.font, text_box.pixels_per_font_unit, size.into(), text_box.position, vertices.len());
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


	pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
		println!("Rendering");

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
			println!("Number of indices: {}", self.number_of_indices);
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
	}	

	pub fn request_redraw(&self) {
		self.window.request_redraw();
	}

	pub fn add_text_box(&mut self, text_box: TextBox) {
		self.text_boxes.push(text_box);
	}

	pub fn draw_text(&self, queue: &wgpu::Queue, output: &wgpu::SurfaceTexture) {

		let view = output.texture.create_view(&wgpu::TextureViewDescriptor {
			label: Some("Tapestry TextureView"),
			..Default::default()
		});

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
			println!("Number of indices: {}", self.number_of_indices);
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

pub struct TextBox {
	pub font: Arc<Font>,
	pub text: Arc<Mutex<String>>,
	pub pixels_per_font_unit: f32,
	pub position: Position<Pixels<f32>>,
}