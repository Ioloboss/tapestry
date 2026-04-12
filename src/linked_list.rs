use std::{cell::RefCell, fmt::Debug, rc::Rc};

pub struct LinkedListItem<T> {
	item: T,
	pub previous_item: Option<Rc<RefCell<LinkedListItem<T>>>>,
	pub next_item: Option<Rc<RefCell<LinkedListItem<T>>>>,
}

impl <T: Debug> Debug for LinkedListItem<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "LinkedListItem {{ item: {:?}, previous_item: {}, next_item: {} }}",
			self.item,
			match self.previous_item {
				Some(_) => "Some",
				None => "None",
			},
			match self.next_item {
				Some(_) => "Some",
				None => "None",
			}
		)
	}
}

impl <T> LinkedListItem<T> {
	pub fn get_item(&self) -> &T {
		&self.item
	}
}

pub mod LinkedListItemFunctions {
	use std::{cell::RefCell, rc::Rc};
	use crate::linked_list::LinkedList;

use super::LinkedListItem;

	pub fn insert_after <T> (current: &Rc<RefCell<LinkedListItem<T>>>, item: T) {
		let next_item = current.borrow().next_item.clone().unwrap();
		let new_item = Rc::new(RefCell::new(LinkedListItem {
			item,
			next_item: Some(next_item.clone()),
			previous_item: Some(current.clone())
		}));
		current.borrow_mut().next_item = Some(new_item.clone());
		next_item.borrow_mut().previous_item = Some(new_item);
	}

	pub fn splice_together <T> (parent: &Rc<RefCell<LinkedListItem<T>>>, child: &Rc<RefCell<LinkedListItem<T>>>) {
		let next_parent_item = parent.borrow().next_item.clone().unwrap();
		let previous_child_item = child.borrow().previous_item.clone().unwrap();

		parent.borrow_mut().next_item = Some(child.clone());
		previous_child_item.borrow_mut().next_item = Some(next_parent_item.clone());

		child.borrow_mut().previous_item = Some(parent.clone());
		next_parent_item.borrow_mut().previous_item = Some(previous_child_item.clone());

		// NEED TO CLEAN UP OTHER LINKED LIST YOURSELF
	}

	pub fn remove <T> (current: Rc<RefCell<LinkedListItem<T>>>, list: &mut LinkedList<T>) -> T {
		// TODO Check if next_item == previous_item == current
		let previous_item = current.borrow().previous_item.clone().unwrap();
		let next_item = current.borrow().next_item.clone().unwrap();

		if Rc::ptr_eq(&current, list.start.as_ref().unwrap()) {
			list.start = Some(next_item.clone());
		}

		if Rc::ptr_eq(&current, list.end.as_ref().unwrap()) {
			list.end = Some(previous_item.clone());
		}

		previous_item.borrow_mut().next_item = Some(next_item.clone());
		next_item.borrow_mut().previous_item = Some(previous_item.clone());

		match Rc::into_inner(current) {
			Some(inner) => inner.into_inner().item,
			None => {
				panic!("Element has references")
			},
		}
	}
}

#[derive(Debug)]
pub struct LinkedList<T> {
	pub start: Option<Rc<RefCell<LinkedListItem<T>>>>,
	pub end: Option<Rc<RefCell<LinkedListItem<T>>>>,
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
		}
	}

	pub fn loose_items(mut self) {
		self.start = None;
		self.end = None;

		drop(self);
	}

	pub fn loose_items_reference(&mut self) {
		self.start = None;
		self.end = None;
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
			self.start.clone().unwrap().borrow_mut().previous_item = Some(new_item.clone());
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
					second_to_last.borrow_mut().next_item = None;
					second_to_last.borrow_mut().previous_item = None;
				} else {
					second_to_last.borrow_mut().next_item = self.start.clone();
					self.start.as_ref().unwrap().borrow_mut().previous_item = Some(second_to_last.clone());
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
					second_to_first.borrow_mut().next_item = None;
					second_to_first.borrow_mut().previous_item = None;
				} else {
					second_to_first.borrow_mut().previous_item = self.end.clone();
					self.end.as_ref().unwrap().borrow_mut().next_item = Some(second_to_first.clone());
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