use std::{fmt::{Debug, Display}, fs::File, io::{self, BufReader, Read, Seek}};

#[derive(Debug)]
pub enum TrueTypeFontReaderError {
	NotEnoughBytesInBuffer(usize, usize),
	IOError(io::Error),
}

impl From<io::Error> for TrueTypeFontReaderError {
	fn from(value: io::Error) -> Self {
	    TrueTypeFontReaderError::IOError(value)
	}
}

pub struct TrueTypeFontReader {
	pub buffer_reader: BufReader<File>,
}

impl TrueTypeFontReader {
	pub fn new(file: File) -> Self {
		let buffer_reader = BufReader::new(file);
		Self {
			buffer_reader,
		}
	}

	pub fn skip(&mut self, bytes: usize) -> Result<(), TrueTypeFontReaderError> {
		self.buffer_reader.seek_relative(bytes as i64)?;
		Ok(())
	}

	pub fn read_bytes<Type: FromBytes>(&mut self) -> Result<Type, TrueTypeFontReaderError> {
		let mut buffer = Type::Bytes::default();
		match self.buffer_reader.read_exact(&mut buffer.as_mut()) {
			Ok(_) => Ok(Type::from_be_bytes(buffer)),
			Err(error) => Err(error.into()),
		}
	}

	pub fn read<Type: FromTTFReader>(&mut self, input: Type::Input) -> Result<Type, TrueTypeFontReaderError> {
		Type::read(self, input)
	}
}

#[derive(Debug, Clone, Copy)]
pub enum TableTag {
	Other([char; 4]),
	Glyph,
	MaximumProfile,
	IndexToLocation,
	FontHeader,
	CharacterToGlyphIndex,
	HorizontalHeaderTable,
	HorizontalMetricsTable,
	OS2AndWindowsMetricsTable,
}

impl Display for TableTag {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			TableTag::Other(chars) => write!(f, "{}{}{}{}", chars[0], chars[1], chars[2], chars[3]),
			TableTag::Glyph => write!(f, "glyf: Glyph Table"),
			TableTag::MaximumProfile => write!(f, "maxp: Maximum Profile Table"),
			TableTag::IndexToLocation => write!(f, "loca: Index To Location Table"),
			TableTag::FontHeader => write!(f, "head: Font Header Table"),
			TableTag::CharacterToGlyphIndex => write!(f, "cmap: Character To Glyph Index Table"),
			TableTag::HorizontalHeaderTable => write!(f, "hhea: Horizontal Header Table"),
			TableTag::HorizontalMetricsTable => write!(f, "hmtx: Horizontal Metrics Table"),
			TableTag::OS2AndWindowsMetricsTable => write!(f, "OS/2: OS/2 and Windows Metrics Table"),
		}
	}
}

#[derive(Debug, Clone, Copy)]
pub struct TableRecord {
	pub table_tag: TableTag,
	pub checksum: u32,
	pub offset: u32,
	pub length: u32,
}

pub struct MaximumProfileTable {
	major_version: u16,
	minor_version: u16,
	pub num_glyphs: u16,
	max_points: u16,
	max_contours: u16,
	max_composite_points: u16,
	max_composite_contours: u16,
	max_zones: u16,
	max_twilight_points: u16,
	max_storage: u16,
	max_function_defs: u16,
	max_instruction_defs: u16,
	max_stack_elements: u16,
	max_size_of_instructions: u16,
	max_component_elements: u16,
	max_component_depth: u16,
}

pub struct IndexToLocationTable {
	pub glyph_offsets: Vec<GlyphOffset>
}

#[derive(Clone, Copy, Debug)]
pub struct GlyphOffset {
	pub id: u16,
	pub glyph_offset: Option<u32>,
	glyph_length: Option<u32>,
}

pub struct FontHeaderTable {
	major_version: u16,
	minor_version: u16,
	font_revision: (u16, u16), // first.second, fixed point real number
	checksum_adjustment: u32,
	magic_number: u32,
	flags: u16,
	pub units_per_em: u16,
	created: i64,
	modified: i64,
	x_min: i16,
	y_min: i16,
	x_max: i16,
	y_max: i16,
	mac_style: u16, // more flags Bold, Italic, Underline, Outline, Shadow, Condensed, Extended, Reserved ..
	lowest_rec_ppem: u16,
	font_direction_hint: i16, // deprecated (should be set to 2)
	pub index_to_location_format: i16, // 0 -> short offset, 1 -> long offset
	glyph_data_format: i16, // 0 -> current format, don't think there should be any other values.
}

#[derive(Debug)]
pub struct GlyphRaw {
	pub number_of_contours: i16,
	pub x_min: i16,
	pub y_min: i16,
	pub x_max: i16,
	pub y_max: i16,
	pub glyph_data: GlyphDataRaw,
}

#[derive(Debug)]
pub enum GlyphDataRaw {
	SimpleGlyphRaw(SimpleGlyphRaw),
	CompositeGlyphRaw(CompositeGlyphRaw),
	None,
}

