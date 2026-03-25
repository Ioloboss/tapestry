use std::{cell::RefCell, fmt::Debug, rc::Rc};

#[derive(Debug)]
pub struct LinkedListItem<T> {
	item: T,
	pub previous_item: Option<Rc<RefCell<LinkedListItem<T>>>>,
	pub next_item: Option<Rc<RefCell<LinkedListItem<T>>>>,
}

impl <T> LinkedListItem<T> {
	pub fn get_item(&self) -> &T {
		&self.item
	}
}

#[derive(Debug)]
pub struct LinkedList<T> {
	start: Option<Rc<RefCell<LinkedListItem<T>>>>,
	end: Option<Rc<RefCell<LinkedListItem<T>>>>,
	current: Option<Rc<RefCell<LinkedListItem<T>>>>,
}

impl <T> Drop for LinkedList<T> {
	fn drop(&mut self) {
		loop {
			if let Some(_) = self.pop_back() {} else {
				break;
			}
		}
	}
}

impl <T> LinkedList<T> {
	pub fn new() -> LinkedList<T> {
		Self {
			start: None,
			end: None,
			current: None,
		}
	}

	pub fn get_current(&self) -> Option<Rc<RefCell<LinkedListItem<T>>>> {
		self.current.clone()
	}

	pub fn go_to_start(&mut self) {
		self.current = self.start.clone();
	}

	pub fn go_to_end(&mut self) {
		self.current = self.end.clone();
	}

	pub fn advance(&mut self) {
		if let Some(item) = self.current.clone() {
			self.current = item.borrow().next_item.clone();
		}
	}

	pub fn advance_back(&mut self) {
		if let Some(item) = self.current.clone() {
			self.current = item.borrow().previous_item.clone();
		}
	}

	pub fn push_back(&mut self, item: T) {
		if let Some(last_item) = self.end.clone() {
			let new_item = LinkedListItem {
				item,
				previous_item: Some(last_item.clone()),
				next_item: self.start.clone(),
			};
			let new_item = Rc::new(RefCell::new(new_item));
			last_item.borrow_mut().next_item = Some(new_item.clone());
			self.end = Some(new_item);
		} else {
			let new_item = LinkedListItem {
				item,
				previous_item: None,
				next_item: None,
			};
			let new_item = Rc::new(RefCell::new(new_item));
			new_item.borrow_mut().next_item = Some(new_item.clone());
			new_item.borrow_mut().previous_item = Some(new_item.clone());
			self.start = Some(new_item.clone());
			self.end = Some(new_item);
		}
	}

	pub fn push_front(&mut self, item: T) {
		if let Some(first_item) = self.start.clone() {
			let new_item = LinkedListItem {
				item,
				previous_item: self.end.clone(),
				next_item: Some(first_item.clone()),
			};
			let new_item = Rc::new(RefCell::new(new_item));
			first_item.borrow_mut().previous_item = Some(new_item.clone());
			self.start = Some(new_item);
		} else {
			let new_item = LinkedListItem {
				item,
				previous_item: None,
				next_item: None,
			};
			let new_item = Rc::new(RefCell::new(new_item));
			new_item.borrow_mut().next_item = Some(new_item.clone());
			new_item.borrow_mut().previous_item = Some(new_item.clone());
			self.start = Some(new_item.clone());
			self.end = Some(new_item);
		}
	}

	pub fn pop_back(&mut self) -> Option<T> {
		if let Some(last_item) = self.end.clone() {
			{
				let second_to_last = last_item.borrow().previous_item.clone().unwrap();
				if Rc::ptr_eq(&second_to_last, &last_item) {
					self.start = None;
					self.end = None;
					self.current = None;
					second_to_last.borrow_mut().next_item = None;
					second_to_last.borrow_mut().previous_item = None;
				} else {
					second_to_last.borrow_mut().next_item = self.start.clone();
					self.start.as_ref().unwrap().borrow_mut().next_item = Some(second_to_last.clone());
					self.end = Some(second_to_last);
				}
			}

			Some(match Rc::into_inner(last_item) {
				Some(inner) => inner.into_inner().item,
				None => {
					panic!("Element has references")
				},
			})
		} else {
			None
		}
	}

