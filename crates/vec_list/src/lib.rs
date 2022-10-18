use std::{
    collections::LinkedList,
    fmt::Debug,
    mem::{ManuallyDrop, MaybeUninit},
};

use num_traits::{bounds::UpperBounded, FromPrimitive, ToPrimitive};

pub struct VecList<T, I: UpperBounded + ToPrimitive + PartialEq + Copy = usize> {
    nodes: Vec<Node<T, I>>,
    head: I,
    tail: I,
    empty_spots: Vec<I>,
}

impl<T, I: ToPrimitive + FromPrimitive + UpperBounded + PartialEq + Copy + Debug> VecList<T, I> {
    pub fn new() -> Self {
        Self {
            nodes: vec![],
            head: I::max_value(),
            tail: I::max_value(),
            empty_spots: vec![],
        }
    }

    /// Retains only the elements specified by the predicate.
    ///
    /// In other words, remove all elements `e` for which `f(&e)` returns `false`.
    /// This method operates in place, visiting each element exactly once in the
    /// original order, and preserves the order of the retained elements.
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&T) -> bool,
    {
        let mut i = self.head;
        while i != I::max_value() {
            let id = i.to_usize().unwrap();
            let value = unsafe { self.nodes[id].value.assume_init_ref() };
            let next = self.nodes[id].next;
            if !f(value) {
                self.remove(i);
            }
            i = next;
            if i == self.head {
                break;
            }
        }
    }

    pub fn push_back(&mut self, value: T) {
        if self.tail == I::max_value() {
            match self.empty_spots.pop() {
                None => {
                    let index = I::from_usize(self.nodes.len()).unwrap();
                    self.nodes.push(Node {
                        prev: index,
                        next: index,
                        value: MaybeUninit::new(value),
                    });
                    self.tail = index;
                }
                Some(index) => {
                    self.nodes[index.to_usize().unwrap()] = Node {
                        prev: index,
                        next: index,
                        value: MaybeUninit::new(value),
                    };
                    self.tail = index;
                }
            }
            if self.head == I::max_value() {
                self.head = self.tail;
            }
        } else {
            match self.empty_spots.pop() {
                None => {
                    let index = I::from_usize(self.nodes.len()).unwrap();
                    self.nodes.push(Node {
                        prev: self.tail,
                        next: self.head,
                        value: MaybeUninit::new(value),
                    });
                    self.nodes[self.tail.to_usize().unwrap()].next = index;
                    self.tail = index;
                }
                Some(index) => {
                    self.nodes[index.to_usize().unwrap()] = Node {
                        prev: self.tail,
                        next: self.head,
                        value: MaybeUninit::new(value),
                    };
                    self.nodes[self.tail.to_usize().unwrap()].next = index;
                    self.tail = index;
                }
            }
        }
    }

    pub fn iter<'a>(&'a mut self) -> VecListIter<'a, T, I> {
        VecListIter {
            current_node_id: self.head,
            list: self,
        }
    }

    pub fn remove(&mut self, index: I) -> Option<T> {
        debug_assert_ne!(index, I::max_value());
        let id = index.to_usize().unwrap();
        let node = self.nodes.get(id)?;
        if node.prev == I::max_value() && node.next == I::max_value() {
            return None;
        }
        if self.head == self.tail {
            self.head = I::max_value();
            self.tail = I::max_value();
            self.nodes[id].next = I::max_value();
            self.nodes[id].prev = I::max_value();
            self.empty_spots.push(self.head);
            return Some(unsafe { self.nodes[id].value.assume_init_read() });
        }
        if self.head == index {
            let new_head = node.next;
            let prev_id = self.nodes[id].prev.to_usize().unwrap();
            let next_id = self.nodes[id].next.to_usize().unwrap();
            self.nodes[prev_id].next = self.nodes[id].next;
            self.nodes[next_id].prev = self.nodes[id].prev;
            self.nodes[id].next = I::max_value();
            self.nodes[id].prev = I::max_value();
            self.empty_spots.push(self.head);
            self.head = new_head;
            return Some(unsafe { self.nodes[id].value.assume_init_read() });
        }
        if self.tail == index {
            let new_tail = node.prev;
            let prev_id = self.nodes[id].prev.to_usize().unwrap();
            let next_id = self.nodes[id].next.to_usize().unwrap();
            self.nodes[prev_id].next = self.nodes[id].next;
            self.nodes[next_id].prev = self.nodes[id].prev;
            self.nodes[id].next = I::max_value();
            self.nodes[id].prev = I::max_value();
            self.empty_spots.push(self.tail);
            self.tail = new_tail;
            return Some(unsafe { self.nodes[id].value.assume_init_read() });
        }
        let prev_id = self.nodes[id].prev.to_usize().unwrap();
        let next_id = self.nodes[id].next.to_usize().unwrap();
        self.nodes[prev_id].next = self.nodes[id].next;
        self.nodes[next_id].prev = self.nodes[id].prev;
        self.nodes[id].next = I::max_value();
        self.nodes[id].prev = I::max_value();
        self.empty_spots.push(index);
        return Some(unsafe { self.nodes[id].value.assume_init_read() });
    }
}