#[derive(Debug)]
pub struct SimpleGlyphRaw {
	pub end_points_of_contours: Vec<u16>,
	pub instruction_length: u16,
	pub instructions: Vec<u8>,
	pub flags: Vec<u8>,
	pub x_coordinates: Vec<i16>,
	pub y_coordinates: Vec<i16>,
}

#[derive(Debug)]
pub struct CompositeGlyphRaw {
	pub children: Vec<ComponentGlyphRaw>,
}

#[derive(Debug)]
pub struct ComponentGlyphRaw {
	pub flag: u16,
	pub glyph_index: u16,
	pub x_offset_point: i32,
	pub y_offset_point: i32,
	pub transform_0: Option<u16>, // THIS IS 2DOT14 
	pub transform_1: Option<u16>, // THIS IS 2DOT14 
	pub transform_2: Option<u16>, // THIS IS 2DOT14 
	pub transform_3: Option<u16>, // THIS IS 2DOT14 
}

pub struct GlyphTable {
	pub glyphs: Vec<GlyphRaw>,
}

#[derive(Debug)]
pub struct EncodingRecord {
	platform_id: u16,
	encoding_id: u16,
	subtable_offset: u32,
}

#[derive(Debug)]
pub enum CharacterToGlyphIndexSubtable {
	Format4(CharacterToGlyphIndexSubtableFormat4),
	Format12(CharacterToGlyphIndexSubtableFormat12),
}

impl CharacterToGlyphIndexSubtable {
	pub fn get_glyph_id(&self, character_code: u64) -> Option<u16> {
		match self {
			CharacterToGlyphIndexSubtable::Format4(subtable) => {subtable.get_glyph_id(character_code)},
			CharacterToGlyphIndexSubtable::Format12(subtable) => {subtable.get_glyph_id(character_code)},
		}
	}
}

#[derive(Debug)]
pub struct CharacterToGlyphIndexSubtableFormat4 {
	pub length: u16,
	pub language: u16,
	pub segment_count: u16,
	pub search_range: u16,
	pub entry_selector: u16,
	pub range_shift: u16,
	pub end_codes: Vec<u16>,
	pub start_codes: Vec<u16>,
	pub id_deltas: Vec<i16>,
	pub id_range_offsets: Vec<u16>,
	pub glyph_id_array: Vec<u16>,
}

impl CharacterToGlyphIndexSubtableFormat4 {
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
}

#[derive(Debug)]
pub struct CharacterToGlyphIndexSubtableFormat12 {
	pub length: u32,
	pub language: u32,
	pub groups: Vec<(u32, u32, u32)>,
}

impl CharacterToGlyphIndexSubtableFormat12 {
	fn get_glyph_id(&self, character_code: u64) -> Option<u16> {
		for (start_code, end_code, start_index) in self.groups.iter() {
			if character_code as u32 >= start_code.clone() && character_code as u32 <= end_code.clone() {
				let delta = (character_code as u32 - start_code) as u16;
				return Some(start_index.clone() as u16 + delta);
			}
		}
		None
	}
}

#[derive(Debug)]
pub struct CharacterToGlyphIndexTable {
	version: u16,
	number_of_subtables: u16,
	encoding_records: Vec<EncodingRecord>,
	pub subtables: Vec<CharacterToGlyphIndexSubtable>,
}

pub struct HorizontalHeaderTable {
	major_version: u16,
	minor_version: u16,
	ascender: i16,
	pub descender: i16,
	line_gap: i16,
	advance_width_max: u16,
	minimum_left_side_bearing: i16,
	minimum_right_side_bearing: i16,
	x_max_extent: i16,
	caret_slope_rise: i16,
	caret_slope_run: i16,
	caret_offset: i16,
	pub number_of_horizontal_metrics: u16,
}

#[derive(Debug)]
pub struct OS2AndWindowsMetricsTable {
	version: u16,
	x_average_character_width: i16,
	us_weight_class: u16,
	us_width_class: u16,
	fs_type: u16,
	y_subscript_x_size: i16,
	y_subscript_y_size: i16,
	y_subscript_x_offset: i16,
	y_subscript_y_offset: i16,
	y_superscript_x_size: i16,
	y_superscript_y_size: i16,
	y_superscript_x_offset: i16,
	y_superscript_y_offset: i16,
	y_strikeout_size: i16,
	y_strikeout_position: i16,
	s_family_class: i16,
	panose: [u8; 10],
	ul_unicode_range_1: u32,
	ul_unicode_range_2: u32,
	ul_unicode_range_3: u32,
	ul_unicode_range_4: u32,
	ach_vend_id: u32,
	fs_selection: u16,
	us_first_character_index: u16,
	us_last_character_index: u16,
	s_typographic_ascender: i16,
	pub s_typographic_descender: i16,
	s_typographic_line_gap: i16,
	pub us_windows_ascent: u16,
	pub us_windows_descend: u16,
	ul_code_page_range_1: u32,
	ul_code_page_range_2: u32,
	sx_height: i16,
	s_cap_height: i16,
	us_default_character: u16,
	us_break_character: u16,
	us_max_context: u16,

}

