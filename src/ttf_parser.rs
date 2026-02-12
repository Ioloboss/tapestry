use crate::{font::{self, Bounds, FontUnits, GlyphParseError, Position, ToTriangles, Vertex}, ttf_reader::{ComponentGlyphRaw, CompositeGlyphRaw, GlyphDataRaw, GlyphRaw, SimpleGlyphRaw}};

impl From<GlyphIntermediate> for font::Glyph {
	fn from(value: GlyphIntermediate) -> Self {
		match value.glyph_data {
			GlyphDataIntermediate::SimpleGlyph(glyph_data) => {
				match glyph_data.to_triangles(false) {
					Ok((vertices, indices, convex_bezier_indices, concave_bezier_indices)) => {
						font::Glyph::new_simple(vertices, indices, convex_bezier_indices, concave_bezier_indices, value.bounds)
					},
					Err(error) => {
						println!("ERROR ==> A GLYPH HAS FAILED TO PARSE");
						font::Glyph::new_failed_parse(error, value.bounds)
					}
				}
				
			},
			GlyphDataIntermediate::CompositeGlyph(glyph_data) => {
				let children: Vec<font::ComponentGlyph> = glyph_data.children.into_iter().map(|v| v.into()).collect();
				font::Glyph::new_composite(children, value.bounds)
			},
			GlyphDataIntermediate::None => { font::Glyph::new_empty(value.bounds)},
		}
	}
}

impl From<GlyphComponentIntermediate> for font::ComponentGlyph {
	fn from(value: GlyphComponentIntermediate) -> Self {
		assert!(value.transformation_matrix.is_identity());
		font::ComponentGlyph {
			child_index: value.glyph_index as usize,
			offset: value.offset.into(),
		}
	}
}

impl From<Offset> for Position<FontUnits<i32>> {
	fn from(value: Offset) -> Self {
		Self { x: value.x.into(), y: value.y.into() }
	}
}

impl Contour {
	fn vertex_inside_contour(&self, vertices: &Vec<Vertex>, vertex: &Vertex) -> bool {
		let mut number_of_intersections = 0;
		for (indices_position, first_vertex_index) in self.indices.iter().enumerate() {
			if let None = first_vertex_index {
				continue;
			}
			let first_vertex_index = first_vertex_index.unwrap();
			let second_vertex_index = self.indices.next(indices_position).unwrap();
			let first_vertex = &vertices[first_vertex_index];
			let second_vertex = &vertices[second_vertex_index];

			let x_1: i64;
			let x_2: i64;
			let y_1: i64;
			let y_2: i64;
			if (first_vertex.y < second_vertex.y) {
				x_1 = (first_vertex.x - second_vertex.x).value as i64; // SO THAT MULTIPLACTION DOESN'T OVERFLOW
				y_1 = (first_vertex.y - second_vertex.y).value as i64;
				x_2 = (vertex.x - second_vertex.x).value as i64;
				y_2 = (vertex.y - second_vertex.y).value as i64;
			} else {
				x_1 = (second_vertex.x - first_vertex.x).value as i64; // SO THAT MULTIPLACTION DOESN'T OVERFLOW
				y_1 = (second_vertex.y - first_vertex.y).value as i64;
				x_2 = (vertex.x - first_vertex.x).value as i64;
				y_2 = (vertex.y - first_vertex.y).value as i64;
			}

			let vertex_east = ( (x_1 * y_2) < (y_1 * x_2) );
			let vertex_in_boundary = if (first_vertex.y < second_vertex.y) {
				(first_vertex.y < vertex.y) && (vertex.y <= second_vertex.y)
			} else {
				(second_vertex.y < vertex.y) && (vertex.y <= first_vertex.y)
			};
			if vertex_east && vertex_in_boundary {
				number_of_intersections += 1
			};
		};
		number_of_intersections % 2 == 1
	}
}

pub trait Intersects {
	fn intersects(self, other_line: Self) -> bool;
}

pub trait IntersectionPoint<T> {
	fn intersection_point(self, other_line: Self) -> T;
}

impl Intersects for (&Vertex, &Vertex) {
	fn intersects(self, other_line: Self) -> bool {
		let vertex_1 = self.0;
		let vertex_2 = self.1;
		let vertex_3 = other_line.0;
		let vertex_4 = other_line.1;

		let orientation_1 = self.to_right_of(vertex_3, true);
		let orientation_2 = self.to_right_of(vertex_4, true);

		let orientation_3 = other_line.to_right_of(vertex_1, true);
		let orientation_4 = other_line.to_right_of(vertex_2, true);

		if (vertex_1 == vertex_3) || (vertex_1 == vertex_4) || (vertex_2 == vertex_3) || (vertex_2 == vertex_4) {
			return false;
		}

		(orientation_1 != orientation_2) && (orientation_3 != orientation_4)
	}
}

impl IntersectionPoint<Vertex> for (&Vertex, &Vertex) {
	fn intersection_point(self, other_line: Self) -> Vertex {
		let vertex_1 = self.0;
		let vertex_2 = self.1;
		let vertex_3 = other_line.0;
		let vertex_4 = other_line.1;

		let m_1 = (vertex_1.y.value as f64 - vertex_2.y.value as f64) / (vertex_1.x.value as f64 - vertex_2.x.value as f64);
		let m_2 = (vertex_3.y.value as f64 - vertex_4.y.value as f64) / (vertex_3.x.value as f64 - vertex_4.x.value as f64);

		let (x, y) = if vertex_1.x == vertex_2.x {
			let x = vertex_1.x.value as f64;
			let y = m_2 * (x - vertex_3.x.value as f64) + vertex_3.y.value as f64;
			(x, y)
		} else if vertex_3.x == vertex_4.x {
			let x = vertex_3.x.value as f64;
			let y = m_1 * (x - vertex_1.x.value as f64) + vertex_1.y.value as f64;
			(x, y)

		} else {
			let x = ( (m_1 * vertex_1.x.value as f64) - (m_2 * vertex_3.x.value as f64) + vertex_3.y.value as f64 - vertex_1.y.value as f64 ) / (m_1 - m_2);
			let y = m_1 * (x - vertex_1.x.value as f64) + vertex_1.y.value as f64;
			(x, y)

		};

		let x = x.round() as i16;
		let y = y.round() as i16;

		Vertex { x: x.into(), y: y.into(), on_curve: true, uv_coords: [0.0, 0.0], }
	}
}