impl<T, I: UpperBounded + ToPrimitive + PartialEq + Copy> Drop for VecList<T, I> {
    fn drop(&mut self) {
        let mut i = self.head;
        while i != I::max_value() {
            let id = i.to_usize().unwrap();
            unsafe {
                self.nodes[id].value.assume_init_drop();
            }
            i = self.nodes[id].next;
            if i == self.head {
                break;
            }
        }
    }
}

struct Node<T, I> {
    prev: I,
    next: I,
    value: MaybeUninit<T>,
}

pub struct NodeItem<'a, T, I> {
    pub index: I,
    pub value: &'a T,
}

pub struct VecListIter<'a, T, I: UpperBounded + ToPrimitive + PartialEq + Copy> {
    current_node_id: I,
    list: &'a VecList<T, I>,
}

impl<'a, T, I: UpperBounded + ToPrimitive + PartialEq + Copy> Iterator for VecListIter<'a, T, I> {
    type Item = NodeItem<'a, T, I>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_node_id == I::max_value() {
            return None;
        }
        let id = self.current_node_id.to_usize().unwrap();
        let node = &self.list.nodes[id];
        let item = NodeItem {
            index: self.current_node_id,
            value: unsafe { node.value.assume_init_ref() },
        };
        self.current_node_id = node.next;
        if self.current_node_id == self.list.head {
            self.current_node_id = I::max_value();
        }
        Some(item)
    }
}

impl<'a, T, I: UpperBounded + ToPrimitive + PartialEq + Copy> IntoIterator for &'a VecList<T, I> {
    type IntoIter = VecListIter<'a, T, I>;
    type Item = NodeItem<'a, T, I>;

    fn into_iter(self) -> Self::IntoIter {
        VecListIter {
            current_node_id: self.head,
            list: self,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::VecList;

    #[test]
    fn push_back() {
        let mut list: VecList<i32> = VecList::new();
        list.push_back(1);
        list.push_back(2);
        list.push_back(3);
        let mut iter = list.iter();
        assert_eq!(*iter.next().unwrap().value, 1);
        assert_eq!(*iter.next().unwrap().value, 2);
        assert_eq!(*iter.next().unwrap().value, 3);
    }

    #[test]
    fn remove() {
        let mut list: VecList<i32> = VecList::new();
        list.push_back(1);
        list.push_back(2);
        list.push_back(3);
        let mut iter = list.iter();
        iter.next().unwrap();
        let second_id = iter.next().unwrap().index;
        assert_eq!(list.remove(second_id), Some(2));
        let mut iter = list.iter();
        assert_eq!(*iter.next().unwrap().value, 1);
        assert_eq!(*iter.next().unwrap().value, 3);
    }
}
