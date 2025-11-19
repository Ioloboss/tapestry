use tapestry::ttf_reader::{self, CharacterToGlyphIndexTable, FontHeaderTable, GlyphTable, IndexToLocationTable, MaximumProfileTable, TableRecord, TableTag};
use tapestry::ttf_parser::{Direction, GlyphDataIntermediate, GlyphIntermediate};
use tapestry::font::{self, Font, GlyphIndex, font_renderer::{self, VertexRaw, ToRawTriangles}};
use winit::error::EventLoopError;

use std::{fs::File, path::Path};
use std::sync::Arc;

use wgpu::util::{DeviceExt};
use winit::{
	application::ApplicationHandler, event::*, event_loop::{ActiveEventLoop, EventLoop}, keyboard::{KeyCode, PhysicalKey}, window::Window
};


#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Instance {
	position: [f32; 2],
}

impl Instance {
	pub fn desc() -> wgpu::VertexBufferLayout<'static> {
		wgpu::VertexBufferLayout {
			array_stride: std::mem::size_of::<Instance>() as wgpu::BufferAddress,
			step_mode: wgpu::VertexStepMode::Instance,
			attributes: &[
				wgpu::VertexAttribute {
					offset: 0,
					shader_location: 2,
					format: wgpu::VertexFormat::Float32x2,
				},
			]
		}
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

pub struct State {
	surface: wgpu::Surface<'static>,
	device: wgpu::Device,
	queue: wgpu::Queue,
	config: wgpu::SurfaceConfiguration,
	is_surface_configured: bool,
	render_pipeline: wgpu::RenderPipeline,
	vertex_buffer: wgpu::Buffer,
	index_buffer: wgpu::Buffer,
	window: Arc<Window>,
	font: Font,
	current_glyph_id: usize,
	indices: Vec<u32>,
	vertices: Vec<VertexRaw>,
	pixels_per_font_unit: f32,
	instance_buffer: wgpu::Buffer,
	instances: Vec<Instance>,
	convex_bezier_indices_start: usize,
	concave_bezier_indices_start: usize,
	mode_bind_group_layout: wgpu::BindGroupLayout,
}

#[derive(Clone, Debug)]
enum NewRendererStateError {
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

impl State {
	pub async fn new(window: Arc<Window>) -> Result<Self, NewRendererStateError> {
		println!("New State");
		let size = window.inner_size();


		
		// let filename = Path::new("./resources/fonts/Geist_Mono/static/GeistMono-Regular.ttf");
		// let font = Font::new(Path::new("./resources/fonts/JetBrainsMono-Regular.ttf"));
		let filename = Path::new("./resources/fonts/Material_Symbols_Outlined/static/MaterialSymbolsOutlined-Regular.ttf");
		// let filename = Path::new("./resources/fonts/NotoJP/static/NotoSansJP-Regular.ttf");
		let font = Font::new(filename);




		let parentless_holes = font.number_of_failed_parse_of_type(font::GlyphParseError::HoleDoesNotHaveParent);
		let triangulisation_stuck = font.number_of_failed_parse_of_type(font::GlyphParseError::StuckInTriangulisationLoop);
		let no_valid_channels = font.number_of_failed_parse_of_type(font::GlyphParseError::NoValidChannel);
		let total_parse_failures = font.number_of_failed_parse();

		println!("\n\n");
		println!("Font File Used: {filename:?}\n");
		println!("Number of glyphs with holes without parents: {parentless_holes}");
		if parentless_holes > 0 && false {
			println!("Glyphs that failed to parse:");
			for (glyph_index, glyph) in font.glyphs.iter().enumerate() {
				if let font::GlyphData::FailedParse(error) = &glyph.data {
					if error == &font::GlyphParseError::HoleDoesNotHaveParent {
						let character_codes = font.get_character_codes(glyph_index as u16);
						println!("	Glyph Index: {glyph_index}, Character Codes: {character_codes:?}");
					}
				}
			}
		}
		println!("Number of glyphs stuck in triangulisation loop: {triangulisation_stuck}");
		if triangulisation_stuck > 0 && false {
			println!("Glyphs that failed to parse:");
			for (glyph_index, glyph) in font.glyphs.iter().enumerate() {
				if let font::GlyphData::FailedParse(error) = &glyph.data {
					if error == &font::GlyphParseError::StuckInTriangulisationLoop {
						let character_codes = font.get_character_codes(glyph_index as u16);
						println!("	Glyph Index: {glyph_index}, Character Codes: {character_codes:?}");
					}
				}
			}
		}
		println!("Number of glyphs with no valid channels: {no_valid_channels}");
		if triangulisation_stuck > 0 && false {
			println!("Glyphs that failed to parse:");
			for (glyph_index, glyph) in font.glyphs.iter().enumerate() {
				if let font::GlyphData::FailedParse(error) = &glyph.data {
					if error == &font::GlyphParseError::NoValidChannel {
						let character_codes = font.get_character_codes(glyph_index as u16);
						println!("	Glyph Index: {glyph_index}, Character Codes: {character_codes:?}");
					}
				}
			}
		}
		println!("Total Parse Fails: {total_parse_failures}");
		println!("\n\n");
		println!("Number of Glyphs {}", font.glyphs.len());
		println!("\n\n");

		let glyph_index_to_check = 3546;
		// let glyph_index_to_check = 390;
		// let glyph_index_to_check = 1043;
		let character_codes = font.get_character_codes(glyph_index_to_check);
		println!("Glyph index {glyph_index_to_check} has character codes: {character_codes:?}");

		println!("\n\n\n");
		tapestry::read::read_one_glyph(filename, glyph_index_to_check as usize);
		println!("\n\n\n");



		let current_glyph_id = 3546;
		// let current_glyph_id = 1031;
		// let current_glyph_id = 390;

		// let character_code = '\u{307}';
		// let character_code = '６';
		// let character_code= 'ひ';
		// println!("Character being displayed: {character_code}");
		// let current_glyph_id = font.get_index(character_code).unwrap_or(0);

		let pixels_per_font_unit: f32 = 1.0;

		let (vertices, mut indices, convex_bezier_indices, concave_bezier_indices) = GlyphIndex(current_glyph_id as u16).to_raw(&font, pixels_per_font_unit, 200, 200, size.width as f32, size.height as f32, 0);
		let convex_bezier_indices_start = indices.len();
		indices.extend(convex_bezier_indices);
		let concave_bezier_indices_start = indices.len();
		indices.extend(concave_bezier_indices);

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
			source: wgpu::ShaderSource::Wgsl(include_str!("triangle_shader.wgsl").into()),
		});