pub trait ToRightOf<T> {
	fn to_right_of(self, vertex: &T, or_equal_to: bool) -> bool;
}

impl ToRightOf<Vertex> for (&Vertex, &Vertex) {
	fn to_right_of(self, vertex: &Vertex, or_equal_to: bool) -> bool {
		let vertex_1 = self.0;
		let vertex_2 = self.1;

		let x_1 = (vertex_2.x - vertex_1.x).value as i64;
		let y_1 = (vertex_2.y - vertex_1.y).value as i64;

		let x_2 = (vertex_2.x - vertex.x).value as i64;
		let y_2 = (vertex_2.y - vertex.y).value as i64;

		if or_equal_to {
			( x_1 * y_2 ) >= ( y_1 * x_2 )
		} else {
			( x_1 * y_2 ) > ( y_1 * x_2 )
		}
	}
}

trait Inside<T> {
	fn inside(self, vertex: &T) -> bool;
}

impl Inside<Vertex> for (&Vertex, &Vertex, &Vertex) {
	fn inside(self, vertex: &Vertex) -> bool {
		let vertex_1 = self.0;
		let vertex_2 = self.1;
		let vertex_3 = self.2;
		let x_1 = (vertex_1.x - vertex_2.x).value as i64;
		let y_1 = (vertex_1.y - vertex_2.y).value as i64;
		let x_2 = (vertex.x - vertex_2.x).value as i64;
		let y_2 = (vertex.y - vertex_2.y).value as i64;
		let orientation_1 = ( (x_1 * y_2) >= (y_1 * x_2) );

		let x_1 = (vertex_2.x - vertex_3.x).value as i64;
		let y_1 = (vertex_2.y - vertex_3.y).value as i64;
		let x_2 = (vertex.x - vertex_3.x).value as i64;
		let y_2 = (vertex.y - vertex_3.y).value as i64;
		let orientation_2 = ( (x_1 * y_2) >= (y_1 * x_2) );

		let x_1 = (vertex_3.x - vertex_1.x).value as i64;
		let y_1 = (vertex_3.y - vertex_1.y).value as i64;
		let x_2 = (vertex.x - vertex_1.x).value as i64;
		let y_2 = (vertex.y - vertex_1.y).value as i64;
		let orientation_3 = ( (x_1 * y_2) >= (y_1 * x_2) );

		(orientation_1 == orientation_2) && (orientation_2 == orientation_3)
	}
}

pub trait EquivalentLineSegments {
	fn equivalent(&self, other_line_segment: &Self) -> bool;
}

impl EquivalentLineSegments for (&Vertex, &Vertex) {
	fn equivalent(&self, other_line_segment: &Self) -> bool {
		( (self.0 == other_line_segment.0) && (self.1 == other_line_segment.1) ) ||
		( (self.0 == other_line_segment.1) && (self.1 == other_line_segment.1) )
	}
}

impl EquivalentLineSegments for ( (&Vertex, &Vertex), (&Vertex, &Vertex) ) {
	fn equivalent(&self, other_line_segment: &Self) -> bool {
		let condition_1 = (self.0.equivalent(&other_line_segment.0));
		let condition_2 = (self.1.equivalent(&other_line_segment.1));
		let condition_3 = (self.0.equivalent(&other_line_segment.1));
		let condition_4 = (self.1.equivalent(&other_line_segment.0));
		( condition_1 && condition_2 ) || ( condition_3 && condition_4 )
	}
}

trait PrintList {
	fn print(&self);
}

impl PrintList for Vec<Vertex> {
	fn print(&self) {
		println!("Start Of Vertices");
		for (index, vertex) in self.iter().enumerate() {
			//println!("	Vertex At Index {index} is {vertex}");
			println!("x_{{{index}}} = {vertex}");
		}
		println!("End Of Vertices");
	}
}

impl PrintList for Contour {
	fn print(&self) {
		println!("Contour: ");
		println!("	Direction = {:?}", self.direction);
		println!("	Indices Removed = {}", self.indices_removed);
		print!("[");
		let mut nones = 0;
		for (position, index) in self.indices.iter().enumerate() {
			match index {
				Some(index) => {
					print!("x_{{{index}}}, ");
				},
				None => {
					nones += 1;
				},
			}
		}
		match self.indices[0] {
			Some(index) => {
				print!("x_{{{index}}}]");
			},
			None => {
				if let None = self.indices.next(0) {
					print!("]");
				} else {
					let index = self.indices.next(0).unwrap();
					print!("x_{{{index}}}]\n");
				}
			},
		}
		println!("	Number Of Nones in Contour = {}", nones);
	}
}

impl Contour {
	fn inside(&self, other_contour: &Self, vertices: &Vec<Vertex>) -> bool {
		let mut inside = true;
		for (inner_position, inner_index) in self.indices.iter().enumerate() {
			if let None = inner_index {
				continue;
			}
			let inner_vertex = &vertices[inner_index.unwrap()];
			let next_inner_vertex = &vertices[self.indices.next(inner_position).unwrap()];
			let inner_vertex_inside_other_contour = other_contour.vertex_inside_contour(vertices, inner_vertex);
			let mut intersects = false;
			for (outer_position, outer_index) in other_contour.indices.iter().enumerate() {
				if let None = outer_index {
					continue;
				}
				let outer_vertex = &vertices[outer_index.unwrap()];
				let next_outer_vertex = &vertices[other_contour.indices.next(outer_position).unwrap()];
				intersects = intersects || (inner_vertex, next_inner_vertex).intersects((outer_vertex, next_outer_vertex));
			}
			inside = inside && (inner_vertex_inside_other_contour && !intersects);
		}
		inside
	}
}

