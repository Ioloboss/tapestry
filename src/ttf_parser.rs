use mircalla_types::vectors::Position;

use crate::{font::{Bounds, ComponentGlyph, Glyph, ToTriangles, Vertex}, linked_list::LinkedList};

pub mod raw_to_intermediate;
pub mod intermediate_to_final;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
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

pub trait GetDirection<T> {
	fn get_direction(&mut self, vertices: &Vec<T>) -> Direction;
}

pub struct Contour {
	indices: LinkedList<usize>,
	direction: Direction,
	empty: bool,
}

impl Contour {
	fn inside(&self, other_contour: &Contour, vertices: &Vec<Vertex>) -> bool {
		let mut inside = true;
		for inner_index in self.indices.iter() {
			let inner_vertex = &vertices[*inner_index.borrow().get_item()];
			let next_inner_index = inner_index.borrow().next_item.clone().unwrap();
			let next_inner_vertex = &vertices[*next_inner_index.borrow().get_item()];
			let inside_other_contour = inner_vertex.is_inside_contour(other_contour, vertices);
			let mut intersects = false;
			for outer_index in other_contour.indices.iter() {
				let outer_vertex = &vertices[*outer_index.borrow().get_item()];
				let next_outer_index = outer_index.borrow().next_item.clone().unwrap();
				let next_outer_vertex = &vertices[*next_outer_index.borrow().get_item()];
				intersects = intersects || (inner_vertex, next_inner_vertex).intersects((outer_vertex, next_outer_vertex))
			}
			inside = inside && (inside_other_contour && !intersects);
		}
		inside
	}
}

pub struct Point {
	flag: u8,
	position: Position<i16>,
}

impl Vertex {
	fn is_inside_contour(&self, contour: &Contour, vertices: &Vec<Vertex>) -> bool {
		let mut number_of_intersections = 0;
		for first_vertex_index in contour.indices.iter() {
			let second_vertex_index = first_vertex_index.borrow().next_item.clone().unwrap();
			let first_vertex = &vertices[*first_vertex_index.borrow().get_item()];
			let second_vertex = &vertices[*second_vertex_index.borrow().get_item()];

			let x_1: i64;
			let x_2: i64;
			let y_1: i64;
			let y_2: i64;
			if (first_vertex.position.y < second_vertex.position.y) {
				x_1 = (first_vertex.position.x - second_vertex.position.x).value as i64; // SO THAT MULTIPLACTION DOESN'T OVERFLOW
				y_1 = (first_vertex.position.y - second_vertex.position.y).value as i64;
				x_2 = (self.position.x - second_vertex.position.x).value as i64;
				y_2 = (self.position.y - second_vertex.position.y).value as i64;
			} else {
				x_1 = (second_vertex.position.x - first_vertex.position.x).value as i64; // SO THAT MULTIPLACTION DOESN'T OVERFLOW
				y_1 = (second_vertex.position.y - first_vertex.position.y).value as i64;
				x_2 = (self.position.x - first_vertex.position.x).value as i64;
				y_2 = (self.position.y - first_vertex.position.y).value as i64;
			}

			let vertex_east = ( (x_1 * y_2) < (y_1 * x_2) );
			let vertex_in_boundary = if (first_vertex.position.y < second_vertex.position.y) {
				(first_vertex.position.y < self.position.y) && (self.position.y <= second_vertex.position.y)
			} else {
				(second_vertex.position.y < self.position.y) && (self.position.y <= first_vertex.position.y)
			};
			if vertex_east && vertex_in_boundary {
				number_of_intersections += 1
			};
		}
		number_of_intersections % 2 == 1
	}

	fn sign(&self, vertex_1: &Vertex, vertex_2: &Vertex) -> bool {
		let x_1 = (vertex_1.position.x - vertex_2.position.x).value as i64;
		let y_1 = (vertex_1.position.y -vertex_2.position.y).value as i64;

		let x_2 = (self.position.x - vertex_2.position.x).value as i64;
		let y_2 = (self.position.y - vertex_2.position.y).value as i64;

		(x_1 * y_2) >= (y_1 * x_2)
	}

	fn is_inside_triangle(&self, vertex_1: &Vertex, vertex_2: &Vertex, vertex_3: &Vertex) -> bool {
		let orientation_1 = self.sign(vertex_1, vertex_2);
		let orientation_2 = self.sign(vertex_2, vertex_3);
		let orientation_3 = self.sign(vertex_3, vertex_1);

		(orientation_1 == orientation_2) && (orientation_2 == orientation_3)
	}
}

pub trait Intersects {
	fn intersects(self, other_line: Self) -> bool;
}

impl Intersects for (&Vertex, &Vertex) {
	fn intersects(self, other_line: Self) -> bool {
		let vertex_1 = self.0;
		let vertex_2 = self.1;
		let vertex_3 = other_line.0;
		let vertex_4 = other_line.1;

		let orientation_1 = vertex_3.sign(vertex_1, vertex_2);
		let orientation_2 = vertex_4.sign(vertex_1, vertex_2);

		let orientation_3 = vertex_1.sign(vertex_3, vertex_4);
		let orientation_4 = vertex_2.sign(vertex_3, vertex_4);

		if (vertex_1 == vertex_3) || (vertex_1 == vertex_4) || (vertex_2 == vertex_3) || (vertex_2 == vertex_4) {
			return false;
		}

		(orientation_1 != orientation_2) && (orientation_3 != orientation_4)
	}
}