pub struct HorizontalMetric {
	pub advance_width: u16,
	pub left_side_bearing: i16,
}

pub struct HorizontalMetricsTable {
	pub horizontal_metrics: Vec<HorizontalMetric>,
}

pub trait FromBytes {
	type Bytes: Default + AsMut<[u8]>;

	fn from_be_bytes(bytes: Self::Bytes) -> Self;
}

impl FromBytes for u8 {
	type Bytes = [u8; 1];

	fn from_be_bytes(bytes: Self::Bytes) -> Self {
		Self::from_be_bytes(bytes)
	}
}

impl FromBytes for u16 {
	type Bytes = [u8; 2];

	fn from_be_bytes(bytes: Self::Bytes) -> Self {
		Self::from_be_bytes(bytes)
	}
}

impl FromBytes for u32 {
	type Bytes = [u8; 4];

	fn from_be_bytes(bytes: Self::Bytes) -> Self {
		Self::from_be_bytes(bytes)
	}
}

impl FromBytes for i8 {
	type Bytes = [u8; 1];

	fn from_be_bytes(bytes: Self::Bytes) -> Self {
		Self::from_be_bytes(bytes)
	}
}

impl FromBytes for i16 {
	type Bytes = [u8; 2];

	fn from_be_bytes(bytes: Self::Bytes) -> Self {
		Self::from_be_bytes(bytes)
	}
}

impl FromBytes for i64 {
	type Bytes = [u8; 8];

	fn from_be_bytes(bytes: Self::Bytes) -> Self {
		Self::from_be_bytes(bytes)
	}
}

impl FromBytes for [u8; 10] {
	type Bytes = [u8; 10];

	fn from_be_bytes(bytes: Self::Bytes) -> Self {
		bytes
	}
}

impl FromBytes for TableTag {
	type Bytes = [u8; 4];

	fn from_be_bytes(bytes: Self::Bytes) -> Self {
		match bytes {
			[b'g', b'l', b'y', b'f'] => TableTag::Glyph,
			[b'm', b'a', b'x', b'p'] => TableTag::MaximumProfile,
			[b'l', b'o', b'c', b'a'] => TableTag::IndexToLocation,
			[b'h', b'e', b'a', b'd'] => TableTag::FontHeader,
			[b'c', b'm', b'a', b'p'] => TableTag::CharacterToGlyphIndex,
			[b'h', b'h', b'e', b'a'] => TableTag::HorizontalHeaderTable,
			[b'h', b'm', b't', b'x'] => TableTag::HorizontalMetricsTable,
			[b'O', b'S', b'/', b'2'] => TableTag::OS2AndWindowsMetricsTable,
			_ => TableTag::Other([bytes[0] as char, bytes[1] as char, bytes[2] as char, bytes[3] as char]),
		}
	}
}

impl FromBytes for TableRecord {
	type Bytes = [u8; 16];

	fn from_be_bytes(bytes: Self::Bytes) -> Self {
		TableRecord {
			table_tag: TableTag::from_be_bytes(bytes[0..4].try_into().unwrap()),
			checksum: u32::from_be_bytes(bytes[4..8].try_into().unwrap()),
			offset: u32::from_be_bytes(bytes[8..12].try_into().unwrap()),
			length: u32::from_be_bytes(bytes[12..16].try_into().unwrap()),
		}
	}
}

impl FromBytes for EncodingRecord {
	type Bytes = [u8; 8];
	
	fn from_be_bytes(bytes: Self::Bytes) -> Self {
		EncodingRecord {
			platform_id: u16::from_be_bytes(bytes[0..2].try_into().unwrap()),
			encoding_id: u16::from_be_bytes(bytes[2..4].try_into().unwrap()),
			subtable_offset: u32::from_be_bytes(bytes[4..8].try_into().unwrap()),
		}
	}
}

pub trait FromTTFReader 
where Self: Sized
{
	type Input;

	fn read(ttf_reader: &mut TrueTypeFontReader, input: Self::Input) -> Result<Self, TrueTypeFontReaderError>;
}

impl FromTTFReader for MaximumProfileTable {
	type Input = u32;

