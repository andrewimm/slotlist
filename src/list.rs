#[cfg(feature = "std")]
use std::vec::Vec;
#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

/// Rather than store `Option` elements, a SlotList uses a custom maybe type
/// that associates a number with each empty slot. These numbers allow the
/// collection to create a linked list of empty slots, reducing an insertion
/// complexity that would otherwise be `O(n)`.
#[derive(Copy, Clone, Debug)]
pub enum Slot<T: Sized> {
  Occupied(T),
  Empty(Option<usize>),
}

impl<T> Slot<T> {
  pub fn replace(&mut self, value: T) -> Slot<T> {
    core::mem::replace(self, Slot::Occupied(value))
  }

  pub fn take(&mut self) -> Slot<T> {
    core::mem::take(self)
  }

  pub fn set_next_empty(&mut self, index: usize) {
    match self {
      Slot::Occupied(_) => panic!("Can't modify empty chain for an occupied slot"),
      Slot::Empty(next) => *next = Some(index),
    }
  }

  pub fn as_option_of_ref(&self) -> Option<&T> {
    match self {
      Slot::Occupied(ref value) => Some(value),
      Slot::Empty(_) => None,
    }
  }

  pub fn as_mut(&mut self) -> Option<&mut T> {
    match *self {
      Slot::Occupied(ref mut value) => Some(value),
      Slot::Empty(_) => None,
    }
  }

  pub fn is_occupied(&self) -> bool {
    match self {
      Slot::Occupied(_) => true,
      Slot::Empty(_) => false,
    }
  }

  pub fn occupied(self) -> Option<T> {
    match self {
      Slot::Occupied(value) => Some(value),
      Slot::Empty(_) => None,
    }
  }
}

impl<T> Default for Slot<T> {
  fn default() -> Slot<T> {
    Slot::Empty(None)
  }
}

/// SlotList is a vector-like data structure where every entry, or "slot," is an
/// `Option` that may contain a value. The added value of a SlotList over a
/// `Vec<Option<T>>` is that inserting a new value tries to re-use any empty
/// slots before allocating new space. This creates a data structure with two
/// properties: the index of a given element will always remain static, and
/// elements can be removed without wasting space.
pub struct SlotList<T: Sized> {
  first_empty_slot: Option<usize>,
  last_empty_slot: Option<usize>,
  slots: Vec<Slot<T>>,
}

impl<T: Sized> SlotList<T> {
  /// Construct a new SlotList with no elements. A new, empty list will not
  /// allocate any memory, and can be a `const` value.
  pub const fn new() -> SlotList<T> {
    SlotList {
      first_empty_slot: None,
      last_empty_slot: None,
      slots: Vec::new(),
    }
  }

  /// Preallocate a SlotList with enough memory to store the requested number of
  /// elements.
  pub fn with_capacity(capacity: usize) -> SlotList<T> {
    SlotList {
      first_empty_slot: None,
      last_empty_slot: None,
      slots: Vec::with_capacity(capacity),
    }
  }

  pub fn capacity(&self) -> usize {
    self.slots.capacity()
  }

  /// Locate the first empty slot that can be used to store a value, returning
  /// its numeric index. If none is found, the list will push an empty slot onto
  /// the end and return the index of that slot.
  fn find_empty_slot(&mut self) -> usize {
    let mut index = self.slots.len();

    if let Some(first_index) = self.first_empty_slot {
      // An empty slot exists, so re-use it
      index = first_index;
      let empty = self.slots.get(first_index).unwrap();
      let next_first = match empty {
        Slot::Occupied(_) => panic!("Empty slot chain was broken"),
        Slot::Empty(next) => *next,
      };
      self.first_empty_slot = next_first;
    }

    if self.first_empty_slot.is_none() {
      // This implies that there are no more empty slots.
      // After the first element has been placed on the list, it maintains at
      // least one empty entry at all times that can be used for the next insert
      // operation.
      // If there was no first empty slot (should only be true for a newly
      // initialized list), this also guarantees that the initial index value
      // set at dthe top of the function will point to an empty entry.
      let mut last_entry = self.slots.len();
      self.slots.push(Slot::Empty(None));
      if last_entry == 0 {
        self.slots.push(Slot::Empty(None));
        last_entry += 1;
      }
      self.first_empty_slot = Some(last_entry);
      self.last_empty_slot = Some(last_entry);
    }

    index
  }

