use std::{cell::RefCell, rc::Rc};

use mircalla_types::vectors::Position;

use crate::{font::{Bounds, ComponentGlyph, Glyph, GlyphParseError, Vertex}, linked_list::LinkedListItem, ttf_parser::{Contour, Direction, Point, raw_to_intermediate::{GlyphDataIntermediate, GlyphIntermediate}}, ttf_parser_old::Intersects};

static DEBUG: bool = true;

impl From<GlyphIntermediate> for Glyph {
	fn from(intermediate: GlyphIntermediate) -> Self {
		match intermediate.glyph_data {
			GlyphDataIntermediate::SimpleGlyph { contours, points } => {
				match ( contours, points, intermediate.bounds ).to_glyph() {
					Ok( glyph ) => {
						glyph
					},
					Err(error) => {
						println!("ERROR ==> A glyph has failed to parse: {error}");
						Glyph::new_failed_parse(error, intermediate.bounds)
					}
				}
			},
			GlyphDataIntermediate::CompositeGlyph { children } => {
				let children: Vec<ComponentGlyph> = children.into_iter().map(|component| component.into()).collect();
				Glyph::new_composite(children, intermediate.bounds)
			},
			GlyphDataIntermediate::None => Glyph::new_empty(intermediate.bounds),
		}
	}
}

trait ToGlyph {
	fn to_glyph(self) -> Result<Glyph, GlyphParseError>;
}

impl ToGlyph for (Vec<Contour>, Vec<Point>, Bounds) {
	fn to_glyph(self) -> Result<Glyph, GlyphParseError> {
		let (contours, points, bounds) = self;
		let mut vertices: Vec<Vertex> = points.into_iter().map(|v| v.into()).collect();

		if DEBUG {
			println!("\n\nInitial Vertices");
			for vertex in vertices.iter() {

			}
		}

		let hole_parents = get_hole_contour_parents(&contours, &vertices);

		let mut channeled: Vec<bool> = vec![false; vertices.len()];
		for hole_contour_index in 0..contours.len() {
			let direction = contours[hole_contour_index].direction;
			if let Direction::Clockwise = direction {
				continue;
			}

			let (hole_index, parent_index) = get_closest_vertices(hole_contour_index, hole_parents[hole_contour_index].unwrap(), hole_parents, &contours, &vertices, &mut channeled);
			

		}

		// --- Move Hole Indices to Parent ---

		Ok(Glyph::new_simple(
			vertices,
			indices,
			convex_bezier_indices,
			concave_bezier_indices,
			bounds,
		))
	}
}

fn get_contour_parents(contours: &Vec<Contour>, vertices: &Vec<Vertex>) -> Vec<Option<usize>> {
	let mut parents = vec![None; contours.len()];

	for contour_index in 0..contours.len() {
		let contour = &contours[contour_index];
		
		let mut parent = None;

		for (parent_index) in 0..contours.len() {
			if parent_index == contour_index {
				continue;
			}

			let parent_contour = &contours[parent_index];

			let inside = contour.inside(parent_contour, &vertices);
			if inside {
				if let Some(old_parent_index) = parent {
					let old_parent_contour = &contours[old_parent_index];
					let new_parent_inside_old_parent = parent_contour.inside(old_parent_contour, vertices);
					if new_parent_inside_old_parent {
						parent = Some(parent_index);
					}
				} else {
					parent = Some(parent_index);
				}
			}
		}

		parents[contour_index] = parent;
	}

	parents
}

fn get_hole_contour_parents(contours: &Vec<Contour>, vertices: &Vec<Vertex>) -> Vec<Option<usize>> {
	let mut parents = vec![None; contours.len()];

	for contour_index in 0..contours.len() {
		let contour = &contours[contour_index];
		
		if let Direction::Clockwise = contour.direction {
			continue;
		}

		let mut parent = None;

		for (parent_index) in 0..contours.len() {
			if parent_index == contour_index {
				continue;
			}

			let parent_contour = &contours[parent_index];

			if let Direction::CounterClockwise = parent_contour.direction {
				continue;
			}

			let inside = contour.inside(parent_contour, &vertices);
			if inside {
				if let Some(old_parent_index) = parent {
					let old_parent_contour = &contours[old_parent_index];
					let new_parent_inside_old_parent = parent_contour.inside(old_parent_contour, vertices);
					if new_parent_inside_old_parent {
						parent = Some(parent_index);
					}
				} else {
					parent = Some(parent_index);
				}
			}
		}
		match parent {
			Some(parent) => {
				parents[contour_index] = Some(parent);
			},
			None => {
				println!("Hole does not have parent");
				todo!("Reverse Hole Direction");
			}
		}
	}

	parents
}

fn get_closest_vertices(hole_contour_index: usize, parent_contour_index: usize, parents: Vec<Option<usize>>, contours: &Vec<Contour>, vertices: &Vec<Vertex>, channeled: &mut Vec<bool>) -> (Rc<RefCell<LinkedListItem<usize>>>, Rc<RefCell<LinkedListItem<usize>>>) {
	let hole = &contours[hole_contour_index];
	let parent = &contours[parent_contour_index];
	
	let mut smallest_distance_squared = i64::MAX;
	let mut closest_hole_index = None;
	let mut closest_parent_index = None;

	for hole_index in hole.indices.iter() {
		let hole_vertex = &vertices[*hole_index.borrow().get_item()];
		for parent_index in parent.indices.iter() {
			let parent_vertex = &vertices[*parent_index.borrow().get_item()];

			let distance_squared = hole_vertex.distance_squared(parent_vertex);

			if (distance_squared < smallest_distance_squared) && hole_vertex.on_curve && parent_vertex.on_curve && !channeled[*parent_index.borrow().get_item()] {
				let next_hole_index = hole_index.borrow().next_item.clone().unwrap();

				let next_hole_vertex = &vertices[*next_hole_index.borrow().get_item()];

				let intersects = (hole_vertex, parent_vertex).intersects((hole_vertex, next_hole_vertex));

				let mut intersects_other_children = false;
				
				for other_hole_contour_index in 0..contours.len() {
					if other_hole_contour_index == hole_contour_index {
						continue;
					}
					if parents[other_hole_contour_index] != parents[hole_contour_index] {
						continue;
					}

					for other_hole_index in contours[other_hole_contour_index].indices.iter() {
						let other_hole_vertex = &vertices[*other_hole_index.borrow().get_item()];
						let next_other_hole_index = other_hole_index.borrow().next_item.clone().unwrap();
						let next_other_hole_vertex = &vertices[*next_other_hole_index.borrow().get_item()];

						intersects_other_children = intersects_other_children || (hole_vertex, parent_vertex).intersects((other_hole_vertex, next_other_hole_vertex));
					}
				}

				if !intersects && !intersects_other_children {
					smallest_distance_squared = distance_squared;
					closest_hole_index = Some(hole_index);
					closest_parent_index = Some(parent_index);
				}
			}
		}
	}

	(closest_hole_index.unwrap(), closest_parent_index.unwrap())
}

impl From<Point> for Vertex {
	fn from(Point { flag, position}: Point) -> Self {
		let on_curve = (flag & 0x01) == 1;
		Vertex { position: Position { x: position.x.into(), y: position.y.into() }, on_curve, uv_coords: [0.0, 0.0] }
	}
}