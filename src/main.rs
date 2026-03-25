use std::{path::Path, time::Instant};

use tapestry::{font::Font, linked_list::LinkedList};

fn main() {
	let mut linked_list = LinkedList::new();

	for i in 0..5 {
		linked_list.push_back(i);
	}

	println!("Items:");

	linked_list.go_to_start();

	for item in linked_list.iter() {
		println!("	Item: {}", item.borrow().get_item());
	}

	linked_list.go_to_start();

	for _ in 0..2 {
		linked_list.advance();
	}

	println!("Inserting After: {}", linked_list.get_current().unwrap().borrow().get_item());

	let item = 5;

	linked_list.insert(item);

	println!("Inserted Items:");

	linked_list.go_to_start();

	for item in linked_list.iter() {
		println!("	Item: {}", item.borrow().get_item());
	}
}

/* fn main() {
	let filename = Path::new("./resources/fonts/NotoJP/static/NotoSansJP-Regular.ttf");

	let before = Instant::now();
	let font = Font::new(filename);
	let elapsed_time = before.elapsed();
	println!("Loading Font took {} milliseconds", elapsed_time.as_millis());
} */