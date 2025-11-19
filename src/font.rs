use std::fmt::Display;

use crate::ttf_reader;

pub mod font_renderer;

pub struct Font {
	pub glyphs: Vec<Glyph>,
	pub mappings: Vec<Mapping>,
}

impl Font {
	pub fn get_index(&self, character: char) -> Option<usize> {
		match self.mappings[0].get_glyph_id(character as u64) {
			Some(index) => Some(index as usize),
			None => None,
		}
	}

	pub fn get_character_codes(&self, glyph_index: u16) -> Vec<char> {
		self.mappings[0].get_character_codes(glyph_index)
	}

	pub fn number_of_failed_parse_of_type(&self, type_of_parse_error: GlyphParseError) -> usize {
		let mut count = 0;
		for glyph in &self.glyphs {
			if let GlyphData::FailedParse(error) = &glyph.data {
				if error == &type_of_parse_error {
					count += 1;
				}
			}
		}
		count
	}

	pub fn number_of_failed_parse(&self) -> usize {
		let mut count = 0;
		for glyph in &self.glyphs {
			if let GlyphData::FailedParse(error) = &glyph.data {
				count += 1;
			}
		}
		count
	}
}

pub struct GlyphIndex(pub u16);

pub struct Glyph {
	bounds: Bounds,
	pub data: GlyphData,
}

#[derive(Debug, PartialEq)]
pub enum GlyphParseError {
	StuckInTriangulisationLoop,
	HoleDoesNotHaveParent,
	NoValidChannel,
}

pub enum GlyphData {
	SimpleGlyph(SimpleGlyph),
	CompositeGlyph(CompositeGlyph),
	FailedParse(GlyphParseError),
	None,
}

pub struct SimpleGlyph {
	vertices: Vec<Vertex>,
	indices: Vec<u32>,
	convex_bezier_indices: Vec<u32>,
	concave_bezier_indices: Vec<u32>,
}

pub struct CompositeGlyph {
	children: Vec<ComponentGlyph>,
}

pub struct ComponentGlyph {
	pub child_index: usize,
	pub offset: Position<i32>,
}

impl Glyph {
	pub fn new_simple(vertices: Vec<Vertex>, indices: Vec<u32>, convex_bezier_indices: Vec<u32>, concave_bezier_indices: Vec<u32>, bounds: Bounds) -> Self {
		let data = GlyphData::SimpleGlyph(SimpleGlyph { vertices, indices, convex_bezier_indices, concave_bezier_indices, });
		Self { bounds, data, }
	}

	pub fn new_composite(children: Vec<ComponentGlyph>, bounds: Bounds) -> Self {
		let data = GlyphData::CompositeGlyph(CompositeGlyph{ children, });
		Self { bounds, data, }
	}

	pub fn new_empty(bounds: Bounds) -> Self {
		let data = GlyphData::None;
		Self { bounds, data, }
	}

	pub fn new_failed_parse(error: GlyphParseError, bounds: Bounds) -> Self {
		let data = GlyphData::FailedParse(error);
		Self { bounds, data, }
	}

	pub fn to_raw(&self, font: &Font, pixels_per_font_unit: f32, offset: Position<i32>, screen_size: Size<f32>, position: Position<f32>, vertices_start: usize) -> (Vec<font_renderer::VertexRaw>, Vec<u32>, Vec<u32>, Vec<u32>) {
		match &self.data {
			GlyphData::SimpleGlyph(data) => {
				let vertices_raw = data.vertices.iter().map(|v| v.to_raw(pixels_per_font_unit, offset, screen_size, position)).collect();
				let indices: Vec<u32> = data.indices.iter().map(|index| index + vertices_start as u32).collect();
				let convex_bezier_indices: Vec<u32> = data.convex_bezier_indices.iter().map(|index| index + vertices_start as u32).collect();
				let concave_bezier_indices: Vec<u32> = data.concave_bezier_indices.iter().map(|index| index + vertices_start as u32).collect();

				(vertices_raw, indices, convex_bezier_indices, concave_bezier_indices)
			},
			GlyphData::CompositeGlyph(data) => {
				let mut vertices_raw: Vec<font_renderer::VertexRaw> = Vec::new();
				let mut indices: Vec<u32> = Vec::new();
				let mut convex_bezier_indices: Vec<u32> = Vec::new();
				let mut concave_bezier_indices: Vec<u32> = Vec::new();
				println!("Composite Glyph woth Child glyph ids:");
				for child in data.children.iter() {
					println!("	{}", child.child_index);
				}
				for child in data.children.iter() {
					let updated_vertices_start = vertices_raw.len() + vertices_start;
					let offset = offset + child.offset;
					let (extra_vertices_raw, extra_indices, extra_convex_bezier_indices, extra_concave_bezier_indices) = font.glyphs[child.child_index].to_raw(font, pixels_per_font_unit, offset, screen_size, position, updated_vertices_start);
					vertices_raw.extend(extra_vertices_raw);
					indices.extend(extra_indices);
					convex_bezier_indices.extend(extra_convex_bezier_indices);
					concave_bezier_indices.extend(extra_concave_bezier_indices);
				};

				(vertices_raw, indices, convex_bezier_indices, concave_bezier_indices)
			},
			GlyphData::FailedParse(error) => {
				panic!("Parsing glyph failed with error: {error:?}");
			}
			GlyphData::None => {
				let vertices = vec![font_renderer::VertexRaw {position: [0.0, 0.0], uv_coords: [0.0, 0.0]},
					font_renderer::VertexRaw {position: [0.0, 0.0], uv_coords: [0.0, 0.0]},
					font_renderer::VertexRaw {position: [0.0, 0.0], uv_coords: [0.0, 0.0]}
				];
				let indices = vec![0, 1, 2];
				(vertices, indices, Vec::new(), Vec::new())
			},
		}
	}
}