		let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Vertex Buffer"),
			contents: bytemuck::cast_slice(&vertices),
			usage: wgpu::BufferUsages::VERTEX,
		});

		let instances: Vec<Instance> = (0..1).map(|v: u32| Instance {position: [v.rem_euclid(75) as f32, (v / 75) as f32]}).collect();

		let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Vertex Buffer"),
			contents: bytemuck::cast_slice(&instances),
			usage: wgpu::BufferUsages::VERTEX,
		});

		let index_buffer=  device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Index Buffer"),
			contents: bytemuck::cast_slice(&indices),
			usage: wgpu::BufferUsages::INDEX,
		});

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
					Instance::desc(),
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
			font,
			current_glyph_id,
			indices,
			vertices,
			pixels_per_font_unit,
			instance_buffer,
			instances,
			convex_bezier_indices_start,
			concave_bezier_indices_start,
			mode_bind_group_layout,
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

	fn update(&mut self) {
		println!("Updating");
		println!("Glyph Index: {}", self.current_glyph_id);
		let character_codes = self.font.get_character_codes(self.current_glyph_id as u16);
		println!("Character Codes for current glyph: {character_codes:?}");

		let size = self.window.inner_size();

		let (vertices, mut indices, convex_bezier_indices, concave_bezier_indices) = GlyphIndex(self.current_glyph_id as u16).to_raw(&self.font, self.pixels_per_font_unit, 200, 200, size.width as f32, size.height as f32, 0);
		let convex_bezier_indices_start = indices.len();
		indices.extend(convex_bezier_indices);
		let concave_bezier_indices_start = indices.len();
		indices.extend(concave_bezier_indices);

		self.vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Vertex Buffer"),
			contents: bytemuck::cast_slice(&vertices),
			usage: wgpu::BufferUsages::VERTEX,
		});

		self.index_buffer=  self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Index Buffer"),
			contents: bytemuck::cast_slice(&indices),
			usage: wgpu::BufferUsages::INDEX,
		});

		self.vertices = vertices;
		self.indices = indices;
		self.convex_bezier_indices_start = convex_bezier_indices_start;
		self.concave_bezier_indices_start = concave_bezier_indices_start;

	}

	pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
		println!("Rendering");

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
							load: wgpu::LoadOp::Clear(
								wgpu::Color {
									r: to_linear_rgb(16) as f64,
									g: to_linear_rgb(16) as f64,
									b: to_linear_rgb(16) as f64,
									a: 1.0,
								}
							),
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
			render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
			render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
			let convex_bezier_indices_start = self.convex_bezier_indices_start;
			let concave_bezier_indices_start = self.concave_bezier_indices_start;
			let number_of_indices = self.indices.len();
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

	fn handle_key(&mut self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
		match (code, is_pressed) {
			(KeyCode::Escape, true) => event_loop.exit(),
			(KeyCode::ArrowRight, true) => {
				self.current_glyph_id += 1;
				self.update();
				self.window.request_redraw();
			},
			(KeyCode::ArrowLeft, true) => {
				self.current_glyph_id -= 1;
				self.update();
				self.window.request_redraw();
			},
			(KeyCode::ArrowUp, true) => {
				self.pixels_per_font_unit *= 1.01;
				self.update();
				self.window.request_redraw();
			},
			(KeyCode::ArrowDown, true) => {
				self.pixels_per_font_unit *= 0.99;
				self.update();
				self.window.request_redraw();
			},
			_ => {}
		}
	}
}