	pub fn pop_front(&mut self) -> Option<T> {
		if let Some(first_item) = self.start.clone() {
			{
				let second_to_first = first_item.borrow().next_item.clone().unwrap();
				if Rc::ptr_eq(&second_to_first, &first_item) {
					self.start = None;
					self.end = None;
					self.current = None;
					second_to_first.borrow_mut().next_item = None;
					second_to_first.borrow_mut().previous_item = None;
				} else {
					second_to_first.borrow_mut().previous_item = self.end.clone();
					self.end.as_ref().unwrap().borrow_mut().previous_item = Some(second_to_first.clone());
					self.start = Some(second_to_first);
				}
			}

			Some(match Rc::into_inner(first_item) {
				Some(inner) => inner.into_inner().item,
				None => {
					panic!("Element has refereances")
				},
			})
		} else {
			None
		}
	}

	pub fn splice(&mut self, mut other_list: LinkedList<T>) {
		let current = self.current.clone().unwrap();
		let next = current.borrow().next_item.clone().unwrap();

		current.borrow_mut().next_item = other_list.start.clone();
		next.borrow_mut().previous_item = other_list.end.clone();
		other_list.start.as_ref().unwrap().borrow_mut().previous_item = Some(current);
		other_list.end.as_ref().unwrap().borrow_mut().next_item = Some(next);

		other_list.start = None;
		other_list.end = None;
		other_list.current = None;
	}

	pub fn insert(&mut self, item: T) {
		let current = self.current.clone().unwrap();
		let next = current.borrow().next_item.clone().unwrap();

		let new_item = LinkedListItem {
			item,
			previous_item: Some(current.clone()),
			next_item: Some(next.clone()),
		};
		let new_item = Rc::new(RefCell::new(new_item));

		current.borrow_mut().next_item = Some(new_item.clone());
		next.borrow_mut().previous_item = Some(new_item.clone());
	}
}

impl <T> LinkedList<T>
where
	T: Clone,
{
	pub fn splice_clone(&mut self, mut other_list: LinkedList<T>) {
		let current = self.current.clone().unwrap();
		let next = current.borrow().next_item.clone().unwrap();
		let current_clone = LinkedListItem {
			item: current.borrow().get_item().clone(),
			next_item: Some(next.clone()),
			previous_item: Some(current.clone()),
		};
		let current_clone = Rc::new(RefCell::new(current_clone));

		next.borrow_mut().previous_item = Some(current_clone.clone());

		current.borrow_mut().next_item = other_list.start.clone();
		current_clone.borrow_mut().previous_item = other_list.end.clone();
		other_list.start.as_ref().unwrap().borrow_mut().previous_item = Some(current);
		other_list.end.as_ref().unwrap().borrow_mut().next_item = Some(current_clone);

		other_list.start = None;
		other_list.end = None;
		other_list.current = None;
	}
}

impl <T, Iter> From<Iter> for LinkedList<T>
where
	Iter: Iterator<Item = T>,
{
	fn from(value: Iter) -> Self {
		let mut list= LinkedList::new();
		for element in value {
			list.push_back(element);
		}

		list
	}
}

pub enum Direction {
	Forward,
	Backward,
}

pub struct LinkedListIterator<'a, T> {
	direction: Direction,
	linked_list: &'a LinkedList<T>,
	current: Rc<RefCell<LinkedListItem<T>>>,
	finished: bool,
}

impl <'a, T> Iterator for LinkedListIterator<'a, T> {
	type Item = Rc<RefCell<LinkedListItem<T>>>;

	fn next(&mut self) -> Option<Self::Item> {
		if self.finished {
			return None;
		}
		if Rc::ptr_eq(&self.current, match self.direction {
			Direction::Forward => &self.linked_list.end.as_ref().unwrap(),
			Direction::Backward => &self.linked_list.start.as_ref().unwrap(),
		}) {
			self.finished = true;
		}
		let current = self.current.clone();
		match self.direction {
			Direction::Forward => {
				self.current = current.borrow().next_item.clone().unwrap();
			},
			Direction::Backward => {
				self.current = current.borrow().previous_item.clone().unwrap();
			},
		}
		Some(current)
	}
}

impl <'a, T> LinkedList<T> {
	pub fn iter(&'a self) -> LinkedListIterator<'a, T> {
		LinkedListIterator { direction: Direction::Forward, linked_list: self, current: self.start.clone().unwrap(), finished: false }
	}

	pub fn iter_reverse(&'a self) -> LinkedListIterator<'a, T> {
		LinkedListIterator { direction: Direction::Backward, linked_list: self, current: self.end.clone().unwrap(), finished: false }
	}
}