  /// Insert a new value into the list. This will attempt to use an empty slot,
  /// before allocating a new one at the end
  pub fn insert(&mut self, item: T) -> usize {
    let index = self.find_empty_slot();
    self.slots[index] = Slot::Occupied(item);
    index
  }

  /// Retrieve a reference to the value at the specified index
  pub fn get(&self, index: usize) -> Option<&T> {
    let slot = self.slots.get(index)?;
    match slot {
      Slot::Occupied(item) => Some(item),
      Slot::Empty(_) => None,
    }
  }

  /// Retrieve a mutable reference to the value at the specified index
  pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
    let slot = self.slots.get_mut(index)?;
    slot.as_mut()
  }

  /// Remove the value at the specified index, returning the value that was
  /// stored there
  pub fn remove(&mut self, index: usize) -> Option<T> {
    let slot = self.slots.get_mut(index)?;
    let prev = slot.take();

    if prev.is_occupied() {
      // `index` now represents the latest in the chain of empty slots
      if let Some(last_slot_index) = self.last_empty_slot {
        self.slots
          .get_mut(last_slot_index)
          .unwrap()
          .set_next_empty(index);
      }
      self.last_empty_slot = Some(index);
    }

    prev.occupied()
  }

  /// Set a specific slot to the provided value, returning the value that was
  /// previously stored there.
  /// This may require fixing up the empty slot chain, and in a worst-case
  /// scenario the complexity of this method becomes O(n).
  pub fn replace(&mut self, index: usize, item: T) -> Option<T> {
    if index >= self.slots.len() {
      panic!("Index out of bounds");
    }
    let slot = self.slots.get_mut(index).unwrap();
    let prev = slot.replace(item);

    if let Slot::Empty(next) = prev {
      // `index` represented an element in the empty chain
      // To fix up the chain, we need to replace pointers to it
      let mut current = self.first_empty_slot;
      while let Some(current_index) = current {
        let current_slot = self.slots.get_mut(current_index).unwrap();
        current = match current_slot {
          Slot::Occupied(_) => panic!("Empty slot chain was broken"),
          Slot::Empty(next_slot) => *next_slot,
        };
        if current == Some(index) {
          *current_slot = Slot::Empty(next);
          // If the removed empty slot was the last in the chain, update the
          // pointer to the new last item
          if self.last_empty_slot == Some(index) {
            self.last_empty_slot = Some(current_index);
          }
          current = None;
        }
      }
    }
    
    prev.occupied()
  }

  /// Construct an iterator that will visit all of the occupied slots in
  /// increasing index order
  pub fn iter(&self) -> impl Iterator<Item = &T> {
    self.slots.iter().filter_map(|i| i.as_option_of_ref())
  }

  /// Helper for testing chain consistency, only available in test mode
  #[cfg(test)]
  pub fn get_first_empty_slot(&self) -> Option<usize> {
    self.first_empty_slot
  }

  /// Helper for testing chain consistency, only available in test mode
  #[cfg(test)]
  pub fn get_last_empty_slot(&self) -> Option<usize> {
    self.last_empty_slot
  }

  /// Helper for testing chain consistency, only available in test mode
  #[cfg(test)]
  pub fn get_raw_slot(&self, index: usize) -> Option<&Slot<T>> {
    self.slots.get(index)
  }
}

impl<T: Clone> Clone for SlotList<T> {
  fn clone(&self) -> Self {
    Self {
      first_empty_slot: self.first_empty_slot,
      last_empty_slot: self.last_empty_slot,
      slots: self.slots.clone(),
    }
  }
}

impl<T: core::fmt::Debug> core::fmt::Debug for SlotList<T> {
  fn fmt(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
    formatter.debug_list()
      .entries(self.slots.iter())
      .finish()
  }
}

