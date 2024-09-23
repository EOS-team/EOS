// SPDX-License-Identifier: GPL-2.0

//! Double Linked lists.
use alloc::boxed::Box;
use core::ptr::NonNull;

#[derive(Debug, Clone, Copy)]
/// The `Node` struct represents a node in a doubly linked list. It holds a value of generic type `T` and optional pointers to the next and previous nodes in the list.
pub struct Node<T> {
    /// `next`: An `Option` that contains a `NonNull` pointer to the next `Node` in the list. If there is no next node, `next` is `None`.
    pub next: Option<NonNull<Node<T>>>,
    /// `prev`: An `Option` that contains a `NonNull` pointer to the previous `Node` in the list. If there is no previous node, `prev` is `None`.
    pub prev: Option<NonNull<Node<T>>>,
    /// `value`: The value of the node. It is of generic type `T`.
    pub value: T,
}

impl<T> Node<T> {
    /// The `new` method is a constructor for `Node`. It takes a value of type `T` and creates a new `Node` with that value and with `next` and `prev` set to `None`.
    pub fn new(v: T) -> Self {
        Node {
            next: None,
            prev: None,
            value: v,
        }
    }

    //self---n---next
    /// The `add` method adds a new node after the current node. It takes a mutable pointer to the next `Node` and a value of type `T` for the new node. It creates a new `Node` with the provided value, sets its `next` and `prev` fields to point to the appropriate nodes, and updates the `next` and `prev` fields of the surrounding nodes to point to the new node.
    pub fn add(&mut self, next: *mut Node<T>, n: T) {
        // Box::try_new_in(123, RrosMem);
        // let mut node = Box::try_new_in(Node::new(n), RrosMem).unwrap();
        let mut node = Box::try_new(Node::new(n)).unwrap();
        node.next = NonNull::new(next);
        node.prev = NonNull::new(self as *mut Node<T>);
        let node = NonNull::new(Box::into_raw(node));

        self.next = node;
        unsafe {
            (*next).prev = node;
        }
    }

    /// The `remove` method removes the current node from the list. It checks if both `next` and `prev` are `Some`, and if so, it updates the `next` and `prev` fields of the surrounding nodes to skip over the current node. The actual removal of the node is not shown in the provided code.
    pub fn remove(&mut self) {
        if self.next.is_some() && self.prev.is_some() {
            unsafe {
                let next = self.next.unwrap().as_ptr();
                let prev = self.prev.unwrap().as_ptr();
                if next == prev {
                    // Handle the case where there is only one element in the list.
                    (*next).prev = None;
                    (*next).next = None;
                } else {
                    (*next).prev = self.prev;
                    (*prev).next = self.next;
                }
            }
        }
        unsafe {
            Box::from_raw(self as *mut Node<T>);
        }
    }

    /// The `into_val` method consumes the `Node` and returns its value. It takes the `Node` by value (consuming it) and returns the `value` field.
    pub fn into_val(self: Box<Self>) -> T {
        self.value
    }
}

#[derive(Clone, Copy)]
/// The `List` struct represents a doubly linked list. It holds a `Node` which is the head of the list.
pub struct List<T> {
    /// `head`: The head node of the list. It is of type `Node<T>`.
    pub head: Node<T>,
}

impl<T> List<T> {
    /// The `new` method is a constructor for `List`. It takes a value of type `T` and creates a new `List` with a head `Node` that holds the provided value.
    pub fn new(v: T) -> Self {
        List { head: Node::new(v) }
    }

    /// The `add_head` method adds a new node at the head of the list. It takes a value of type `T` for the new node. If the list is empty, it adds the new node after the head. Otherwise, it adds the new node after the node currently following the head.
    pub fn add_head(&mut self, v: T) {
        if self.is_empty() {
            let x = &mut self.head as *mut Node<T>;
            self.head.add(x, v);
        } else {
            self.head.add(self.head.next.unwrap().as_ptr(), v);
        }
    }

    /// The `add_tail` method adds a new node at the tail of the list. It takes a value of type `T` for the new node. If the list is empty, it adds the new node after the head. Otherwise, it adds the new node after the last node in the list. After adding the new node, it prints the new length of the list.
    pub fn add_tail(&mut self, v: T) {
        if self.is_empty() {
            let x = &mut self.head as *mut Node<T>;
            self.head.add(x, v);
        } else {
            unsafe {
                let prev = self.head.prev.unwrap().as_mut();
                prev.add(&mut self.head as *mut Node<T>, v);
            }
        }
        // pr_info!("after add tail, the length is {}", self.len());
    }