impl ToTriangles for GlyhpSimpleIntermediate {
	fn to_triangles(self, debug_mode: bool) -> Result<(Vec<Vertex>, Vec<u32>, Vec<u32>, Vec<u32>), GlyphParseError> {
		let mut vertices: Vec<Vertex> = self.points.iter().map(|v| v.into()).collect();
		//println!("virtices initially created ({})", vertices.len());

		let mut pre_processed_contours: Vec<Contour> = self.contours;

		if (debug_mode) {
			println!("\n\nInitial Vertices");
			vertices.print();
			println!("\nInitial Contours");
			for contour in pre_processed_contours.iter() {
				println!("\n");
				contour.print();
			}
		}


/* 
		// --- REVERSE HOLE IF IT HAS NO PARENT
		for contour_index in 0..pre_processed_contours.len() {
			let contour = &pre_processed_contours[contour_index];
			if let Direction::CounterClockwise = contour.direction {
				let mut has_parent = false;

				for (parent_index, parent_contour) in pre_processed_contours.iter().enumerate() {
					if let Direction::Clockwise = parent_contour.direction {
						let inside = contour.inside(parent_contour, &vertices);
						if inside {
							has_parent = true;
						}
					}
				};
				if !has_parent {
					println!("Hole Parent Does Not Have Hole. Assuming Font is not Spec-Compliant. Reversing contour.indices and swapping contour.direction");
					let contour_mut = &mut pre_processed_contours[contour_index];
					contour_mut.indices.reverse();
					contour_mut.direction = contour_mut.indices.get_direction(&vertices);
					if let Direction::CounterClockwise = contour_mut.direction {
						println!("Direction is still counter clockwise");
						return Err(GlyphParseError::HoleDoesNotHaveParent);
					}
				};


			}
		};
*/

		// --- Find Parent of Contours to Fix Non-Spec Compliant Glyphs

		let mut parents: Vec<Option<usize>> = (0..pre_processed_contours.len()).map(|_| None).collect();
		for contour_index in 0..pre_processed_contours.len() {
			let contour = &pre_processed_contours[contour_index];
			let mut parent: Option<usize> = None;

			for (parent_index, parent_contour) in pre_processed_contours.iter().enumerate() {
				if contour_index == parent_index {
					continue;
				}
				let inside = contour.inside(parent_contour, &vertices);
				if inside {
					if let Some(old_parrent_index) = parent {
						let old_parent_contour = &pre_processed_contours[old_parrent_index];
						let new_parent_inside_old_parent = parent_contour.inside(old_parent_contour, &vertices);
						if new_parent_inside_old_parent {
							parent = Some(parent_index);
						}
					} else {
						parent = Some(parent_index);
					}
				}
			};
			parents[contour_index] = parent;
		};

		if debug_mode {
			println!("\n\nParents");
			println!("{parents:?}");
		}

		// Calculate Parent Depths

		let mut directions: Vec<Option<Direction>> = (0..pre_processed_contours.len()).map(|_| None).collect();
		let mut directions_calculated = 0;
		let mut contour_index = 0;
		loop {
			if let None = directions[contour_index] {
				match parents[contour_index] {
					Some(parent_index) => {
						match directions[parent_index] {
							Some(parent_direction) => {
								// FIND INTERSECTIONS
								let contour = &pre_processed_contours[contour_index];
								let mut intersects = false;
								for (inner_position, inner_index) in contour.indices.iter().enumerate() {
									if let None = inner_index {
										continue;
									}
									let inner_vertex = &vertices[inner_index.unwrap()];
									let next_inner_index = contour.indices.next(inner_position);
									let next_inner_vertex = &vertices[next_inner_index.unwrap()];
									for (other_contour_position, other_contour) in pre_processed_contours.iter().enumerate() {
										if other_contour_position == contour_index {
											continue;
										}
										for (outer_position, outer_index) in other_contour.indices.iter().enumerate() {
											let outer_vertex = &vertices[outer_index.unwrap()];
											let next_outer_index = other_contour.indices.next(outer_position);
											let next_outer_vertex = &vertices[next_outer_index.unwrap()];
											intersects = intersects || (inner_vertex, next_inner_vertex).intersects((outer_vertex, next_outer_vertex));
										}
									}
								}
								if intersects {
									directions[contour_index] = Some(contour.direction);
									directions_calculated += 1;
								} else {
									directions[contour_index] = Some(parent_direction.opposite());
									directions_calculated += 1;
								}
							},
							None => {
							}
						}
					},
					None => {
						directions[contour_index] = Some(Direction::Clockwise);
						directions_calculated += 1;
					}
				}
			}
			if directions_calculated == pre_processed_contours.len() {
				break;
			}
			contour_index = (contour_index + 1).rem_euclid(pre_processed_contours.len());
		}

		if debug_mode {
			println!("\n\nParent Depths");
			println!("{directions:?}");
			println!("\n");
		}

		// Fix Contour Directions
		for (contour_index, contour) in pre_processed_contours.iter_mut().enumerate() {
			let direction_from_parents: Direction = directions[contour_index].unwrap();
			if direction_from_parents != contour.direction {
				//println!("Contour Direction Does NOT Match Contour Direction Derived from parent");
				contour.indices.reverse();
				contour.direction = contour.indices.get_direction(&vertices);
			}
		}
 
		// --- Adding Vertices Between Subsequent Off-Curve Vertices

		for contour in pre_processed_contours.iter_mut() {
			let mut contour_indices_position: usize = 0;
			while contour_indices_position < contour.indices.len() {
				let index = contour.indices[contour_indices_position];
				match index {
					Some(index) => {
						let previous_index = contour.indices.previous(contour_indices_position).unwrap();
						let vertex = &vertices[index];
						let previous_vertex = &vertices[previous_index];
						if ( !vertex.on_curve && !previous_vertex.on_curve ) {
							let extra_vertex = Vertex::new((vertex.x + previous_vertex.x).value / 2, (vertex.y + previous_vertex.y).value / 2);
							let extra_vertex_index = vertices.len();
							vertices.push(extra_vertex);
							if debug_mode {
								println!("Vertice {extra_vertex_index} Added Between Subsequent off curve points (1)");
							}
							let end_part = contour.indices.split_off(contour_indices_position);
							contour.indices.push(Some(extra_vertex_index));
							contour.indices.extend(end_part);
						}
					},
					None => {},
				}
				contour_indices_position += 1;
			}
		}

		if debug_mode {
			println!("\n\nBefore Bezier Triangles Changed");
			for contour in pre_processed_contours.iter() {
				println!("\n");
				contour.print();
			}
			println!("\n\n");
		}

		// --- Remove Off Curve Convex Points and Add Bezier Triangles

		let mut convex_bezier_indices: Vec<u32> = Vec::new();
		let mut concave_bezier_indices: Vec<u32> = Vec::new();
		for contour in pre_processed_contours.iter_mut() {
			//if let Direction::CounterClockwise = contour.direction {
			//	continue;
			//}
			for contour_indices_position in 0..contour.indices.len() {
				let index = contour.indices[contour_indices_position];
				match index {
					Some(index) => {
						let vertex = (&vertices[index]).with_changed_uv_coord([0.5, 0.0]);
						if vertex.on_curve {
							continue;
						}

						let previous_index = contour.indices.previous(contour_indices_position).unwrap();
						let next_index = contour.indices.next(contour_indices_position).unwrap();
						let previous_vertex = (&vertices[previous_index]).with_changed_uv_coord([1.0, 1.0]);
						let next_vertex = (&vertices[next_index]).with_changed_uv_coord([0.0, 0.0]);
						if (&previous_vertex, &vertex).to_right_of(&next_vertex, true) {

							let mut intersects = false;
							for inner_contour_indices_position in 0..contour.indices.len() {
								match contour.indices[inner_contour_indices_position] {
									Some(inner_index) => {
										let inner_vertex = &vertices[inner_index];
										let inner_index_next = contour.indices.next(inner_contour_indices_position).unwrap();
										let inner_vertex_next = &vertices[inner_index_next];
										intersects = intersects || (inner_vertex, inner_vertex_next).intersects((&previous_vertex, &next_vertex));
									},
									None => {},
								};
							}

							let new_index = vertices.len();
							if intersects {
								let new_vertex_x = vertex.x.value as f64 + 0.25*(previous_vertex.x.value as f64 - vertex.x.value as f64) + 0.25*(next_vertex.x.value as f64 - vertex.x.value as f64);
								let new_vertex_y = vertex.y.value as f64 + 0.25*(previous_vertex.y.value as f64 - vertex.y.value as f64) + 0.25*(next_vertex.y.value as f64 - vertex.y.value as f64);
								let new_vertex_x = new_vertex_x.round() as i16;
								let new_vertex_y = new_vertex_y.round() as i16;

								let new_vertex = Vertex { x: new_vertex_x.into(), y: new_vertex_y.into(), on_curve: true, uv_coords: [0.0, 0.0]};
								vertices.push(new_vertex);
								if debug_mode {
									println!("Vertex {new_index} added so that contour fits line better");
								}
							}

							let next_index = vertices.len();
							vertices.push(next_vertex);
							convex_bezier_indices.push(next_index as u32);

							let index = vertices.len();
							vertices.push(vertex);
							convex_bezier_indices.push(index as u32);

							let previous_index = vertices.len();
							vertices.push(previous_vertex);
							convex_bezier_indices.push(previous_index as u32);
							if debug_mode {
								println!("Off Curve Convex Triangle with vertices ({next_index}, {index}, {previous_index}) Added (3)");
							}


							if intersects { 
								contour.indices[contour_indices_position] = Some(new_index);
							} else {
								RemovableVector::remove(&mut contour.indices, contour_indices_position).expect("contour.indices[contour_indices_position] should not be None.");
								contour.indices_removed += 1;
								//println!("Off Curve Convex Point Removed");
							}
						} else {
							let previous_index = vertices.len();
							vertices.push(previous_vertex);
							concave_bezier_indices.push(previous_index as u32);

							let index = vertices.len();
							vertices.push(vertex);
							concave_bezier_indices.push(index as u32);

							let next_index = vertices.len();
							vertices.push(next_vertex);
							concave_bezier_indices.push(next_index as u32);
							if debug_mode {
								println!("Off Curve Concave Triangle with vertices ({next_index}, {index}, {previous_index}) Added (3)");
							}
						}
					},
					None => {},
				}
			}
		}

		if debug_mode {
			println!("\n\nBefore Self Intersecting Contours");
			for contour in pre_processed_contours.iter() {
				println!("\n");
				contour.print();
			}
			println!("\n\n");
		}

		// --- Fix Self-Intersecting Contours ---[x_{41}, x_{42}, x_{43}, x_{44}, x_{45}, x_{46}, x_{47}, x_{48}, x_{49}, x_{50}, x_{51}, x_{52}, x_{53}, x_{54}, x_{55}, x_{56}, x_{57}, x_{58}, x_{41}]

		let mut contour_index: usize = 0;
		'contours_loop: loop {
			let contour = match pre_processed_contours.get_mut(contour_index) {
				Some(contour) => contour,
				None => break,
			};
			let mut first_intersection: Option<usize> = None;
			let mut second_intersection: Option<usize> = None;
			let mut first_intersection_line_segments: Option<( (&Vertex, &Vertex), (&Vertex, &Vertex) )> = None;
			for (contour_indices_position, vertex_index) in contour.indices.iter().enumerate() {
				if let None = vertex_index {
					continue;
				}
				let first_vertex_index = vertex_index.unwrap();
				let first_vertex = &vertices[first_vertex_index];
				let second_vertex_index = contour.indices.next(contour_indices_position).unwrap();
				let second_vertex = &vertices[second_vertex_index];

				for (inner_contour_indices_position, inner_vertex_index) in contour.indices.iter().enumerate() {
					if let None = inner_vertex_index {
						continue;
					}
					let third_vertex_index = inner_vertex_index.unwrap();
					let third_vertex = &vertices[third_vertex_index];
					let fourth_vertex_index = contour.indices.next(inner_contour_indices_position).unwrap();
					let fourth_vertex = &vertices[fourth_vertex_index];

					let intersect = (first_vertex, second_vertex).intersects((third_vertex, fourth_vertex));
					if intersect {
						match first_intersection_line_segments {
							None => {
								first_intersection = Some(contour_indices_position + 1);
								first_intersection_line_segments = Some(( (first_vertex, second_vertex), (third_vertex, fourth_vertex) ));

							},
							Some(first_intersection_line_segments) => {
								let same_intersectioin = ( (first_vertex, second_vertex), (third_vertex, fourth_vertex) ).equivalent(&first_intersection_line_segments);
								if same_intersectioin {
									let second_intersection = contour_indices_position + 1; // NEED TO WRAP ARROUND
									let intersection_point = (first_vertex, second_vertex).intersection_point((third_vertex, fourth_vertex));
									let intersection_point_index = vertices.len();
									let end_part = contour.indices.split_off(second_intersection);
									let mut mid_part = contour.indices.split_off(first_intersection.unwrap());
									vertices.push(intersection_point);
									if debug_mode {
										println!("Vertex {intersection_point_index} added because of self intersection");
									}
									contour.indices.push(Some(intersection_point_index));
									contour.indices.extend(end_part);

									contour.direction = contour.indices.get_direction(&vertices);

									
									mid_part.push(Some(intersection_point_index));

									let mid_part_direction = mid_part.get_direction(&vertices);
									if debug_mode {
										println!("\n\n Contour: {mid_part:?}");
										println!("is {mid_part_direction:?}");
									}

									pre_processed_contours.push(Contour {
										indices: mid_part,
										indices_removed: 0,
										direction: mid_part_direction,

									});

									if debug_mode {
										println!("\n\n Removed Self Intersection");
										for contour in pre_processed_contours.iter() {
											println!("\n");
											contour.print();
										}
									}
									continue 'contours_loop;
								}
							}
						}
					}

				}
			}
			contour_index += 1;
		}