#[derive(Debug, Clone, Copy)]
pub struct Bounds {
	pub x_min: i16,
	pub x_max: i16,
	pub y_min: i16,
	pub y_max: i16,
}

#[derive(Debug, Clone, Copy)]
pub struct Size<T> {
	pub width: T,
	pub height: T,
}

impl<T> From<(T, T)> for Size<T> {
	fn from(value: (T, T)) -> Self {
		Self { width: value.0, height: value.1 }
	}
}

#[derive(Debug, Clone, Copy)]
pub struct Position<T> {
	pub x: T,
	pub y: T,
}

impl<T> From<(T, T)> for Position<T> {
	fn from(value: (T, T)) -> Self {
		Self { x: value.0, y: value.1 }
	}
}

impl std::ops::Add for Position<i32> {
	type Output = Position<i32>;

	fn add(self, rhs: Self) -> Self::Output {
		Self::Output {
			x: self.x + rhs.x,
			y: self.y + rhs.y,
		}
	}
}

#[derive(Debug, PartialEq)]
pub struct Vertex {
	pub x: i16,
	pub y: i16,
	pub on_curve: bool,
	pub uv_coords: [f32; 2],
}

impl Vertex {
	pub fn same_position(&self, other_vertex: &Self) -> bool{
		(self.x == other_vertex.x) && (self.y == other_vertex.y)
	}
}

impl Vertex {
	fn to_raw(&self, pixels_per_font_unit: f32, offset: Position<i32>, screen_size: Size<f32>, position: Position<f32>) -> font_renderer::VertexRaw {
		let x = self.x as i32 + offset.x;
		let y = self.y as i32 + offset.y;
		let transformed_x = ((((x as f32 * pixels_per_font_unit) + position.x) / screen_size.width) * 2.0) - 1.0;
		let transformed_y = ((((y as f32 * pixels_per_font_unit) + position.x) / screen_size.height) * 2.0) - 1.0;
		font_renderer::VertexRaw{ position: [transformed_x, transformed_y], uv_coords: self.uv_coords }
	}
}

impl Vertex {
	pub fn new(x: i16, y: i16) -> Self {
		Self { x, y, on_curve: true, uv_coords: [0.0, 0.0]}
	}

	pub fn with_changed_uv_coord(&self, uv_coords: [f32; 2]) -> Self {
		let x = self.x;
		let y = self.y;
		let on_curve = self.on_curve;
		Self { x, y, on_curve, uv_coords, }
	}
}

impl From<(i16, i16)> for Vertex {
	fn from(value: (i16, i16)) -> Self {
		Vertex { x: value.0, y: value.1, on_curve: true, uv_coords: [0.0, 0.0] }
	}
} 

impl Display for Vertex {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "({}, {})", self.x, self.y)
	}
}

pub trait ToTriangles {
	fn to_triangles(self, debug_mode: bool) -> Result<(Vec<Vertex>, Vec<u32>, Vec<u32>, Vec<u32>), GlyphParseError>; //vertices, indices, convex_bezier_indices, concave_bezier_indices
}

pub enum Mapping {
	TrueTypeFormat4(MappingTrueTypeFormat4),
	TrueTypeFormat12(MappingTrueTypeFormat12),
}

impl Mapping {
	fn get_glyph_id(&self, character_code: u64) -> Option<u16> {
		match self {
			Mapping::TrueTypeFormat4(mapping) => mapping.get_glyph_id(character_code),
			Mapping::TrueTypeFormat12(mapping) => mapping.get_glyph_id(character_code),
		}
	}

	fn get_character_codes(&self, glyph_index: u16) -> Vec<char> {
		match self {
			Mapping::TrueTypeFormat4(mapping) => mapping.get_character_codes(glyph_index),
			Mapping::TrueTypeFormat12(mapping) => mapping.get_character_codes(glyph_index),
		}
	}
}

