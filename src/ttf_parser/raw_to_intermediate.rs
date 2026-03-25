use mircalla_types::vectors::Position;

use crate::{font::Bounds, linked_list::LinkedList, ttf_parser::{Contour, Direction, GetDirection, Point}, ttf_reader::{ComponentGlyphRaw, GlyphDataRaw}};


pub struct GlyphIntermediate {
	pub number_of_contours: Option<u16>,
	pub bounds: Bounds,
	pub glyph_data: GlyphDataIntermediate,
}

pub enum GlyphDataIntermediate {
	CompositeGlyph{ children: Vec<GlyphComponentIntermediate> },
	SimpleGlyph{ contours: Vec<Contour>, points: Vec<Point> },
	None,
}

impl From<GlyphDataRaw> for GlyphDataIntermediate {
	fn from(value: GlyphDataRaw) -> Self {
		match value {
			GlyphDataRaw::CompositeGlyphRaw(value) => {
				let children = value.children.into_iter().map(|child| child.into()).collect();
				GlyphDataIntermediate::CompositeGlyph { children }
			},
			GlyphDataRaw::SimpleGlyphRaw(value) => {
				let points: Vec<Point> = value.flags.into_iter().zip(value.x_coordinates.into_iter().zip(value.y_coordinates.into_iter())).map(
					| (flag, position) | {
						Point {
							flag,
							position: position.into(),
						}
					}
				).collect();

				let mut contours = Vec::new();
				let mut start = 0;

				for end_point in value.end_points_of_contours.into_iter().map(|value| value as usize) {
					let start_point = start;
					let mut indices: LinkedList<usize> = (start_point ..= end_point).into();

					let direction = indices.get_direction(&points);

					contours.push(Contour {
						indices,
						direction,
					});

					start = end_point + 1;
				}

				GlyphDataIntermediate::SimpleGlyph { contours, points, }
			},
			GlyphDataRaw::None => {
				GlyphDataIntermediate::None
			},
		}
	}
}

pub struct GlyphComponentIntermediate {
	flag: u16,
	glyph_index: u16,
	offset: Position<i32>,
	transformation_matrix: TransformationMatrix,
}

impl From<ComponentGlyphRaw> for GlyphComponentIntermediate {
	fn from(value: ComponentGlyphRaw) -> Self {
		let transformation_matrix = [
			value.transform_0.map(|inner| inner.into()),
			value.transform_1.map(|inner| inner.into()),
			value.transform_2.map(|inner| inner.into()),
			value.transform_3.map(|inner| inner.into()),
		].into();
		GlyphComponentIntermediate {
			flag: value.flag,
			glyph_index: value.glyph_index,
			offset: (value.x_offset_point, value.y_offset_point).into(),
			transformation_matrix,
		}
	}
}

pub struct TransformationMatrix {
	p11: f32,
	p12: f32,
	p21: f32,
	p22: f32,
}

impl TransformationMatrix {
	fn identity_scaled(x_scale: f32, y_scale: f32) -> Self {
		TransformationMatrix {
			p11: x_scale,
			p12: 0.0,
			p21: 0.0,
			p22: y_scale,
		}
	}

	pub fn is_identity(&self) -> bool {
		self.p11 - 1.0 < f32::EPSILON
		&& self.p12 - 0.0 < f32::EPSILON
		&& self.p21 - 0.0 < f32::EPSILON
		&& self.p22 - 1.0 < f32::EPSILON
	}
}

impl From<[Option<Fixed2Dot14>; 4]> for TransformationMatrix {
	fn from(value: [Option<Fixed2Dot14>; 4]) -> Self {
		match value {
			[None, None, None, None] => TransformationMatrix::identity_scaled(1.0, 1.0),
			[Some(scale), None, None, None] => TransformationMatrix::identity_scaled(scale.0, scale.0),
			[Some(x_scale), Some(y_scale), None, None] => TransformationMatrix::identity_scaled(x_scale.0, y_scale.0),
			[Some(p11), Some(p12), Some(p21), Some(p22)] => TransformationMatrix { p11: p11.0, p12: p12.0, p21: p21.0, p22: p22.0, },
			_ => panic!("Transformation number should have one of the above patterns.")
		}
	}
}

struct Fixed2Dot14(f32);

impl From<u16> for Fixed2Dot14 {
	fn from(value: u16) -> Self {
		let integer_part = (value >> 14) as f32;
		let float_part = (value & 0b0011_1111_1111_1111) as f32 / 2u32.pow(14) as f32;

		Fixed2Dot14(integer_part + float_part)
	}
}

impl GetDirection<Point> for LinkedList<usize> {
	fn get_direction(&mut self, vertices: &Vec<Point>) -> Direction {
		let mut lowest_y = i16::MAX;
		let mut highest_x = i16::MIN;
		let mut chosen_index = None;
		for index in self.iter() {
			let point = &vertices[*index.borrow().get_item()];
			if (point.position.y < lowest_y) || ((point.position.y == lowest_y) && (point.position.x > highest_x)) {
				lowest_y = point.position.y;
				highest_x = point.position.x;
				chosen_index = Some(index.clone());
			}
		}

		let centre_point = &vertices[*chosen_index.as_ref().unwrap().borrow().get_item()];

		let previous_index = chosen_index.as_ref().unwrap().borrow().previous_item.clone().unwrap();

		let next_index = chosen_index.as_ref().unwrap().borrow().next_item.clone().unwrap();

		let previous_point = &vertices[*previous_index.borrow().get_item()];
		let next_point = &vertices[*next_index.borrow().get_item()];

		let x_1 = (previous_point.position.x - centre_point.position.x) as i64;
		let y_1 = (previous_point.position.y - centre_point.position.y) as i64;
		let x_2 = (next_point.position.x - centre_point.position.x) as i64;
		let y_2 = (next_point.position.y - centre_point.position.y) as i64;

		( (x_1 * y_2) > (y_1 * x_2) ).into()
	}
}