		// --- Find Parent of Holes

		let mut parents: Vec<Option<usize>> = (0..pre_processed_contours.len()).map(|_| None).collect();
		for contour_index in 0..pre_processed_contours.len() {
			let contour = &pre_processed_contours[contour_index];
			if let Direction::CounterClockwise = contour.direction {
				let mut parent: Option<usize> = None;

				for (parent_index, parent_contour) in pre_processed_contours.iter().enumerate() {
					if let Direction::Clockwise = parent_contour.direction {
						let inside = contour.inside(parent_contour, &vertices);
						if inside {
							if let Some(old_parrent_index) = parent {
								let old_parent_contour = &pre_processed_contours[old_parrent_index];
								let new_parent_inside_old_parent = parent_contour.inside(old_parent_contour, &vertices);
								if new_parent_inside_old_parent {
									parent = Some(parent_index);
								}
							} else {
								parent = Some(parent_index);
							}
						} else {
						}
					}
				};
				match parent {
					Some(parent) => {
						parents[contour_index] = (Some(parent));
					},
					None => {
						println!("Hole Parent Does Not Have Hole. Assuming Font is not Spec-Compliant. Reversing contour.indices and swapping contour.direction");
						let contour_mut = &mut pre_processed_contours[contour_index];
						contour_mut.indices.reverse();
						contour_mut.direction = contour_mut.indices.get_direction(&vertices);
						if let Direction::CounterClockwise = contour_mut.direction {
							println!("Direction is still counter clockwise");
							return Err(GlyphParseError::HoleDoesNotHaveParent);
						}
					},
				};


			}
		};

