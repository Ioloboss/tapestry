use tapestry::ttf_reader::{self, CharacterToGlyphIndexTable, FontHeaderTable, GlyphTable, IndexToLocationTable, MaximumProfileTable, TableRecord, TableTag};
use tapestry::ttf_parser::{Direction, GlyphDataIntermediate, GlyphIntermediate};
use winit::error::EventLoopError;

use std::{fs::File, path::Path};
use std::sync::Arc;

use wgpu::util::{DeviceExt};
use winit::{
	application::ApplicationHandler, event::*, event_loop::{ActiveEventLoop, EventLoop}, keyboard::{KeyCode, PhysicalKey}, window::Window
};

trait ToInstances {
	fn to_instances(&self, font: &Font, offset_x: i32, offset_y: i32) -> Vec<Instance>;
}

impl ToInstances for GlyphIntermediate {
	fn to_instances(&self, font: &Font, offset_x: i32, offset_y: i32) -> Vec<Instance> {
		match &self.glyph_data {
			GlyphDataIntermediate::SimpleGlyph(simple_glyph) => {
				let mut output: Vec<Instance> = Vec::new();
				for contour in simple_glyph.contours.iter() {
					let colour = match contour.direction {
						Direction::Clockwise => [ 0.216, 0.022, 0.022 ],
						Direction::CounterClockwise => [ 0.216, 0.022, 0.074],
					};
					for i in contour.start_point as usize..=contour.end_point as usize {
						let point = &simple_glyph.points[i];
						let instance = Instance {
							position: [(point.x as i32 + offset_x) as u64, (point.y as i32 + offset_y) as u64],
							size: [ 20, 20 ],
							colour,
						};
						output.push(instance);
					}
				};
				output
			},
			GlyphDataIntermediate::CompositeGlyph(composite_glyph) => {
				let mut output: Vec<Instance> = Vec::new();
				for child in composite_glyph.children.iter() {
					if !(child.transformation_matrix.is_identity()){
						panic!("{:?}", child.transformation_matrix)
					};
					output.extend(GlyphIndex(child.glyph_index).to_instances(font, offset_x + child.offset.x, offset_y + child.offset.y));
				};
				output
			},
			GlyphDataIntermediate::None => {
				let output = GlyphIndex(0).to_instances(font, offset_x, offset_y);
				output
			},
		}
	}
}

impl ToInstances for char {
	fn to_instances(&self, font: &Font, offset_x: i32, offset_y: i32) -> Vec<Instance> {
		let character_glyph_id = font.character_to_glyph_index_table.subtables[0].get_glyph_id(self.clone() as u64);
		let character_glyph = match character_glyph_id {
			Some(glyph_id) => &font.glyphs[glyph_id as usize],
			None => &font.glyphs[0],
		};

		character_glyph.to_instances(font, offset_x, offset_y)
	}
}

struct GlyphIndex(u16);

impl ToInstances for GlyphIndex {
	fn to_instances(&self, font: &Font, offset_x: i32, offset_y: i32) -> Vec<Instance> {
		let character_glyph = &font.glyphs[self.0 as usize];

		character_glyph.to_instances(font, offset_x, offset_y)
	}
}

struct Font {
	maximum_profile_table: MaximumProfileTable,
	font_header_table: FontHeaderTable,
	glyphs: Vec<GlyphIntermediate>,
	character_to_glyph_index_table: CharacterToGlyphIndexTable,
}