pub struct App {
	state: Option<State>,

}

impl App {
	pub fn new() -> Self {
		Self {
			state: None,
		}
	}
}

impl ApplicationHandler<State> for App {
	fn resumed(&mut self, event_loop: &ActiveEventLoop) {
		let window_attributes = Window::default_attributes()
			.with_title("GUI RENDERER TEST");

		let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

		{
			self.state = Some(pollster::block_on(State::new(window)).unwrap());
		}


	}

	fn window_event(
		&mut self,
		event_loop: &ActiveEventLoop,
		_window_id: winit::window::WindowId,
		event: WindowEvent,
	) {
		let state = match &mut self.state {
			Some(state) => state,
			None => return,
		};

		match event {
			WindowEvent::CloseRequested => event_loop.exit(),
			WindowEvent::Resized(size) => state.resize(size.width, size.height),
			WindowEvent::RedrawRequested => {
				match state.render() { // RENDER CALLED
					Ok(_) => {}
					Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
						let size = state.window.inner_size();
						state.resize(size.width, size.height);
					}
					Err(e) => {
						log::error!("Unable to render. Error: {e}");
					}
				}
			},
			WindowEvent::KeyboardInput {
				event:
					KeyEvent {
						physical_key: PhysicalKey::Code(code),
						state: key_state,
						..
					},
					..
			} => state.handle_key(event_loop, code, key_state.is_pressed()),
			_ => {},
		}
	}
}

pub fn run() -> Result<(), EventLoopError> {
	env_logger::init();

	let event_loop = EventLoop::with_user_event().build()?;
	let mut app = App::new();
	event_loop.run_app(&mut app)?;

	Ok(())
}

fn main() {
	run().unwrap();
}