		if debug_mode {
			vertices.print();
			println!("\n\nBefore Holes Moved to Parents");
			for contour in pre_processed_contours.iter() {
				println!("\n");
				contour.print();
			}
			println!("\n\n");
		}

		// --- Moves Hole Indices to Parent ---

		let mut channeled: Vec<bool> = (0..vertices.len()).map(|_| false).collect();
		for contour_index in 0..pre_processed_contours.len() {
			let direction = pre_processed_contours[contour_index].direction;
			if let Direction::CounterClockwise = direction {
				let mut distance_squared = i64::MAX;
				let mut parent_index: Option<usize> = None;
				let mut child_index: Option<usize> = None;

				let contour_indices = pre_processed_contours[contour_index].indices.clone();
				let parent = &(pre_processed_contours[parents[contour_index].unwrap()].indices);

				for (contour_indices_index, child_index_current) in contour_indices.iter().enumerate() {
					if let None = child_index_current {
						continue;
					}
					let child_index_current = child_index_current.unwrap();
					for (parent_indices_index, parent_index_current) in parent.iter().enumerate() {
						if let None = parent_index_current {
							continue;
						}
						let parent_index_current = parent_index_current.unwrap();
						let child_vertex = &vertices[child_index_current];
						let parent_vertex = &vertices[parent_index_current];
						let distance_squared_current = (child_vertex.x.value as i64 - parent_vertex.x.value as i64).pow(2) + (child_vertex.y.value as i64 - parent_vertex.y.value as i64).pow(2);
						if (distance_squared_current < distance_squared) && child_vertex.on_curve && parent_vertex.on_curve && !channeled[parent_index_current] {
							//let first_vertex_index = contour_indices.previous(child_index_current).unwrap();
							let first_vertex_index = child_index_current; // IF THIS POINT IS OFF CURVE && CONVEX SHOULD BE PREVIOUS POINT ????????
							let second_vertex_index = contour_indices.next(contour_indices_index).unwrap();

							let first_vertex = &vertices[first_vertex_index];
							let second_vertex = &vertices[second_vertex_index];
							let intersects = (child_vertex, parent_vertex).intersects((first_vertex, second_vertex));
							let mut intersects_other_children = false;
							for (other_child_contour_index, other_child_contour) in pre_processed_contours.iter().enumerate() {
								if parents[other_child_contour_index] == parents[contour_index] {
									for (other_child_indices_position, other_child_index) in other_child_contour.indices.iter().enumerate() {
										if let None = other_child_index {
											continue;
										}
										let first_other_child_vertex = &vertices[other_child_index.unwrap()];
										let second_other_child_vertex = &vertices[other_child_contour.indices.next(other_child_indices_position).unwrap()];
										intersects_other_children = intersects_other_children || (child_vertex, parent_vertex).intersects((first_other_child_vertex, second_other_child_vertex));
									}
								}
							}
							if !(intersects || intersects_other_children) {
								distance_squared = distance_squared_current;
								parent_index = Some(parent_indices_index);
								child_index = Some(contour_indices_index);
							}
						}
					}
				}

				let child_index = match child_index {
					Some(child_index) => child_index,
					None => {
						println!("\nNo Valid Channel");
						return Err(GlyphParseError::NoValidChannel);
					}
				};
				let parent_index = parent_index.unwrap();

				let parent = &mut (pre_processed_contours[parents[contour_index].unwrap()].indices);

				let mut child_indices = contour_indices;
				channeled[child_indices[child_index].unwrap()] = true;
				let mut child_indices_after_splice = child_indices.split_off(child_index);
				child_indices_after_splice.extend(child_indices);
				child_indices_after_splice.push(child_indices_after_splice.next(child_indices_after_splice.len() - 1));
				//child_indices_after_splice.push(child_indices_after_splice[0]);


				channeled[parent[parent_index].unwrap()] = true;
				parent.reserve(child_indices_after_splice.len() + 1);
				let parent_indices_after_splice = parent.split_off(parent_index);
				parent.push(parent_indices_after_splice.next(parent_indices_after_splice.len() - 1));
				//parent.push(parent_indices_after_splice[0]);


				parent.extend(child_indices_after_splice);
				parent.extend(parent_indices_after_splice);
				pre_processed_contours[parents[contour_index].unwrap()].indices_removed += pre_processed_contours[contour_index].indices_removed;
			}
		}