	fn read(ttf_reader: &mut TrueTypeFontReader, offset: u32) -> Result<MaximumProfileTable, TrueTypeFontReaderError> {
		ttf_reader.buffer_reader.seek(io::SeekFrom::Start(offset as u64))?;

		let major_version: u16 = ttf_reader.read_bytes()?;
		let minor_version: u16 = ttf_reader.read_bytes()?;

		if major_version != 1 || minor_version != 0 {
			todo!("Version {major_version}.{minor_version} for maxp table is not currently supported");
		}

		let num_glyphs: u16 = ttf_reader.read_bytes()?;
		let max_points: u16 = ttf_reader.read_bytes()?;
		let max_contours: u16 = ttf_reader.read_bytes()?;
		let max_composite_points: u16 = ttf_reader.read_bytes()?;
		let max_composite_contours: u16 = ttf_reader.read_bytes()?;
		let max_zones: u16 = ttf_reader.read_bytes()?;
		let max_twilight_points: u16 = ttf_reader.read_bytes()?;
		let max_storage: u16 = ttf_reader.read_bytes()?;
		let max_function_defs: u16 = ttf_reader.read_bytes()?;
		let max_instruction_defs: u16 = ttf_reader.read_bytes()?;
		let max_stack_elements: u16 = ttf_reader.read_bytes()?;
		let max_size_of_instructions: u16 = ttf_reader.read_bytes()?;
		let max_component_elements: u16 = ttf_reader.read_bytes()?;
		let max_component_depth: u16 = ttf_reader.read_bytes()?;

		Ok(MaximumProfileTable {
			major_version,
			minor_version,
			num_glyphs,
			max_points,
			max_contours,
			max_composite_points,
			max_composite_contours,
			max_zones,
			max_twilight_points,
			max_storage,
			max_function_defs,
			max_instruction_defs,
			max_stack_elements,
			max_size_of_instructions,
			max_component_elements,
			max_component_depth,
		})
	}
}

impl FromTTFReader for FontHeaderTable {
	type Input = u32;

	fn read(ttf_reader: &mut TrueTypeFontReader, offset: u32) -> Result<FontHeaderTable, TrueTypeFontReaderError> {
		ttf_reader.buffer_reader.seek(io::SeekFrom::Start(offset as u64))?;
		let major_version = ttf_reader.read_bytes()?;
		let minor_version = ttf_reader.read_bytes()?;

		if major_version != 1 || minor_version != 0 {
			todo!("Version {major_version}.{minor_version} for head table is not currently supported");
		}

		Ok(FontHeaderTable {
			major_version,
			minor_version,
			font_revision: (ttf_reader.read_bytes()?, ttf_reader.read_bytes()?),
			checksum_adjustment: ttf_reader.read_bytes()?,
			magic_number: ttf_reader.read_bytes()?,
			flags: ttf_reader.read_bytes()?,
			units_per_em: ttf_reader.read_bytes()?,
			created: ttf_reader.read_bytes()?,
			modified: ttf_reader.read_bytes()?,
			x_min: ttf_reader.read_bytes()?,
			y_min: ttf_reader.read_bytes()?,
			x_max: ttf_reader.read_bytes()?,
			y_max: ttf_reader.read_bytes()?,
			mac_style: ttf_reader.read_bytes()?,
			lowest_rec_ppem: ttf_reader.read_bytes()?,
			font_direction_hint: ttf_reader.read_bytes()?,
			index_to_location_format: ttf_reader.read_bytes()?,
			glyph_data_format: ttf_reader.read_bytes()?,
		})
	}
}

impl FromTTFReader for HorizontalHeaderTable {
	type Input = u32;

	fn read(ttf_reader: &mut TrueTypeFontReader, offset: u32) -> Result<HorizontalHeaderTable, TrueTypeFontReaderError> {
		ttf_reader.buffer_reader.seek(io::SeekFrom::Start(offset as u64))?;
		let major_version = ttf_reader.read_bytes()?;
		let minor_version = ttf_reader.read_bytes()?;

		if major_version != 1 || minor_version != 0 {
			todo!("Version {major_version}.{minor_version} for hhea table is not currently supported");
		}

		let ascender = ttf_reader.read_bytes()?;
		let descender = ttf_reader.read_bytes()?;
		let line_gap = ttf_reader.read_bytes()?;
		let advance_width_max = ttf_reader.read_bytes()?;
		let minimum_left_side_bearing = ttf_reader.read_bytes()?;
		let minimum_right_side_bearing = ttf_reader.read_bytes()?;
		let x_max_extent = ttf_reader.read_bytes()?;
		let caret_slope_rise = ttf_reader.read_bytes()?;
		let caret_slope_run = ttf_reader.read_bytes()?;
		let caret_offset = ttf_reader.read_bytes()?;
		let reserved: i16 = ttf_reader.read_bytes()?;
		let reserved: i16 = ttf_reader.read_bytes()?;
		let reserved: i16 = ttf_reader.read_bytes()?;
		let reserved: i16 = ttf_reader.read_bytes()?;
		let metric_data_format: i16 = ttf_reader.read_bytes()?;
		if metric_data_format != 0 {
			todo!("Metric Data Format {metric_data_format} from hhea is not currently supported");
		}
		let number_of_horizontal_metrics = ttf_reader.read_bytes()?;


		Ok(HorizontalHeaderTable {
			major_version,
			minor_version,
			ascender,
			descender,
			line_gap,
			advance_width_max,
			minimum_left_side_bearing,
			minimum_right_side_bearing,
			x_max_extent,
			caret_slope_rise,
			caret_slope_run,
			caret_offset,
			number_of_horizontal_metrics,
		})
	}
}

impl FromTTFReader for GlyphOffset {
	type Input = (i16, u16);