    /// The `get_head` method returns a mutable reference to the first node in the list. If the list is empty, it returns `None`. Otherwise, it returns the node following the head.
    pub fn get_head<'a>(&self) -> Option<&'a mut Node<T>> {
        if self.is_empty() {
            return None;
        } else {
            Some(unsafe { self.head.next.unwrap().as_mut() })
        }
    }

    /// The `get_tail` method returns a mutable reference to the last node in the list. If the list is empty, it returns `None`. Otherwise, it returns the node preceding the head.
    pub fn get_tail<'a>(&self) -> Option<&'a mut Node<T>> {
        if self.is_empty() {
            return None;
        } else {
            Some(unsafe { self.head.prev.unwrap().as_mut() })
        }
    }

    /// The `get_by_index` method returns a mutable reference to the node at a specific index in the list. It takes an index of type `u32`. The implementation of this method is not shown in the provided code.
    pub fn get_by_index<'a>(&mut self, index: u32) -> Option<&'a mut Node<T>> {
        if index <= self.len() {
            let mut p = self.head.next;
            for _ in 1..index {
                p = unsafe { p.unwrap().as_ref().next };
            }
            return Some(unsafe { p.unwrap().as_mut() });
        } else {
            return None;
        }
    }

    /// The `enqueue_by_index` method inserts a new node at a specific index in the list. It takes an index of type `u32` and a value of type `T` for the new node. If the index is 0, it adds the new node at the head of the list. If the index is equal to the length of the list, it adds the new node at the tail of the list. Otherwise, it adds the new node after the node at the specified index.
    pub fn enqueue_by_index(&mut self, index: u32, v: T) {
        if index <= self.len() {
            if index == 0 {
                self.add_head(v);
            } else if index == self.len() {
                self.add_tail(v);
            } else {
                let x = self.get_by_index(index).unwrap();
                let next = self.get_by_index(index + 1).unwrap();
                x.add(next as *mut Node<T>, v);
            }
        }
    }

    /// The `dequeue` method removes the node at a specific index from the list. It takes an index of type `u32`. If the list has only one node and the index is 1, it removes the head node. Otherwise, if the index is less than or equal to the length of the list, it removes the node at the specified index.
    pub fn dequeue(&mut self, index: u32) {
        if self.len() == 1 && index == 1 {
            unsafe {
                Box::from_raw(self.head.next.as_mut().unwrap().as_ptr() as *mut Node<T>);
            }
            self.head.next = None;
            self.head.prev = None;
        } else if index <= self.len() {
            self.get_by_index(index).unwrap().remove();
        }
    }

    /// The `de_head` method removes the head node from the list. It does this by calling the `dequeue` method with an index of 1.
    pub fn de_head(&mut self) {
        self.dequeue(1);
    }

    /// The `de_tail` method removes the tail node from the list. It does this by calling the `dequeue` method with an index equal to the length of the list.
    pub fn de_tail(&mut self) {
        self.dequeue(self.len());
    }

    /// The `len` method calculates and returns the length of the list. If the list is not empty, it initializes a counter to 1 and then traverses the list, incrementing the counter for each node until it reaches the node preceding the head. If the list is empty, it returns 0.
    pub fn len(&self) -> u32 {
        let mut ans = 0;
        if !self.is_empty() {
            ans = 1;
            let mut p = self.head.next;
            while p.unwrap().as_ptr() != self.head.prev.unwrap().as_ptr() {
                ans = ans + 1;
                unsafe {
                    p = p.unwrap().as_ref().next;
                }
            }
        }
        ans
    }

    /// The `is_empty` method checks if the list is empty. It does this by checking if both the `next` and `prev` fields of the head node are `None`. If they are, the list is empty and the method returns `true`. Otherwise, it returns `false`.
    pub fn is_empty(&self) -> bool {
        self.head.next.is_none() && self.head.prev.is_none()
    }
}

impl<T: core::fmt::Display> List<T> {
    /// The `traverse` method is used for testing. It traverses the list and prints each node. If the list is empty, it returns immediately. Otherwise, it starts from the node following the head and continues until it reaches the node preceding the head.
    pub fn traverse(&self) {
        if self.is_empty() {
            return;
        }
        let mut p = self.head.next;
        while p.unwrap().as_ptr() != self.head.prev.unwrap().as_ptr() {
            unsafe {
                p = p.unwrap().as_ref().next;
            }
        }
    }
}