		// --- Removing Overlaping Vertices

		for contour in pre_processed_contours.iter_mut() {
			for (contour_indices_position) in 0..contour.indices.len() {
				let index = contour.indices[contour_indices_position];
				match index {
					Some(index) => {
						let previous_index = contour.indices.previous(contour_indices_position).unwrap();
						let vertex = &vertices[index];
						let previous_vertex = &vertices[previous_index];
						//if (vertex.same_position(previous_vertex) && vertex != previous_vertex) {
						//	println!("SAME POSITION BUT DIFFERENT VERTEX");
						//}
						if vertex.same_position(previous_vertex) {
							RemovableVector::remove(&mut contour.indices, contour_indices_position).expect("contour.indices[contour_indices_position] should not be None.");
							contour.indices_removed += 1;
							//println!("Removed Duplicated Vertex");
						}
					},
					None => {},
				}
			}
		}

		// Recalculate Indices Removed

		for contour in pre_processed_contours.iter_mut() {
			let mut nones = 0;
			for index in contour.indices.iter() {
				if let None = index {
					nones += 1;
				}
			}
			contour.indices_removed = nones;
		}


		// --- Calculates Triangles ---

		// --------------------------------------------------------------------------------------
		//    MAKE THIS REMOVED IF NOT DEBUG BUILD
		// --------------------------------------------------------------------------------------
		if debug_mode {
			vertices.print();
			println!("\nProcessed Contours");
			for contour in pre_processed_contours.iter() {
				println!("\n");
				contour.print();
			}
			println!("\n\n Stepping Through");
		}
		//println!("{}", vertices.len());
		let mut indices: Vec<u32> = Vec::new();
		for contour in pre_processed_contours.iter_mut() {
			if let Direction::CounterClockwise = contour.direction {
				continue;
			}
			if debug_mode {
				println!("\n\n");
				contour.print();
			}
			let length = contour.indices.len();
			let mut current_index: usize = 0;
			let mut last_index_processed: i64 = -1;
			while contour.indices_removed < (length - 2) {
				if current_index as i64 == last_index_processed {
					println!("\nStuck in Triangulisation");
					return Err(GlyphParseError::StuckInTriangulisationLoop);
				}
				let centre_index = match contour.indices[current_index] {
					Some(index) => index,
					None => {
						current_index = (current_index + 1).rem_euclid(length);
						continue;
					},
				};
				let previous_index = contour.indices.previous(current_index).unwrap();
				let next_index = contour.indices.next(current_index).unwrap();

				let centre_point = &vertices[centre_index];
				let previous_point = &vertices[previous_index];
				let next_point = &vertices[next_index];

				let x_1 = (previous_point.x - centre_point.x).value as i64; // SO THAT MULTIPLACTION DOESN'T OVERFLOW
				let y_1 = (previous_point.y - centre_point.y).value as i64;
				let x_2 = (next_point.x - centre_point.x).value as i64;
				let y_2 = (next_point.y - centre_point.y).value as i64;

				let direction = ( (x_1 * y_2) >= (y_1 * x_2) ).into(); // CHANGED TO GE

				let mut ear = false;

				if debug_mode {
					println!("\n\n");
				}

				if let Direction::Clockwise = direction {
					let all_outside: bool = contour.indices.iter().map(|index| {
						match index {
							Some(index) => {
								let point = &vertices[*index];
								if point.same_position(previous_point) || point.same_position(centre_point) || point.same_position(next_point) {
									true
								} else {
									!(previous_point, centre_point, next_point).inside(point)
								}
							},
							None => {true},
						}
					}).fold(true, |acc, v| acc && v);

					if all_outside {
						indices.push(next_index as u32);
						indices.push(centre_index as u32);
						indices.push(previous_index as u32);
						contour.indices_removed += 1;
						last_index_processed = current_index as i64;
						RemovableVector::remove(&mut contour.indices, current_index).expect("contour.indices[current_index] should not be None.");
						ear = true;
					} else {
						if debug_mode{
							println!("Not Ear Because Point Inside");
						}
					}
				} else {
					if debug_mode {
						println!("Not Ear becase CounterClockwise");
					}
				}

				if debug_mode {
					println!("Contour Length: {}", length);
					println!("Indices Removed: {}", contour.indices_removed);
					println!("Considering Contour Index: {current_index}");
					println!("Previous Vertex Index: {previous_index}");
					println!("Centre Vertex Index: {centre_index}");
					println!("Next Vertex Index: {next_index}");
					contour.print();
					if ear {
						println!("Is an Ear");
					} else {
						println!("Not Ear");
					}
					//println!("Contour: {contour:?}");
					let mut buffer = String::new();
					std::io::stdin().read_line(&mut buffer);
				}

				current_index = (current_index + 1).rem_euclid(length); // IF REMOVED REDUCE BY ONE INSTEAD????
				if last_index_processed == -1 {
					last_index_processed = 0;
				}
			}
		}

		Ok((vertices, indices, convex_bezier_indices, concave_bezier_indices))
	}
}

#[derive(Debug)]
enum RemovableVectorError {
	AlreadyRemoved,
}
trait RemovableVector<T> {
	fn next(&self, index: usize) -> Option<T>;
	fn previous(&self, index: usize) -> Option<T>;
	fn remove(&mut self, index: usize) -> Result<(), RemovableVectorError>;
}