	fn read(ttf_reader: &mut TrueTypeFontReader, (index_to_location_format, count): (i16, u16)) -> Result<GlyphOffset, TrueTypeFontReaderError> {
		let glyph_offset: u32;
		if index_to_location_format == 0 {
			let half_glyph_offset: u16 = ttf_reader.read_bytes()?;
			glyph_offset = half_glyph_offset as u32 * 2;
		} else if index_to_location_format == 1 {
			glyph_offset = ttf_reader.read_bytes()?;
		} else {
			panic!("Only 0 and 1 are valid values for the index_to_location_format.")
		}

		let result = Ok(GlyphOffset {
			id: count,
			glyph_offset: Some(glyph_offset),
			glyph_length: None,
		});

		result
	}
}

impl FromTTFReader for IndexToLocationTable {
	type Input = (u32, i16, u16);

	fn read(ttf_reader: &mut TrueTypeFontReader, (offset, index_to_location_format, num_glyphs): (u32, i16, u16)) -> Result<IndexToLocationTable, TrueTypeFontReaderError> {
		ttf_reader.buffer_reader.seek(io::SeekFrom::Start(offset as u64))?;

		let mut previous: GlyphOffset = ttf_reader.read((index_to_location_format, 0))?;
		let mut glyph_offsets = Vec::new();

		for count in 1..=num_glyphs {
			let current: GlyphOffset = ttf_reader.read((index_to_location_format, count))?;
			if current.glyph_offset == previous.glyph_offset { previous.glyph_offset = None; } else {
				previous.glyph_length = Some(current.glyph_offset.unwrap() - previous.glyph_offset.unwrap());
			};
			glyph_offsets.push(previous);
			previous = current;
		}
	
		Ok(IndexToLocationTable {
			glyph_offsets,
		})
	}
}

impl FromTTFReader for GlyphRaw {
	type Input = (GlyphOffset, u64);

