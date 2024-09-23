// SPDX-License-Identifier: GPL-2.0

//! Double Linked lists v2.
use core::marker::PhantomData;
use core::ptr::NonNull;

//use super::SpecExtend;
use crate::prelude::*;

/// The `LinkedList` struct represents a doubly linked list. It holds optional pointers to the head and tail nodes of the list, the length of the list, and a marker of type `PhantomData`.
pub struct LinkedList<T> {
    head: Option<NonNull<Node<T>>>,
    tail: Option<NonNull<Node<T>>>,
    len: usize,
    marker: PhantomData<Box<Node<T>>>,
}

struct Node<T> {
    next: Option<NonNull<Node<T>>>,
    prev: Option<NonNull<Node<T>>>,
    element: T,
}

impl<T> Node<T> {
    fn new(element: T) -> Self {
        Node {
            next: None,
            prev: None,
            element,
        }
    }

    fn into_element(self: Box<Self>) -> T {
        self.element
    }
}

impl<T> Default for LinkedList<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> LinkedList<T> {
    /// The `new` method is a constructor for `LinkedList`. It creates a new `LinkedList` with `len` set to 0 and `head`, `tail`, and `marker` set to `None`.
    pub fn new() -> Self {
        Self {
            len: 0,
            head: None,
            tail: None,
            marker: PhantomData,
        }
    }

    /// The `len` method returns the length of the list.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Adds the given node to the front of the list.
    pub fn push_front(&mut self, element: T) {
        // Use box to help generate raw ptr
        let mut node = Box::try_new(Node::new(element)).unwrap();
        node.next = self.head;
        node.prev = None;
        let node = NonNull::new(Box::into_raw(node));

        match self.head {
            None => self.tail = node,
            Some(head) => unsafe { (*head.as_ptr()).prev = node },
        }

        self.head = node;
        self.len += 1;
    }

    /// Adds the given node to the back of the list.
    pub fn push_back(&mut self, element: T) {
        // Use box to help generate raw ptr
        let mut node = Box::try_new(Node::new(element)).unwrap();
        node.next = None;
        node.prev = self.tail;
        let node = NonNull::new(Box::into_raw(node));

        match self.tail {
            None => self.head = node,
            // Not creating new mutable (unique!) references overlapping `element`.
            Some(tail) => unsafe { (*tail.as_ptr()).next = node },
        }

        self.tail = node;
        self.len += 1;
    }

    /// Removes the first element and returns it, or `None` if the list is
    /// empty.
    ///
    /// This operation should compute in *O*(1) time.
    pub fn pop_front(&mut self) -> Option<T> {
        self.head.map(|node| {
            self.len -= 1;

            unsafe {
                let node = Box::from_raw(node.as_ptr());

                self.head = node.next;

                match self.head {
                    None => self.tail = None,
                    Some(head) => (*head.as_ptr()).prev = None,
                }
                node.into_element()
            }
        })
    }

    /// Removes the last element from a list and returns it, or `None` if
    /// it is empty.
    ///
    /// This operation should compute in *O*(1) time.
    pub fn pop_back(&mut self) -> Option<T> {
        self.tail.map(|node| {
            self.len -= 1;

            unsafe {
                let node = Box::from_raw(node.as_ptr());

                self.tail = node.prev;

                match self.tail {
                    None => self.head = None,
                    Some(tail) => (*tail.as_ptr()).next = None,
                }
                node.into_element()
            }
        })
    }

    /// Provides a reference to the front element, or `None` if the list is
    /// empty.
    ///
    /// This operation should compute in *O*(1) time.
    ///
    /// # Examples
    ///
    /// ```
    /// use collection::list::linked_list::LinkedList;
    ///
    /// let mut dl = LinkedList::new();
    /// assert_eq!(dl.peek_front(), None);
    ///
    /// dl.push_front(1);
    /// assert_eq!(dl.peek_front(), Some(&1));
    /// ```
    pub fn peek_front(&self) -> Option<&T> {
        unsafe { self.head.as_ref().map(|node| &node.as_ref().element) }
    }

    /// Provides a reference to the back element, or `None` if the list is
    /// empty.
    ///
    /// This operation should compute in *O*(1) time.
    pub fn peek_back(&self) -> Option<&T> {
        unsafe { self.tail.as_ref().map(|node| &node.as_ref().element) }
    }

    /// Provides a mutable reference to the front element, or `None` if the list
    /// is empty.
    ///
    /// This operation should compute in *O*(1) time.
    pub fn peek_front_mut(&mut self) -> Option<&mut T> {
        unsafe { self.head.as_mut().map(|node| &mut node.as_mut().element) }
    }

    /// Provides a mutable reference to the back element, or `None` if the list
    /// is empty.
    ///
    /// This operation should compute in *O*(1) time.
    pub fn peek_back_mut(&mut self) -> Option<&mut T> {
        unsafe { self.tail.as_mut().map(|node| &mut node.as_mut().element) }
    }
}
