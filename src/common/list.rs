use std::sync::{Arc, Mutex};

use crate::features::stack::StackLike;

#[derive(Debug, Clone)]
pub struct SharedVec<T> {
    inner: Arc<Mutex<Vec<T>>>,
}

impl<T> SharedVec<T> {
    pub fn new(list: Vec<T>) -> SharedVec<T> {
        SharedVec {
            inner: Arc::new(Mutex::new(list)),
        }
    }
}

impl<T> StackLike<T> for SharedVec<T> {
    fn push(&mut self, item: T) {
        self.inner.lock().unwrap().push(item)
    }

    fn pop(&mut self) -> Option<T> {
        self.inner.lock().unwrap().pop()
    }

    fn len(&self) -> usize {
        self.inner.lock().unwrap().len()
    }
}