	fn read(ttf_reader: &mut TrueTypeFontReader, (glyph_offset, glyph_table_start): (GlyphOffset, u64)) -> Result<GlyphRaw, TrueTypeFontReaderError> {
		match glyph_offset.glyph_offset {
			Some(glyph_offset) => ttf_reader.buffer_reader.seek(io::SeekFrom::Start(glyph_table_start + glyph_offset as u64))?,
			None => {
				return Ok(GlyphRaw {
					number_of_contours: 0,
					x_min: 0,
					y_min: 0,
					x_max: 0,
					y_max: 0,
					glyph_data: GlyphDataRaw::None,
				})
			},
		};

		let number_of_contours: i16 = ttf_reader.read_bytes()?;

		let x_min: i16 = ttf_reader.read_bytes()?;
		let y_min: i16 = ttf_reader.read_bytes()?;
		let x_max: i16 = ttf_reader.read_bytes()?;
		let y_max: i16 = ttf_reader.read_bytes()?;

		if number_of_contours > 0 {
			// SIMPLE GLYPH START
			let mut end_points_of_contours: Vec<u16> = Vec::new();
			for _ in 0..number_of_contours {
				end_points_of_contours.push(ttf_reader.read_bytes()?);
			}

			let instruction_length: u16 = ttf_reader.read_bytes()?;

			let mut instructions: Vec<u8> = Vec::new();
			for _ in 0..instruction_length {
				instructions.push(ttf_reader.read_bytes()?);
			}

			let number_of_points = end_points_of_contours.last().unwrap() + 1;

			let mut glyph_flags: Vec<u8> = Vec::new();
			loop {
				let flag: u8 = ttf_reader.read_bytes()?;

				let repeated = flag & 0x08 > 0;
				if repeated {
					let number_of_repeats: u8 = ttf_reader.read_bytes()?; // Number of additional repeats.
					for _ in 0..=number_of_repeats {
						glyph_flags.push(flag);
					}
				} else {
					glyph_flags.push(flag);
				}

				if glyph_flags.len() >= number_of_points as usize {
					break;
				};
			};

			let mut glyph_x_coordinates: Vec<i16> = Vec::new();
			let mut x_coordinates_processed: u16 = 0;
			let mut current_x_coordinate: i16 = 0;
			loop {
				let flag = glyph_flags[x_coordinates_processed as usize];
				let short = flag & 0x02 > 0;
				let same_or_positive = flag & 0x10 > 0;
				if short {
					let short_x_coordinate: u8 = ttf_reader.read_bytes()?;
					if same_or_positive {
						current_x_coordinate += short_x_coordinate as i16;
					} else {
						current_x_coordinate += short_x_coordinate as i16 * -1;
					}
				} else {
					if same_or_positive {

					} else {
						let long_x_coordinate: i16 = ttf_reader.read_bytes()?;
						current_x_coordinate += long_x_coordinate;
					}
				}

				glyph_x_coordinates.push(current_x_coordinate);
				x_coordinates_processed += 1;

				if x_coordinates_processed >= number_of_points {
					break;
				};
			}

			let mut glyph_y_coordinates: Vec<i16> = Vec::new();
			let mut y_coordinates_processed: u16 = 0;
			let mut current_y_coordinate: i16 = 0;
			loop {
				let flag = glyph_flags[y_coordinates_processed as usize];
				let short = flag & 0x04 > 0;
				let same_or_positive = flag & 0x20 > 0;
				if short {
					let short_y_coordinate: u8 = ttf_reader.read_bytes()?;
					if same_or_positive {
						current_y_coordinate += short_y_coordinate as i16;
					} else {
						current_y_coordinate += short_y_coordinate as i16 * -1;
					}
				} else {
					if same_or_positive {

					} else {
						let long_y_coordinate: i16 = ttf_reader.read_bytes()?;
						current_y_coordinate += long_y_coordinate;
					}
				}

				glyph_y_coordinates.push(current_y_coordinate);
				y_coordinates_processed += 1;

				if y_coordinates_processed >= number_of_points {
					break;
				};
			}

			Ok(GlyphRaw {
				number_of_contours,
				x_min,
				x_max,
				y_min,
				y_max,
				glyph_data: GlyphDataRaw::SimpleGlyphRaw(SimpleGlyphRaw {
					end_points_of_contours,
					instruction_length,
					instructions,
					flags: glyph_flags,
					x_coordinates: glyph_x_coordinates,
					y_coordinates: glyph_y_coordinates,
				})
			})
			// SIMPLE GLYPH END
		} else if number_of_contours < 0 {
			// COMPOSITE GLYPH START
			let mut more = true;
			let mut children: Vec<ComponentGlyphRaw> = Vec::new();
			while more {
				let flag: u16 = ttf_reader.read_bytes()?;
				let glyph_index: u16 = ttf_reader.read_bytes()?;

				let xy_long = flag & 0x0001 > 0;
				let xy_signed = flag & 0x0002 > 0;
				more = flag & 0x0020 > 0;
				let x_offset_point: i32;
				let y_offset_point: i32;
				match (xy_long, xy_signed) {
					(true, true) => {
						let x_offset_point_long: i16 = ttf_reader.read_bytes()?;
						let y_offset_point_long: i16 = ttf_reader.read_bytes()?;
						x_offset_point = x_offset_point_long as i32;
						y_offset_point = y_offset_point_long as i32;
					},
					(true, false) => {
						let x_offset_point_long: u16 = ttf_reader.read_bytes()?;
						let y_offset_point_long: u16 = ttf_reader.read_bytes()?;
						x_offset_point = x_offset_point_long as i32;
						y_offset_point = y_offset_point_long as i32;
						panic!("WHAT IS THIS INSANITY");
					},
					(false, true) => {
						let x_offset_point_short: i8 = ttf_reader.read_bytes()?;
						let y_offset_point_short: i8 = ttf_reader.read_bytes()?;
						x_offset_point = x_offset_point_short as i32;
						y_offset_point = y_offset_point_short as i32;
					},
					(false, false) => {
						let x_offset_point_short: u8 = ttf_reader.read_bytes()?;
						let y_offset_point_short: u8 = ttf_reader.read_bytes()?;
						x_offset_point = x_offset_point_short as i32;
						y_offset_point = y_offset_point_short as i32;
						panic!("WHAT IS THIS INSTANITY");
					},
				}

				let has_scale = flag & 0x0008 > 0;
				let has_xy_scale = flag & 0x0040 > 0;
				let has_2x2 = flag & 0x0080 > 0;

				let mut transform_0: Option<u16> = None;
				let mut transform_1: Option<u16> = None;
				let mut transform_2: Option<u16> = None;
				let mut transform_3: Option<u16> = None;

				if has_scale {
					transform_0 = Some(ttf_reader.read_bytes()?);
				} else if has_xy_scale {
					transform_0 = Some(ttf_reader.read_bytes()?);
					transform_1 = Some(ttf_reader.read_bytes()?);
				} else if has_2x2 {
					transform_0 = Some(ttf_reader.read_bytes()?);
					transform_1 = Some(ttf_reader.read_bytes()?);
					transform_2 = Some(ttf_reader.read_bytes()?);
					transform_3 = Some(ttf_reader.read_bytes()?);
				} else {

				}

				children.push(ComponentGlyphRaw {
					flag,
					glyph_index,
					x_offset_point,
					y_offset_point,
					transform_0,
					transform_1,
					transform_2,
					transform_3,
				});
			}

			Ok(GlyphRaw {
				number_of_contours,
				x_min,
				y_min,
				x_max,
				y_max,
				glyph_data: GlyphDataRaw::CompositeGlyphRaw(CompositeGlyphRaw {
					children,
				}),
			})
			// COMPOSITE GLYPH END
		} else {
			Ok(GlyphRaw {
				number_of_contours,
				x_min,
				y_min,
				x_max,
				y_max,
				glyph_data: GlyphDataRaw::None,
			})
		}
	}
}

impl FromTTFReader for GlyphTable {
	type Input = (Vec<GlyphOffset>, u64);

	fn read(ttf_reader: &mut TrueTypeFontReader, (glyph_offsets, glyph_table_start): (Vec<GlyphOffset>, u64)) -> Result<GlyphTable, TrueTypeFontReaderError> {
		let mut glyphs: Vec<GlyphRaw> = Vec::new();
		for glyph_offset in glyph_offsets {
			let glyph: GlyphRaw = ttf_reader.read((glyph_offset.clone(), glyph_table_start))?;
			glyphs.push(glyph);
		}
		Ok(GlyphTable {
			glyphs,
		})
	}
}

