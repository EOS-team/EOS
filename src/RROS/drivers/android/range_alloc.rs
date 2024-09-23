// SPDX-License-Identifier: GPL-2.0

use core::ptr::NonNull;
use kernel::{
    linked_list::{CursorMut, GetLinks, Links, List},
    prelude::*,
};

pub(crate) struct RangeAllocator<T> {
    list: List<Box<Descriptor<T>>>,
}

#[derive(Debug, PartialEq, Eq)]
enum DescriptorState {
    Free,
    Reserved,
    Allocated,
}

impl<T> RangeAllocator<T> {
    pub(crate) fn new(size: usize) -> Result<Self> {
        let desc = Box::try_new(Descriptor::new(0, size))?;
        let mut list = List::new();
        list.push_back(desc);
        Ok(Self { list })
    }

    fn find_best_match(&self, size: usize) -> Option<NonNull<Descriptor<T>>> {
        // TODO: Use a binary tree instead of list for this lookup.
        let mut best = None;
        let mut best_size = usize::MAX;
        let mut cursor = self.list.cursor_front();
        while let Some(desc) = cursor.current() {
            if desc.state == DescriptorState::Free {
                if size == desc.size {
                    return Some(NonNull::from(desc));
                }

                if size < desc.size && desc.size < best_size {
                    best = Some(NonNull::from(desc));
                    best_size = desc.size;
                }
            }

            cursor.move_next();
        }
        best
    }

    pub(crate) fn reserve_new(&mut self, size: usize) -> Result<usize> {
        let desc_ptr = match self.find_best_match(size) {
            None => return Err(Error::ENOMEM),
            Some(found) => found,
        };

        // SAFETY: We hold the only mutable reference to list, so it cannot have changed.
        let desc = unsafe { &mut *desc_ptr.as_ptr() };
        if desc.size == size {
            desc.state = DescriptorState::Reserved;
            return Ok(desc.offset);
        }

        // We need to break up the descriptor.
        let new = Box::try_new(Descriptor::new(desc.offset + size, desc.size - size))?;
        unsafe { self.list.insert_after(desc_ptr, new) };
        desc.state = DescriptorState::Reserved;
        desc.size = size;
        Ok(desc.offset)
    }

    fn free_with_cursor(cursor: &mut CursorMut<'_, Box<Descriptor<T>>>) -> Result {
        let mut size = match cursor.current() {
            None => return Err(Error::EINVAL),
            Some(ref mut entry) => {
                match entry.state {
                    DescriptorState::Free => return Err(Error::EINVAL),
                    DescriptorState::Allocated => return Err(Error::EPERM),
                    DescriptorState::Reserved => {}
                }
                entry.state = DescriptorState::Free;
                entry.size
            }
        };

        // Try to merge with the next entry.
        if let Some(next) = cursor.peek_next() {
            if next.state == DescriptorState::Free {
                next.offset -= size;
                next.size += size;
                size = next.size;
                cursor.remove_current();
            }
        }

        // Try to merge with the previous entry.
        if let Some(prev) = cursor.peek_prev() {
            if prev.state == DescriptorState::Free {
                prev.size += size;
                cursor.remove_current();
            }
        }

        Ok(())
    }

    fn find_at_offset(&mut self, offset: usize) -> Option<CursorMut<'_, Box<Descriptor<T>>>> {
        let mut cursor = self.list.cursor_front_mut();
        while let Some(desc) = cursor.current() {
            if desc.offset == offset {
                return Some(cursor);
            }

            if desc.offset > offset {
                return None;
            }

            cursor.move_next();
        }
        None
    }

    pub(crate) fn reservation_abort(&mut self, offset: usize) -> Result {
        // TODO: The force case is currently O(n), but could be made O(1) with unsafe.
        let mut cursor = self.find_at_offset(offset).ok_or(Error::EINVAL)?;
        Self::free_with_cursor(&mut cursor)
    }

    pub(crate) fn reservation_commit(&mut self, offset: usize, data: Option<T>) -> Result {
        // TODO: This is currently O(n), make it O(1).
        let mut cursor = self.find_at_offset(offset).ok_or(Error::ENOENT)?;
        let desc = cursor.current().unwrap();
        desc.state = DescriptorState::Allocated;
        desc.data = data;
        Ok(())
    }

    /// Takes an entry at the given offset from [`DescriptorState::Allocated`] to
    /// [`DescriptorState::Reserved`].
    ///
    /// Returns the size of the existing entry and the data associated with it.
    pub(crate) fn reserve_existing(&mut self, offset: usize) -> Result<(usize, Option<T>)> {
        // TODO: This is currently O(n), make it O(log n).
        let mut cursor = self.find_at_offset(offset).ok_or(Error::ENOENT)?;
        let desc = cursor.current().unwrap();
        if desc.state != DescriptorState::Allocated {
            return Err(Error::ENOENT);
        }
        desc.state = DescriptorState::Reserved;
        Ok((desc.size, desc.data.take()))
    }

    pub(crate) fn for_each<F: Fn(usize, usize, Option<T>)>(&mut self, callback: F) {
        let mut cursor = self.list.cursor_front_mut();
        while let Some(desc) = cursor.current() {
            if desc.state == DescriptorState::Allocated {
                callback(desc.offset, desc.size, desc.data.take());
            }

            cursor.move_next();
        }
    }
}

struct Descriptor<T> {
    state: DescriptorState,
    size: usize,
    offset: usize,
    links: Links<Descriptor<T>>,
    data: Option<T>,
}

impl<T> Descriptor<T> {
    fn new(offset: usize, size: usize) -> Self {
        Self {
            size,
            offset,
            state: DescriptorState::Free,
            links: Links::new(),
            data: None,
        }
    }
}

impl<T> GetLinks for Descriptor<T> {
    type EntryType = Self;
    fn get_links(desc: &Self) -> &Links<Self> {
        &desc.links
    }
}