impl RemovableVector<usize> for Vec<Option<usize>> {
	fn next(&self, index: usize) -> Option<usize> {
		let start_index = index;
		let mut index = index;
		let length = self.len();
		loop {
			index = (index + 1).rem_euclid(length);
			match self[index] {
				Some(result) => return Some(result),
				None => {
					if start_index == index {
						return None;
					}
				},
			}
		}
	}

	fn previous(&self, index: usize) -> Option<usize> {
		let start_index = index;
		let mut index = index;
		let length = self.len();
		loop {
			index = (index as i64 - 1).rem_euclid(length as i64) as usize;
			match self[index] {
				Some(result) => return Some(result),
				None => {
					if start_index == index {
						return None;
					}
				},
			}
		}
	}

	fn remove(&mut self, index: usize) -> Result<(), RemovableVectorError> {
		match self[index] {
			Some(_) => {self[index] = None; Ok(())},
			None => Err(RemovableVectorError::AlreadyRemoved),
		}
	}
}

impl From<&Point> for Vertex {
	fn from(value: &Point) -> Self {
		let on_curve = (value.flag & 0x01) == 1;
		Vertex{ x: value.x.into(), y: value.y.into(), on_curve, uv_coords: [0.0, 0.0], }
	}
}

pub struct GlyphIntermediate {
	pub number_of_contours: Option<u16>,
	pub bounds: Bounds,
	pub glyph_data: GlyphDataIntermediate,
}

impl From<[i16; 4]> for Bounds {
	fn from(value: [i16; 4]) -> Self {
		Bounds {
			x_min: value[0],
			x_max: value[1],
			y_min: value[2],
			y_max: value[3],
		}
	}
}

pub enum GlyphDataIntermediate {
	CompositeGlyph(GlyphCompositeIntermediate),
	SimpleGlyph(GlyhpSimpleIntermediate),
	None,
}

impl From<GlyphDataRaw> for GlyphDataIntermediate {
	fn from(value: GlyphDataRaw) -> Self {
		match value {
			GlyphDataRaw::CompositeGlyphRaw(value) => GlyphDataIntermediate::CompositeGlyph(value.into()),
			GlyphDataRaw::SimpleGlyphRaw(value) => GlyphDataIntermediate::SimpleGlyph(value.into()),
			GlyphDataRaw::None => GlyphDataIntermediate::None,
		}
	}
}

impl From<GlyphRaw> for GlyphIntermediate {
	fn from(value: GlyphRaw) -> Self {
		let number_of_contours = if value.number_of_contours > 0 {
			Some(value.number_of_contours as u16)
		} else {
			None
		};
		let bounds = [value.x_min, value.x_max, value.y_min, value.y_max].into();
		let glyph_data = value.glyph_data.into();
		GlyphIntermediate {
			number_of_contours,
			bounds,
			glyph_data,
		}
	}
}

pub struct GlyphCompositeIntermediate {
	pub children: Vec<GlyphComponentIntermediate>
}

impl From<CompositeGlyphRaw> for GlyphCompositeIntermediate {
	fn from(value: CompositeGlyphRaw) -> Self {
		let children = value.children.into_iter().map(|v| v.into()).collect();
		GlyphCompositeIntermediate {
			children,
		}
	}
}

pub struct GlyphComponentIntermediate {
	pub flag: u16,
	pub glyph_index: u16,
	pub offset: Offset,
	pub transformation_matrix: TransformationMatrix2x2,
}

pub struct Offset {
	pub x: i32,
	pub y: i32,
}

impl From<(i32, i32)> for Offset {
	fn from((x, y): (i32, i32)) -> Self {
		Offset { x, y, }
	}
}

#[derive(Debug)]
pub struct TransformationMatrix2x2 {
	p11: f32,
	p12: f32,
	p21: f32,
	p22: f32,
}