impl FromTTFReader for CharacterToGlyphIndexSubtableFormat4 {
	type Input = (u32, u32);

	fn read(ttf_reader: &mut TrueTypeFontReader, (subtable_offset, cmap_table_offset): (u32, u32)) -> Result<Self, TrueTypeFontReaderError> {
		let length: u16 = ttf_reader.read_bytes()?;
		let language: u16 = ttf_reader.read_bytes()?;
		let segment_count_x2: u16 = ttf_reader.read_bytes()?;
		let segment_count = segment_count_x2 / 2;
		let search_range: u16 = ttf_reader.read_bytes()?;
		let entry_selector: u16 = ttf_reader.read_bytes()?;
		let range_shift: u16 = ttf_reader.read_bytes()?;

		let mut end_codes: Vec<u16> = Vec::new();
		for _ in 0..segment_count {
			end_codes.push(ttf_reader.read_bytes()?);
		};

		let reserved_pad: u16 = ttf_reader.read_bytes()?;
		assert_eq!(reserved_pad, 0);

		let mut start_codes: Vec<u16> = Vec::new();
		for _ in 0..segment_count {
			start_codes.push(ttf_reader.read_bytes()?);
		}

		let mut id_deltas: Vec<i16> = Vec::new();
		for _ in 0..segment_count {
			id_deltas.push(ttf_reader.read_bytes()?);
		}

		let mut id_range_offsets: Vec<u16> = Vec::new();
		for _ in 0..segment_count {
			id_range_offsets.push(ttf_reader.read_bytes()?);
		}

		let current_position = ttf_reader.buffer_reader.seek(io::SeekFrom::Current(0))?;
		let current_position_from_table = current_position - (subtable_offset + cmap_table_offset) as u64;
		let remaining_size = length as u64 - current_position_from_table;
		let glyph_ids_to_read = remaining_size / 2;

		let mut glyph_id_array: Vec<u16> = Vec::new();
		for _ in 0..glyph_ids_to_read {
			glyph_id_array.push(ttf_reader.read_bytes()?);
		}

		return Ok(CharacterToGlyphIndexSubtableFormat4 {
			length,
			language,
			segment_count,
			search_range,
			entry_selector,
			range_shift,
			end_codes,
			start_codes,
			id_deltas,
			id_range_offsets,
			glyph_id_array,
		})
	}
}

impl FromTTFReader for CharacterToGlyphIndexSubtableFormat12 {
	type Input = ();

	fn read(ttf_reader: &mut TrueTypeFontReader, _: ()) -> Result<Self, TrueTypeFontReaderError> {
		let reserved: u16 = ttf_reader.read_bytes()?;
		assert_eq!(reserved, 0);
		let length: u32 = ttf_reader.read_bytes()?;
		let language: u32 = ttf_reader.read_bytes()?;
		let num_groups: u32 =ttf_reader.read_bytes()?;

		let mut groups: Vec<(u32, u32, u32)> = Vec::with_capacity(num_groups as usize);
		for _ in 0..num_groups {
			let start_code: u32 = ttf_reader.read_bytes()?;
			let end_code: u32 = ttf_reader.read_bytes()?;
			let start_index: u32 = ttf_reader.read_bytes()?;
			groups.push((start_code, end_code, start_index));
		}

		Ok(CharacterToGlyphIndexSubtableFormat12 {
			length,
			language,
			groups,
		})
	}
}

impl FromTTFReader for CharacterToGlyphIndexSubtable {
	type Input = (u32, u32);

	fn read(ttf_reader: &mut TrueTypeFontReader, (subtable_offset, cmap_table_offset): (u32, u32)) -> Result<CharacterToGlyphIndexSubtable, TrueTypeFontReaderError> {
		ttf_reader.buffer_reader.seek(io::SeekFrom::Start((subtable_offset + cmap_table_offset) as u64))?;
		
		let format: u16 = ttf_reader.read_bytes()?;

		match format {
			4 => {
				let subtable: CharacterToGlyphIndexSubtableFormat4 = ttf_reader.read((subtable_offset, cmap_table_offset))?;
				Ok(CharacterToGlyphIndexSubtable::Format4(subtable))
			},
			12 => {
				let subtable: CharacterToGlyphIndexSubtableFormat12 = ttf_reader.read(())?;
				Ok(CharacterToGlyphIndexSubtable::Format12(subtable))
			},
			_ => {
				todo!("Format {format} cmap subtables not supported")
			}
		}
	}
}

impl FromTTFReader for CharacterToGlyphIndexTable {
	type Input = u32;