#[cfg(test)]
mod tests {
  use super::{Slot, SlotList};

  #[test]
  fn initialization() {
    let mut list: SlotList<u32> = SlotList::new();
    assert_eq!(list.insert(5), 0);
  }

  #[test]
  fn inserting_items() {
    let mut list: SlotList<u32> = SlotList::with_capacity(3);
    assert_eq!(list.get(1), None);
    assert_eq!(list.insert(20), 0);
    assert_eq!(list.insert(30), 1);
    assert_eq!(list.insert(40), 2);
    assert_eq!(list.get(0), Some(&20));
    assert_eq!(list.get(1), Some(&30));
    assert_eq!(list.get(2), Some(&40));
    assert_eq!(list.get(3), None);
  }

  #[test]
  fn grow_to_fit() {
    let mut list: SlotList<u32> = SlotList::new();
    assert_eq!(list.get(1), None);
    assert_eq!(list.insert(20), 0);
    assert_eq!(list.insert(30), 1);
    assert_eq!(list.insert(40), 2);
    assert_eq!(list.get(0), Some(&20));
    assert_eq!(list.get(1), Some(&30));
    assert_eq!(list.get(2), Some(&40));
    assert_eq!(list.get(3), None);
  }

  #[test]
  fn removing_items() {
    let mut list: SlotList<u32> = SlotList::new();
    list.insert(55);
    list.insert(40);
    list.insert(60);
    assert_eq!(list.remove(1), Some(40));
    assert_eq!(list.get(1), None);
  }

  #[test]
  fn replacing_emptied_items() {
    let mut list: SlotList<u32> = SlotList::new();
    list.insert(11);
    list.insert(22);
    list.insert(33);
    list.remove(0);
    list.remove(1);
    // First it will fill the empty slot at the end of the list
    assert_eq!(list.insert(44), 3);
    // Another empty slot has been added to index 4, but that is at the end of
    // the empty chain. 
    // Next it will fill the previously freed slots at 0 and 1
    assert_eq!(list.insert(55), 0);
    assert_eq!(list.insert(66), 1);
    // Once those have been filled, the chain returns to point to slot 4
    assert_eq!(list.insert(77), 4);
  }

  #[test]
  fn replacing_empty_slot() {
    let mut list: SlotList<u32> = SlotList::new();
    list.insert(0);
    assert_eq!(list.get_first_empty_slot(), Some(1));
    assert_eq!(list.get_last_empty_slot(), Some(1));
    list.remove(0);
    assert_eq!(list.get_first_empty_slot(), Some(1));
    assert_eq!(list.get_last_empty_slot(), Some(0));
    // Replacing the last element in the "empty chain" should fix up the chain
    // and its pointers.
    assert_eq!(list.replace(0, 5), None);
    assert_eq!(list.get_first_empty_slot(), Some(1));
    assert_eq!(list.get_last_empty_slot(), Some(1));
    if let Slot::Empty(next) = list.get_raw_slot(1).unwrap() {
      assert!(next.is_none());
    } else {
      panic!("First slot was not empty");
    }
  }

  #[test]
  fn replacing_existing_entries() {
    let mut list: SlotList<u32> = SlotList::new();
    list.insert(1);
    list.insert(3);
    list.insert(5);
    list.remove(1);
    assert_eq!(list.replace(0, 10), Some(1));
    assert_eq!(list.replace(1, 12), None);
  }

  #[test]
  fn iterator() {
    let mut list: SlotList<u32> = SlotList::new();
    list.insert(1);
    list.insert(2);
    list.insert(1);
    list.insert(3);
    list.insert(1);

    list.remove(1);
    list.remove(3);
    let mut count = 0;
    for x in list.iter() {
      count += 1;
      assert_eq!(*x, 1);
    }
    assert_eq!(count, 3);
  }

  #[test]
  fn maintain_size() {
    let mut list: SlotList<u32> = SlotList::with_capacity(4);
    for _ in 0..100 {
      let index = list.insert(10);
      list.remove(index);
    }
    assert_eq!(list.capacity(), 4);
  }
}
