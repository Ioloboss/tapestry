use std::{path::Path, time::Instant};

use tapestry::{font::Font, linked_list::LinkedList};

/* fn main() {
	let mut linked_list = LinkedList::new();

	for i in 0..5 {
		linked_list.push_back(i);
	}

	println!("Items:");

	linked_list.go_to_start();

	for item in linked_list.iter() {
		println!("	Item: {}", item.borrow().get_item());
	}

	let mut other_list = LinkedList::new();

	for i in 100..105 {
		other_list.push_back(i);
	}

	linked_list.go_to_start();

	for _ in 0..2 {
		linked_list.advance();
	}

	linked_list.splice_clone(other_list);


	println!("Spliced Items:");

	linked_list.go_to_start();

	for item in linked_list.iter() {
		println!("	Item: {}", item.borrow().get_item());
	}
} */

fn main() {
	let filename = Path::new("./resources/fonts/NotoJP/static/NotoSansJP-Regular.ttf");

	let before = Instant::now();
	let font = Font::new(filename);
	let elapsed_time = before.elapsed();
	println!("Loading Font took {} milliseconds", elapsed_time.as_millis());
}