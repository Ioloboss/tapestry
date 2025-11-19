pub mod ttf_reader;
pub mod ttf_parser;
pub mod font;
pub mod read {
	use crate::font::{self, Font, ToTriangles};
	use crate::ttf_reader::{self, CharacterToGlyphIndexTable, FontHeaderTable, GlyphTable, IndexToLocationTable, MaximumProfileTable, TableRecord, TableTag};
	use crate::ttf_parser::{Direction, GlyphDataIntermediate, GlyphIntermediate};
	use std::{fs::File, path::Path};

	pub fn read_one_glyph(filename: &Path, glyph_index: usize) {
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

		let mut glyphs: Vec<GlyphIntermediate> = glyph_table.glyphs.into_iter().map(|v| v.into()).collect();

		let character_to_glyph_index_table: CharacterToGlyphIndexTable = match character_to_glyph_index_table_record {
			Some(charachter_to_glyph_index_table_record) => ttf_reader.read(charachter_to_glyph_index_table_record.offset).unwrap(),
			None => panic!("Font should have a cmap table."),
		};

		let glyph = match glyphs.remove(glyph_index).glyph_data {
			GlyphDataIntermediate::CompositeGlyph(_) => todo!(),
			GlyphDataIntermediate::None => todo!(),
			GlyphDataIntermediate::SimpleGlyph(simple_glyph) => {
				simple_glyph.to_triangles(true)
			},
		};

		match glyph {
			Ok(_) => println!("Success"),
			Err(_) => println!("Error"),
		}
	}

	impl Font {
		pub fn new(filename: &Path) -> Self {
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

			let glyphs: Vec<GlyphIntermediate> = glyph_table.glyphs.into_iter().map(|v| v.into()).collect();

			let character_to_glyph_index_table: CharacterToGlyphIndexTable = match character_to_glyph_index_table_record {
				Some(charachter_to_glyph_index_table_record) => ttf_reader.read(charachter_to_glyph_index_table_record.offset).unwrap(),
				None => panic!("Font should have a cmap table."),
			};

			let mappings: Vec<font::Mapping> = character_to_glyph_index_table.subtables.into_iter().map(|v| v.into()).collect();
			let glyphs: Vec<font::Glyph> = glyphs.into_iter().map(|v| v.into()).collect();

			Font {
				glyphs,
				mappings,
			}

		}
	}
}

#[cfg(test)]
mod tests {
	use std::fs::File;

	use crate::{font::Vertex, ttf_parser::{Contour, EquivalentLineSegments, GetDirection, IntersectionPoint, Intersects, ToRightOf}, ttf_reader::{CharacterToGlyphIndexTable, FontHeaderTable, GlyphTable, IndexToLocationTable, MaximumProfileTable, TableRecord, TableTag}};

	use super::*;

	#[test]
	fn point_to_right_of_line() {
		let vertex_1: Vertex = (10, 0).into();
		let vertex_2: Vertex = (15, 100).into();

		let vertex_3: Vertex = (20, 20).into();

		assert!((&vertex_1, &vertex_2).to_right_of(&vertex_3, true));
	}

	#[test]
	fn point_to_left_of_line() {
		let vertex_1: Vertex = (10, 0).into();
		let vertex_2: Vertex = (15, 100).into();

		let vertex_3: Vertex = (5, 20).into();

		assert!(!(&vertex_1, &vertex_2).to_right_of(&vertex_3, true));
	}

	#[test]
	fn lines_intersect() {
		let line_1: (&Vertex, &Vertex) = (&(186, 350).into(), &(186, 0).into());
		let line_2: (&Vertex, &Vertex) = (&(522, 306).into(), &(142, 306).into());

		assert!(line_1.intersects(line_2));
	}

	#[test]
	fn line_segments_equivalent() {
		assert!( ( &(186, 670).into(), &(186, 344).into() ).equivalent(&(&(186, 670).into(), &(186, 344).into())))
	}

	#[test]
	fn pairs_of_line_segments_equivalent() {
		assert!(((&(186, 670).into(), &(186, 344).into()), (&(540, 628).into(), &(142, 628).into())).equivalent(&((&(540, 628).into(), &(142, 628).into()), (&(186, 670).into(), &(186, 344).into()))))
	}

	#[test]
	fn lines_intersection_point() {
		let line_1: (&Vertex, &Vertex) = (&(540,628).into(), &(142,628).into());
		let line_2: (&Vertex, &Vertex) = (&(186,670).into(), &(186,344).into());
		
		let intersection_point = line_1.intersection_point(line_2);
		println!("{intersection_point}");
		assert_eq!(intersection_point, (186, 628).into());
	}

	#[test]
	fn lines_intersect_both_vertical() {
		let line_1: (&Vertex, &Vertex) = (&(329, 50).into(), &(329, 100).into());
		let line_2: (&Vertex, &Vertex) = (&(329, 75).into(), &(329, 125).into());

		assert!(line_1.intersects(line_2));

		assert_eq!(line_1.intersection_point(line_2), (329, 62).into());
	}

	#[test]
	fn contour_is_clockwise() {
		let vertices: Vec<Vertex> = vec![(329,334).into(), (329,270).into(), (328,268).into(), (325,355).into(), (329,360).into()];
		let contour: Vec<Option<usize>> = vec![0, 1, 2, 3, 4, 0].into_iter().map(|v| Some(v)).collect();

		println!("{:?}", contour.get_direction(&vertices));
		panic!();
	}
}