impl Font {
	fn new(filename: &Path) -> Self {
		let file = File::open(filename).unwrap();
		let mut ttf_reader = ttf_reader::TrueTypeFontReader::new(file);
		let sfnt_version: u32 = ttf_reader.read_bytes().unwrap();
		let number_of_tables: u16 = ttf_reader.read_bytes().unwrap();
		assert_eq!(sfnt_version, 0x00010000);

		ttf_reader.skip(6).unwrap();

		let mut glyph_table_record: Option<TableRecord> = None;
		let mut maximum_profile_table_record: Option<TableRecord> = None;
		let mut index_to_location_table_record: Option<TableRecord> = None;
		let mut font_header_table_record: Option<TableRecord> = None;
		let mut character_to_glyph_index_table_record: Option<TableRecord> = None;
		let mut other_table_records = Vec::<TableRecord>::new();

		for _ in 0..number_of_tables {
			let table_record: TableRecord = ttf_reader.read_bytes().unwrap();
			match table_record.table_tag {
				TableTag::Glyph => glyph_table_record = Some(table_record),
				TableTag::MaximumProfile => maximum_profile_table_record = Some(table_record),
				TableTag::IndexToLocation => index_to_location_table_record = Some(table_record),
				TableTag::FontHeader => font_header_table_record = Some(table_record),
				TableTag::CharacterToGlyphIndex => character_to_glyph_index_table_record = Some(table_record),
				TableTag::Other(_) =>other_table_records.push(table_record),
			};
		}

		let maximum_profile_table: MaximumProfileTable = match maximum_profile_table_record {
			Some(maximum_profile_table_record) =>  ttf_reader.read(maximum_profile_table_record.offset).unwrap(),
			None => panic!("Font should have a maxp table."),
		};

		let font_header_table: FontHeaderTable = match font_header_table_record {
			Some(font_header_table_record) => ttf_reader.read(font_header_table_record.offset).unwrap(),
			None => panic!("Font should have a head table."),
		};

		let index_to_location_table: IndexToLocationTable = match index_to_location_table_record {
			Some(index_to_location_table_record) => ttf_reader.read((index_to_location_table_record.offset, font_header_table.index_to_location_format, maximum_profile_table.num_glyphs)).unwrap(),
			None => panic!("Font should have a loca table."),
		};

		let glyph_table: GlyphTable = match glyph_table_record {
			Some(glyph_table_record) => ttf_reader.read((index_to_location_table.glyph_offsets, glyph_table_record.offset as u64)).unwrap(),
			None => panic!("Font should have a glyf table."),
		};

		let glyphs = glyph_table.glyphs.into_iter().map(|v| v.into()).collect();

		let character_to_glyph_index_table: CharacterToGlyphIndexTable = match character_to_glyph_index_table_record {
			Some(charachter_to_glyph_index_table_record) => ttf_reader.read(charachter_to_glyph_index_table_record.offset).unwrap(),
			None => panic!("Font should have a cmap table."),
		};

		Font {
			maximum_profile_table,
			font_header_table,
			glyphs,
			character_to_glyph_index_table,
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

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
	position: [f32; 2],
}

impl Vertex {
	fn desc() -> wgpu::VertexBufferLayout<'static> {
		wgpu::VertexBufferLayout {
			array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
			step_mode: wgpu::VertexStepMode::Vertex,
			attributes: &[
				wgpu::VertexAttribute {
					offset: 0,
					shader_location: 0,
					format: wgpu::VertexFormat::Float32x2,
				},
			]
		}
	}
}

struct Instance {
	position: [u64; 2],
	size: [u64; 2],
	colour: [f32; 3],

}

impl Instance {
	fn to_raw(&self, screen_width: u32, screen_height: u32) -> InstanceRaw {
		let normalised_x: f32 = (self.position[0] as f32 / (screen_width as f32 / 2.0)) - 1.0;
		let normalised_y: f32 = (self.position[1] as f32 / (screen_height as f32 / 2.0)) - 1.0;
		let normalised_width: f32 = (self.size[0] as f32 / screen_width as f32) * 2.0;
		let normalised_height: f32 = (self.size[1] as f32 / screen_height as f32)* 2.0;
		InstanceRaw {
			position: [normalised_x, normalised_y],
			size: [normalised_width, normalised_height],
			colour: self.colour,
		}
	}
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceRaw {
	position: [f32; 2],
	size: [f32; 2],
	colour: [f32; 3],
}

impl InstanceRaw {
	fn desc() -> wgpu::VertexBufferLayout<'static> {
		use std::mem;
		wgpu::VertexBufferLayout {
			array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
			step_mode: wgpu::VertexStepMode::Instance,
			attributes: &[
				wgpu::VertexAttribute {
					offset: 0,
					shader_location: 1,
					format: wgpu::VertexFormat::Float32x2,
				},
				wgpu::VertexAttribute {
					offset: mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
					shader_location: 2,
					format: wgpu::VertexFormat::Float32x2,
				},
				wgpu::VertexAttribute {
					offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
					shader_location: 3,
					format: wgpu::VertexFormat::Float32x3,
				},
			]
		}
	}
}

const VERTICES: &[Vertex] = &[
	Vertex { position: [0.0, 1.0] },
	Vertex { position: [0.0, 0.0] },
	Vertex { position: [1.0, 1.0] },
	Vertex { position: [1.0, 0.0] },
];

const INDICES: &[u16] = &[
	0, 1, 2, 3,
];

pub struct State {
	surface: wgpu::Surface<'static>,
	device: wgpu::Device,
	queue: wgpu::Queue,
	config: wgpu::SurfaceConfiguration,
	is_surface_configured: bool,
	render_pipeline: wgpu::RenderPipeline,
	vertex_buffer: wgpu::Buffer,
	index_buffer: wgpu::Buffer,
	instances: Vec<Instance>,
	instance_buffer: wgpu::Buffer,
	window: Arc<Window>,
	font: Font,
	current_glyph_id: u16,
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

