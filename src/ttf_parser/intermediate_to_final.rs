use std::{cell::RefCell, rc::Rc};

use mircalla_types::vectors::Position;

use crate::{font::{self, Bounds, ComponentGlyph, Glyph, GlyphParseError, Vertex}, linked_list::{LinkedListItem, LinkedListItemFunctions}, ttf_parser::{Contour, Direction, Intersects, Point, raw_to_intermediate::{GlyphComponentIntermediate, GlyphDataIntermediate, GlyphIntermediate}}};

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

impl From<GlyphComponentIntermediate> for font::ComponentGlyph {
	fn from(value: GlyphComponentIntermediate) -> Self {
		assert!(value.transformation_matrix.is_identity());
		font::ComponentGlyph {
			child_index: value.glyph_index as usize,
			offset: value.offset.map(|offset| offset.into()),
		}
	}
}

pub trait ToGlyph {
	fn to_glyph(self) -> Result<Glyph, GlyphParseError>;
}

impl ToGlyph for (Vec<Contour>, Vec<Point>, Bounds) {
	fn to_glyph(self) -> Result<Glyph, GlyphParseError> {
		let (mut contours, points, bounds) = self;
		let mut vertices: Vec<Vertex> = points.into_iter().map(|v| v.into()).collect();

		if DEBUG {
			println!("\n\nInitial Vertices");
			for vertex in vertices.iter() {
				println!("	{vertex:?}");
			}

			println!("\nInitial Contours");
			for contour in contours.iter() {
				println!("	Direction: {:?}", contour.direction);
				println!("	Empty: {:?}", contour.empty);
				println!("	Start: {:?}", contour.indices.start);
				println!("	End: {:?}\n", contour.indices.end);
			}
		}



		// --- Remove Off Curve Convex Points and Add Bezier Triangles
		let mut convex_bezier_indices: Vec<u32> = Vec::new();
		let mut concave_bezier_indices: Vec<u32> = Vec::new();



		// --- Move Hole Indices to Parent ---
		let hole_parents = get_hole_contour_parents(&contours, &vertices);

		let mut channeled: Vec<bool> = vec![false; vertices.len()];
		for hole_contour_index in 0..contours.len() {
			let direction = contours[hole_contour_index].direction;
			if let Direction::Clockwise = direction {
				continue;
			}

			let (hole_index, parent_index) = get_closest_vertices(hole_contour_index, hole_parents[hole_contour_index].unwrap(), &hole_parents, &contours, &vertices, &mut channeled);
			
			if DEBUG {
				println!("Closest Vertices are {}, {}", hole_index.borrow_mut().get_item(), parent_index.borrow_mut().get_item());
			}

			let hole_index_value = hole_index.borrow().get_item().clone();
			let parent_index_value = parent_index.borrow().get_item().clone();

			LinkedListItemFunctions::insert_after(&hole_index, hole_index_value);
			LinkedListItemFunctions::insert_after(&parent_index, parent_index_value);

			let hole_next = hole_index.borrow().next_item.as_ref().unwrap().clone();

			LinkedListItemFunctions::splice_together(&parent_index, &hole_next);

			contours[hole_contour_index].indices.loose_items_reference();
			contours[hole_contour_index].empty = true;
		}



		// --- Calculate Triangles ---
		let mut indices: Vec<u32> = Vec::new();
		for contour in contours.iter_mut() {
			if contour.empty {
				continue;
			}

			let length = contour.indices.iter().count();
			let mut current_index = contour.indices.start.clone().unwrap();
			let mut indices_removed = 0;
			let mut indices_passed = 0;
			while indices_removed < (length - 2) {
				if indices_passed == length - indices_removed {
					println!("Stuck in Triangulisation");
					return Err(GlyphParseError::StuckInTriangulisationLoop);
				}

				let previous_index = current_index.borrow().previous_item.clone().unwrap();
				let next_index = current_index.borrow().next_item.clone().unwrap();

				let centre_vertex = &vertices[current_index.borrow().get_item().clone()];
				let previous_vertex = &vertices[previous_index.borrow().get_item().clone()];
				let next_vertex = &vertices[next_index.borrow().get_item().clone()];

				let x_1 = (previous_vertex.position.x - centre_vertex.position.x).value as i64; // So that multiplication doesn't overflow
				let y_1 = (previous_vertex.position.y - centre_vertex.position.y).value as i64;

				let x_2 = (next_vertex.position.x - centre_vertex.position.x).value as i64;
				let y_2 = (next_vertex.position.y - centre_vertex.position.y).value as i64;

				let direction: Direction = ( (x_1 * y_2 ) >= (y_1 * x_2) ).into();

				let mut ear = false;

				if direction == Direction::Clockwise {
					let all_outside = contour.indices.iter().map(|index| {
						let other_vertex = &vertices[*index.borrow().get_item()];
						if (other_vertex.position == previous_vertex.position) || (other_vertex.position == centre_vertex.position) || (other_vertex.position == next_vertex.position) {
							true
						} else {
							!other_vertex.is_inside_triangle(previous_vertex, centre_vertex, next_vertex)
						}
					}).fold(true, |acc, v| acc && v);

					if all_outside {
						indices.push(next_index.borrow().get_item().clone() as u32);
						indices.push(current_index.borrow().get_item().clone() as u32);
						indices.push(previous_index.borrow().get_item().clone() as u32);
						indices_removed += 1;
						indices_passed = 0;
						ear = true;
						let old_current = current_index;
						current_index = old_current.borrow().next_item.clone().unwrap();

						let _ = LinkedListItemFunctions::remove(old_current, &mut contour.indices);
					}
				}

				if !ear {
					indices_passed += 1;
					let old_current = current_index;
					current_index = old_current.borrow().next_item.clone().unwrap();
				}



				
			}
		}

		if DEBUG {
			for contour in contours.into_iter() {
				println!("Dropping Contour");
				drop(contour)
			}
		}

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

fn get_closest_vertices(hole_contour_index: usize, parent_contour_index: usize, parents: &Vec<Option<usize>>, contours: &Vec<Contour>, vertices: &Vec<Vertex>, channeled: &mut Vec<bool>) -> (Rc<RefCell<LinkedListItem<usize>>>, Rc<RefCell<LinkedListItem<usize>>>) {
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
					if contours[other_hole_contour_index].empty {
						continue;
					}

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
					closest_hole_index = Some(hole_index.clone());
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