impl TransformationMatrix2x2 {
	fn identity_scaled(x_scale: f32, y_scale: f32) -> Self {
		TransformationMatrix2x2 {
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

impl From<[Option<Fixed2Dot14>; 4]> for TransformationMatrix2x2 {
	fn from(value: [Option<Fixed2Dot14>; 4]) -> Self {
		match value {
			[None, None, None, None] => TransformationMatrix2x2::identity_scaled(1.0, 1.0),
			[Some(scale), None, None, None] => TransformationMatrix2x2::identity_scaled(scale.into(), scale.into()),
			[Some(x_scale), Some(y_scale), None, None] => TransformationMatrix2x2::identity_scaled(x_scale.into(), y_scale.into()),
			[Some(p11), Some(p12), Some(p21), Some(p22)] => TransformationMatrix2x2 { p11: p11.into(), p12: p12.into(), p21: p21.into(), p22: p22.into(), },
			_ => panic!("Transformation number should have one of the above patterns.")
		}
	}
}

impl From<ComponentGlyphRaw> for GlyphComponentIntermediate {
	fn from(value: ComponentGlyphRaw) -> Self {
		let flag = value.flag;
		let glyph_index = value.glyph_index;
		let offset = (value.x_offset_point, value.y_offset_point).into();
		let transformation_matrix = [
			value.transform_0.map(|v| v.into()),
			value.transform_1.map(|v| v.into()),
			value.transform_2.map(|v| v.into()),
			value.transform_3.map(|v| v.into()),
		].into();

		GlyphComponentIntermediate {
			flag,
			glyph_index,
			offset,
			transformation_matrix,
		}
	}
}

#[derive(Clone, Copy)]
struct Fixed2Dot14(u16);

impl From<Fixed2Dot14> for f32 {
	fn from(input: Fixed2Dot14) -> Self {
		let integer_part = (input.0 >> 14) as f32;
		let float_part = (input.0 & 0b0011_1111_1111_1111) as f32 / 2u32.pow(14) as f32;

		integer_part + float_part
	}
}

impl From<u16> for Fixed2Dot14 {
	fn from(value: u16) -> Self {
		Fixed2Dot14(value)
	}
}

pub struct GlyhpSimpleIntermediate {
	pub contours: Vec<Contour>,
	pub points: Vec<Point>,
}

pub struct Point {
	pub flag: u8,
	pub x: i16,
	pub y: i16,
}

impl Point {
	pub fn same_position(&self, other_point: &Self) -> bool{
		(self.x == other_point.x) && (self.y == other_point.y)
	}
}

#[derive(Debug, Clone)]
pub struct Contour {
	pub indices: Vec<Option<usize>>,
	pub indices_removed: usize,
	pub direction: Direction,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Direction {
	Clockwise,
	CounterClockwise,
}

impl From<bool> for Direction {
	fn from(value: bool) -> Self {
		match value {
			true => Direction::Clockwise,
			false => Direction::CounterClockwise,
		}
	}
}

impl Into<bool> for Direction {
	fn into(self) -> bool {
		match self {
			Direction::Clockwise => true,
			Direction::CounterClockwise => false,
		}
	}
}

impl Direction {
	fn opposite(self) -> Self {
		match self {
			Direction::Clockwise => Direction::CounterClockwise,
			Direction::CounterClockwise => Direction::Clockwise,
		}
	}
}

impl From<SimpleGlyphRaw> for GlyhpSimpleIntermediate {
	fn from(value: SimpleGlyphRaw) -> Self {
		let mut start: u16 = 0;

		let points: Vec<Point> = value.flags.into_iter().zip(value.x_coordinates.into_iter()).zip(value.y_coordinates.into_iter()).map(|((flag, x), y)| Point { flag, x, y, }).collect();
		
		let mut contours = Vec::new();
		for end_point in value.end_points_of_contours.into_iter() {
			let start_point = start;
			let indices: Vec<Option<usize>> = (start_point as usize ..= end_point as usize).map(|v| (Some(v))).collect();
			let direction = indices.get_direction(&points);
			contours.push(Contour {
				indices,
				indices_removed: 0,
				direction,
			});
			start = end_point + 1;
		};

		GlyhpSimpleIntermediate { contours, points, }
	}
}

pub trait GetDirection<T> {
	fn get_direction(&self, vertices: &Vec<T>) -> Direction;
}

impl GetDirection<Vertex> for Vec<Option<usize>> {
	fn get_direction(&self, vertices: &Vec<Vertex>) -> Direction {
		let mut lowest_y: FontUnits<i16> = i16::MAX.into();
		let mut highest_x: FontUnits<i16> = i16::MIN.into();
		let mut chosen_indices_position: usize = 0;
		for (indices_position, index) in self.iter().enumerate() {
			match index {
				Some(index) => {	
					let point = &vertices[*index];
					if (point.y < lowest_y) || ((point.y == lowest_y) && (point.x > highest_x)) {
						lowest_y = point.y;
						highest_x = point.x;
						chosen_indices_position = indices_position;
					}
				},
				None => {},
			}
		}

		let centre_point = &vertices[self[chosen_indices_position].unwrap()];

		let previous_index = match {
			let start_index = chosen_indices_position;
			let mut index = chosen_indices_position;
			let length = self.len();
			loop {
				index = (index as i64 - 1).rem_euclid(length as i64) as usize;
				if start_index == index {
					break None;
				}
				match self[index] {
					Some(result) => {
						if !(vertices[result].same_position(centre_point)) {
							break Some(result);
						}
					},
					None => {
					},
				}
			}
		} {
			Some(index) => index,
			None => self.previous(chosen_indices_position).unwrap(),
		};

		let next_index = match {
			let start_index = chosen_indices_position;
			let mut index = chosen_indices_position;
			let length = self.len();
			loop {
				index = (index  + 1).rem_euclid(length);
				if start_index == index {
					break None;
				}
				match self[index] {
					Some(result) => {
						if !(vertices[result].same_position(centre_point)) {
							break Some(result);
						}
					},
					None => {
					},
				}
			}
		} {
			Some(index) => index,
			None => self.next(chosen_indices_position).unwrap(),
		};

		let previous_point = &vertices[previous_index];
		let next_point = &vertices[next_index];

		(previous_point, centre_point).to_right_of(next_point, true).into()
	}
}

impl GetDirection<Point> for Vec<Option<usize>> {
	fn get_direction(&self, vertices: &Vec<Point>) -> Direction {
		let mut lowest_y = i16::MAX;
		let mut highest_x = i16::MIN;
		let mut chosen_indices_position: usize = 0;
		for (indices_position, index) in self.iter().enumerate() {
			match index{
				Some(index) => {
					let point = &vertices[*index];
					if (point.y < lowest_y) || ((point.y == lowest_y) && (point.x > highest_x)) {
						lowest_y = point.y;
						highest_x = point.x;
						chosen_indices_position = indices_position;
					}
				},
				None => {},
			}
		}

		let centre_point = &vertices[self[chosen_indices_position].unwrap()];

		let previous_index = match {
			let start_index = chosen_indices_position;
			let mut index = chosen_indices_position;
			let length = self.len();
			loop {
				index = (index as i64 - 1).rem_euclid(length as i64) as usize;
				if start_index == index {
					break None;
				}
				match self[index] {
					Some(result) => {
						if !(vertices[result].same_position(centre_point)) {
							break Some(result);
						}
					},
					None => {
					},
				}
			}
		} {
			Some(index) => index,
			None => self.previous(chosen_indices_position).unwrap(),
		};

		let next_index = match {
			let start_index = chosen_indices_position;
			let mut index = chosen_indices_position;
			let length = self.len();
			loop {
				index = (index  + 1).rem_euclid(length);
				if start_index == index {
					break None;
				}
				match self[index] {
					Some(result) => {
						if !(vertices[result].same_position(centre_point)) {
							break Some(result);
						}
					},
					None => {
					},
				}
			}
		} {
			Some(index) => index,
			None => self.next(chosen_indices_position).unwrap(),
		};

		let previous_point = &vertices[previous_index];
		let next_point = &vertices[next_index];

		let x_1 = (previous_point.x - centre_point.x) as i64;
		let y_1 = (previous_point.y - centre_point.y) as i64;
		let x_2 = (next_point.x - centre_point.x) as i64;
		let y_2 = (next_point.y - centre_point.y) as i64;

		( (x_1 * y_2) > (y_1 * x_2) ).into()
	}
}