impl From<ttf_reader::CharacterToGlyphIndexSubtable> for Mapping {
	fn from(value: ttf_reader::CharacterToGlyphIndexSubtable) -> Self {
		match value {
			ttf_reader::CharacterToGlyphIndexSubtable::Format4(subtable) => Mapping::TrueTypeFormat4(subtable.into()),
			ttf_reader::CharacterToGlyphIndexSubtable::Format12(subtable) => Mapping::TrueTypeFormat12(subtable.into()),
		}
	}
}

pub struct MappingTrueTypeFormat4 {
	length: u16,
	language: u16,
	segment_count: u16,
	search_range: u16,
	entry_selector: u16,
	range_shift: u16,
	end_codes: Vec<u16>,
	start_codes: Vec<u16>,
	id_deltas: Vec<i16>,
	id_range_offsets: Vec<u16>,
	glyph_id_array: Vec<u16>,
}

impl MappingTrueTypeFormat4 {
	fn get_glyph_id(&self, character_code: u64) -> Option<u16> {
		for i in 0..self.segment_count as usize {
			if character_code >= self.start_codes[i] as u64 && character_code <= self.end_codes[i] as u64 {
				if self.id_range_offsets[i] == 0 {
					return Some(((character_code as i128 + self.id_deltas[i] as i128) % 65536) as u16);
				} else {
					let glyph_id_index = (character_code - self.start_codes[i] as u64) + (self.id_range_offsets[i] as u64 / 2) + i as u64 - self.segment_count as u64;
					let glyph_id = self.glyph_id_array[glyph_id_index as usize];
					if glyph_id == 0 {
						return None;
					} else {
						return Some(((glyph_id as i32 + self.id_deltas[i] as i32) % 65536) as u16);
					}
				}
			}
		}

		None
	}

	fn get_character_codes(&self, glyph_id: u16) -> Vec<char> {
		let mut character_codes: Vec<char> = Vec::new();
		for i in 0..self.segment_count as usize {
			for character_code in self.start_codes[i] as u64 ..=self.end_codes[i] as u64 {
				let calculated_glyph_id;
				if self.id_range_offsets[i] == 0 {
					calculated_glyph_id = ((character_code as i128 + self.id_deltas[i] as i128) % 65536) as u16;
				} else {
					let glyph_id_index = (character_code - self.start_codes[i] as u64) + (self.id_range_offsets[i] as u64 / 2) + i as u64 - self.segment_count as u64;
					let glyph_id = self.glyph_id_array[glyph_id_index as usize];
					calculated_glyph_id = if glyph_id == 0 {
						0
					} else {
						((glyph_id as i32 + self.id_deltas[i] as i32) % 65536) as u16
					}
				}
				if calculated_glyph_id == glyph_id {
					character_codes.push(char::from_u32(character_code as u32).unwrap());
				}
			}
		}
		character_codes
	}
}

impl From<ttf_reader::CharacterToGlyphIndexSubtableFormat4> for MappingTrueTypeFormat4 {
	fn from(value: ttf_reader::CharacterToGlyphIndexSubtableFormat4) -> Self {
		Self {
			length: value.length,
			language: value.language,
			segment_count: value.segment_count,
			search_range: value.search_range,
			entry_selector: value.entry_selector,
			range_shift: value.range_shift,
			end_codes: value.end_codes,
			start_codes: value.start_codes,
			id_deltas: value.id_deltas,
			id_range_offsets: value.id_range_offsets,
			glyph_id_array: value.glyph_id_array,
		}
	}
}

pub struct MappingTrueTypeFormat12 {
	length: u32,
	language: u32,
	groups: Vec<(u32, u32, u32)>,
}

impl MappingTrueTypeFormat12 {
	fn get_glyph_id(&self, character_code: u64) -> Option<u16> {
		for (start_code, end_code, start_index) in self.groups.iter() {
			if character_code as u32 >= *start_code && character_code as u32 <= *end_code {
				let delta = (character_code as u32 - start_code) as u16;
				return Some(start_index.clone() as u16 + delta);
			}
		}
		None
	}

	fn get_character_codes(&self, glyph_id: u16) -> Vec<char> {
		let mut character_codes: Vec<char> = Vec::new();
		for (start_code, end_code, start_index) in self.groups.iter() {
			for character_code in *start_code..=*end_code {
				let delta = (character_code as u32 - start_code) as u16;
				let calcualted_glyph_index = start_index.clone() as u16 + delta;
				if calcualted_glyph_index == glyph_id {
					character_codes.push(char::from_u32(character_code).unwrap());
				}
			}
		}
		character_codes
	}
}

impl From<ttf_reader::CharacterToGlyphIndexSubtableFormat12> for MappingTrueTypeFormat12 {
	fn from(value: ttf_reader::CharacterToGlyphIndexSubtableFormat12) -> Self {
		Self {
			length: value.length,
			language: value.language,
			groups: value.groups,
		}
	}
}