	fn read(ttf_reader: &mut TrueTypeFontReader, offset: u32) -> Result<CharacterToGlyphIndexTable, TrueTypeFontReaderError> {
		ttf_reader.buffer_reader.seek(io::SeekFrom::Start(offset as u64))?;

		let version: u16 = ttf_reader.read_bytes()?;
		if version != 0 {
			todo!("Version {version} for cmap table is not currently supported");
		}
		let number_of_subtables: u16 = ttf_reader.read_bytes()?;

		let mut encoding_records: Vec<EncodingRecord> = Vec::new();
		for _ in 0..number_of_subtables {
			encoding_records.push(ttf_reader.read_bytes()?);
		}

		let mut subtables: Vec<CharacterToGlyphIndexSubtable> = Vec::new();
		for encoding_record in encoding_records.iter() {
			subtables.push(ttf_reader.read((encoding_record.subtable_offset, offset))?)
		}
		
		Ok(CharacterToGlyphIndexTable {
			version,
			number_of_subtables,
			encoding_records,
			subtables,
		})
	}
}

impl FromTTFReader for HorizontalMetricsTable {
	type Input = (u16, u16, u64);

	fn read(ttf_reader: &mut TrueTypeFontReader, (number_of_horizontal_metrics, number_of_glyphs, offset): (u16, u16, u64)) -> Result<HorizontalMetricsTable, TrueTypeFontReaderError> {
		ttf_reader.buffer_reader.seek(io::SeekFrom::Start(offset))?;

		let mut horizontal_metrics = Vec::with_capacity(number_of_glyphs as usize);
		let mut most_recent_advance_width = 0;
		for _ in 0..number_of_horizontal_metrics {
			let advance_width = ttf_reader.read_bytes()?;
			let left_side_bearing = ttf_reader.read_bytes()?;
			most_recent_advance_width = advance_width;
			horizontal_metrics.push(HorizontalMetric { advance_width, left_side_bearing });
		}

		for _ in 0..(number_of_glyphs - number_of_horizontal_metrics) {
			let left_side_bearing = ttf_reader.read_bytes()?;
			horizontal_metrics.push(HorizontalMetric { advance_width: most_recent_advance_width, left_side_bearing });
		}

		Ok(HorizontalMetricsTable {
			horizontal_metrics,
		})
	}
}

impl FromTTFReader for OS2AndWindowsMetricsTable {
	type Input = u32;

	fn read(ttf_reader: &mut TrueTypeFontReader, offset: u32) -> Result<OS2AndWindowsMetricsTable, TrueTypeFontReaderError> {
		ttf_reader.buffer_reader.seek(io::SeekFrom::Start(offset as u64))?;

		let version: u16 = ttf_reader.read_bytes()?;

		if version != 4 {
			todo!("Version {version} for OS/2 table is not currently supported");
		}

		println!("Size of OS/2 table: {}", size_of::<OS2AndWindowsMetricsTable>());

		Ok(OS2AndWindowsMetricsTable {
			version,
			x_average_character_width: ttf_reader.read_bytes()?,
			us_weight_class: ttf_reader.read_bytes()?,
			us_width_class: ttf_reader.read_bytes()?,
			fs_type: ttf_reader.read_bytes()?,
			y_subscript_x_size: ttf_reader.read_bytes()?,
			y_subscript_y_size: ttf_reader.read_bytes()?,
			y_subscript_x_offset: ttf_reader.read_bytes()?,
			y_subscript_y_offset: ttf_reader.read_bytes()?,
			y_superscript_x_size: ttf_reader.read_bytes()?,
			y_superscript_y_size: ttf_reader.read_bytes()?,
			y_superscript_x_offset: ttf_reader.read_bytes()?,
			y_superscript_y_offset: ttf_reader.read_bytes()?,
			y_strikeout_size: ttf_reader.read_bytes()?,
			y_strikeout_position: ttf_reader.read_bytes()?,
			s_family_class: ttf_reader.read_bytes()?,
			panose: ttf_reader.read_bytes()?,
			ul_unicode_range_1: ttf_reader.read_bytes()?,
			ul_unicode_range_2: ttf_reader.read_bytes()?,
			ul_unicode_range_3: ttf_reader.read_bytes()?,
			ul_unicode_range_4: ttf_reader.read_bytes()?,
			ach_vend_id: ttf_reader.read_bytes()?,
			fs_selection: ttf_reader.read_bytes()?,
			us_first_character_index: ttf_reader.read_bytes()?,
			us_last_character_index: ttf_reader.read_bytes()?,
			s_typographic_ascender: ttf_reader.read_bytes()?,
			s_typographic_descender: ttf_reader.read_bytes()?,
			s_typographic_line_gap: ttf_reader.read_bytes()?,
			us_windows_ascent: ttf_reader.read_bytes()?,
			us_windows_descend: ttf_reader.read_bytes()?,
			ul_code_page_range_1: ttf_reader.read_bytes()?,
			ul_code_page_range_2: ttf_reader.read_bytes()?,
			sx_height: ttf_reader.read_bytes()?,
			s_cap_height: ttf_reader.read_bytes()?,
			us_default_character: ttf_reader.read_bytes()?,
			us_break_character: ttf_reader.read_bytes()?,
			us_max_context: ttf_reader.read_bytes()?,
		})
	}
}