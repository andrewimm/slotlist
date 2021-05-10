# SlotList - A Vector-like collection that re-uses elements
===

SlotList is a collection designed for directly indexing elements that are
frequently added and removed. It guarantees that indexes are static --
a specific element will always be accesible at the same numeric index, no
matter how many other items are added and removed.

```rust
let mut list: SlotList<i32> = SlotList::new();
let index = list.insert(12);
// The item will always be available at `index` until it is removed
assert_eq!(list.get(index), Some(&12));
```

The other core property of SlotList is that it recycles previously emptied
slots. If the element at a particular index is removed, that index may be
re-used in the future for a new entry. This ensures that the list only
grows when it runs out of space.

```rust
let mut list: SlotList<i32> = SlotList::new();
let index = list.insert(50);
list.remove(index);
// Adding this element will not allocate any new entries, since it can
// re-use existing empty space
list.insert(55);
```

This behavior is ideal for collections where entires are frequently added and removed, especially over a long period of time.

An example use case (and what this was originally implemented for) is the
allocation of Unix-style file handles for a process. Over its lifetime, a
process may open and close many files which are accesible with numeric
handles. The memory allocated to this process for file handles should not
grow with the lifetime of the process, but only with the number of open
files.

```rust
// Trivial demonstration of file handle allocation using a SlotList
let mut open_files: SlotList<FileDescriptor> = SlotList::new();

fn open_file(descriptor: FileDescriptor) -> usize {
  open_files.insert(descriptor)
}

fn get_file(handle: usize) -> Option<&FileDescriptor> {
  open_files.get(handle)
}

fn close_file(handle: usize) -> Option<FileDescriptor> {
  open_files.remove(handle)
}
```
