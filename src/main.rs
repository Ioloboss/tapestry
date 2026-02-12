use tapestry::font::font_renderer::{FontRenderer, TextBox};
use tapestry::ttf_reader::{self, CharacterToGlyphIndexTable, FontHeaderTable, GlyphTable, IndexToLocationTable, MaximumProfileTable, TableRecord, TableTag};
use tapestry::ttf_parser::{Direction, GlyphDataIntermediate, GlyphIntermediate};
use tapestry::font::{self, Font, GlyphIndex, font_renderer::{self, VertexRaw, ToRawTriangles, NewRendererStateError}};
use winit::error::EventLoopError;

use std::{fs::File, path::Path};
use std::sync::{Arc, Mutex};

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
	font_renderer: FontRenderer,
	font: Arc<Font>,
	number_of_indices: usize,
	pixels_per_font_unit: f32,
	text: Arc<Mutex<String>>,
}

impl State {
	pub async fn new(window: Arc<Window>) -> Result<Self, NewRendererStateError> {
		println!("New State");
		let size = window.inner_size();


		
		let filename = Path::new("./resources/fonts/Geist_Mono/static/GeistMono-Regular.ttf");
		// let font = Font::new(Path::new("./resources/fonts/JetBrainsMono-Regular.ttf"));
		// let filename = Path::new("./resources/fonts/Material_Symbols_Outlined/static/MaterialSymbolsOutlined-Regular.ttf");
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

		// let glyph_index_to_check = 3546;
		// let glyph_index_to_check = 390;
		// let glyph_index_to_check = 1043;
		// let character_codes = font.get_character_codes(glyph_index_to_check);
		// println!("Glyph index {glyph_index_to_check} has character codes: {character_codes:?}");

		// println!("\n\n\n");
		// tapestry::read::read_one_glyph(filename, glyph_index_to_check as usize);
		// println!("\n\n\n");



		// let current_glyph_id = 3546;
		// let current_glyph_id = 1031;
		// let current_glyph_id = 390;

		// let character_code = '\u{307}';
		// let character_code = '６';
		// let character_code = 'ひ';
		// let character_code = 'h';
		// println!("Character being displayed: {character_code}");
		// let current_glyph_id = font.get_index(character_code).unwrap_or(0);
		let string = String::from("Hello world!");

		let pixels_per_font_unit: f32 = 0.1;

		let mut font_renderer = FontRenderer::new(window).await?;

		let font = Arc::new(font);

		let text = Arc::new(Mutex::new(string));

		let string_goodbye = String::from("Goodbye world!");
		let text2 = Arc::new(Mutex::new(string_goodbye));

		let text_box = TextBox {
			font: Arc::clone(&font),
			text: Arc::clone(&text),
			pixels_per_font_unit,
			position: (200.0, 200.0).into(),
		};

		let text_box_2 = TextBox {
			font: Arc::clone(&font),
			text: text2,
			pixels_per_font_unit: 0.2,
			position: (200.0, 1000.0).into()
		};

		font_renderer.add_text_box(text_box);
		font_renderer.add_text_box(text_box_2);

		Ok(Self {
			font_renderer,
			font,
			number_of_indices: 0,
			pixels_per_font_unit,
			text,
		})
	}

	pub fn resize(&mut self, width: u32, height: u32) {
		self.font_renderer.resize(width, height);
	}

	fn update(&mut self) {
		self.font_renderer.update();

	}

	pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
		self.font_renderer.render()
	}

	fn handle_key(&mut self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
		match (code, is_pressed) {
			(KeyCode::Escape, true) => event_loop.exit(),
			(KeyCode::ArrowUp, true) => {
				self.pixels_per_font_unit *= 1.01;
				self.update();
				self.font_renderer.request_redraw();
			},
			(KeyCode::ArrowDown, true) => {
				self.pixels_per_font_unit *= 0.99;
				self.update();
				self.font_renderer.request_redraw();
			},
			(KeyCode::Backspace, true) => {
				let mut string = self.text.lock().unwrap();
				string.pop();
				drop(string);
				self.update();
				self.font_renderer.request_redraw();
			},
			(KeyCode::Space, true) => {
				let mut string = self.text.lock().unwrap();
				string.push('!');
				drop(string);
				self.update();
				self.font_renderer.request_redraw();
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
						let size = state.font_renderer.window.inner_size();
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