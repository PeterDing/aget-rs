use std::{cell::RefCell, rc::Rc};

use crate::features::stack::StackLike;

#[derive(Debug, Clone)]
pub struct SharedVec<T> {
    inner: Rc<RefCell<Vec<T>>>,
}

impl<T> SharedVec<T> {
    pub fn new(list: Vec<T>) -> SharedVec<T> {
        SharedVec {
            inner: Rc::new(RefCell::new(list)),
        }
    }
}

impl<T> StackLike<T> for SharedVec<T> {
    fn push(&mut self, item: T) {
        self.inner.borrow_mut().push(item)
    }

    fn pop(&mut self) -> Option<T> {
        self.inner.borrow_mut().pop()
    }

    fn len(&self) -> usize {
        self.inner.borrow().len()
    }
}