		// let font = Font::new(Path::new("./resources/fonts/GeistMono-Regular.ttf"));
		// let font = Font::new(Path::new("./resources/fonts/Material_Symbols_Outlined/static/MaterialSymbolsOutlined-Regular.ttf"));
		let font = Font::new(Path::new("./resources/fonts/NotoJP/static/NotoSansJP-Regular.ttf"));


		// let current_glyph_id = 0;
		let current_glyph_id = font.character_to_glyph_index_table.subtables[0].get_glyph_id('ひ' as u64).unwrap_or(0);





		let size = window.inner_size();

		let instances = GlyphIndex(current_glyph_id).to_instances(&font, 100, 100);

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
			source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
		});

		let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Vertex Buffer"),
			contents: bytemuck::cast_slice(VERTICES),
			usage: wgpu::BufferUsages::VERTEX,
		});

		let instance_data = instances.iter().map(|instance| instance.to_raw(size.width, size.height)).collect::<Vec<_>>();


		let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Instance Buffer"),
			contents: bytemuck::cast_slice(&instance_data),
			usage: wgpu::BufferUsages::VERTEX
		});

		let index_buffer=  device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Index Buffer"),
			contents: bytemuck::cast_slice(INDICES),
			usage: wgpu::BufferUsages::INDEX,
		});

		let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
			label: Some("Render Pipeline Layout"),
			bind_group_layouts: &[],
			push_constant_ranges: &[],
		});

		let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
			label: Some("Render Pipeline"),
			layout: Some(&render_pipeline_layout),
			vertex: wgpu::VertexState {
				module: &shader,
				entry_point: Some("vs_main"),
				buffers: &[
					Vertex::desc(),
					InstanceRaw::desc(),
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
				topology: wgpu::PrimitiveTopology::TriangleStrip,
				strip_index_format: None,
				front_face: wgpu::FrontFace::Ccw,
				cull_mode: Some(wgpu::Face::Back),
				polygon_mode: wgpu::PolygonMode::Fill,
				unclipped_depth: false,
				conservative: false,
			},
			depth_stencil: None,
			multisample: wgpu::MultisampleState {
				count: 1,
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
			instances,
			instance_buffer,
			window,
			font,
			current_glyph_id,
		})
	}

	pub fn resize(&mut self, width: u32, height: u32) {
		println!("Resized to: {width}x{height}");
		if width > 0 && height > 0 {
			self.config.width = width;
			self.config.height = height;
			self.surface.configure(&self.device, &self.config);
			self.is_surface_configured = true;
		}
	}

	fn update(&mut self) {
		self.instances = GlyphIndex(self.current_glyph_id).to_instances(&self.font, 200, 200);

		let size = self.window.inner_size();

		let instance_data = self.instances.iter().map(|instance| instance.to_raw(size.width, size.height)).collect::<Vec<_>>();

		self.instance_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Instance Buffer"),
			contents: bytemuck::cast_slice(&instance_data),
			usage: wgpu::BufferUsages::VERTEX
		});
	}

	pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
		println!("Rendering");
		//self.window.request_redraw();

		if !self.is_surface_configured {
			return Ok(());
		}

		let output = self.surface.get_current_texture()?;

		let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

		let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
			label: Some("Render Encoder"),
		});

		{
			let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
				label: Some("Render Pass"),
				color_attachments: &[
					Some(wgpu::RenderPassColorAttachment {
						view: &view,
						resolve_target: None,
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
			render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
			render_pass.draw_indexed(0..4, 0, 0..self.instances.len() as _);
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
				self.window.request_redraw();
			},
			(KeyCode::ArrowLeft, true) => {
				self.current_glyph_id -= 1;
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
				state.update();
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