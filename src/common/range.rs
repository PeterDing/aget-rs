use std::{cell::RefCell, rc::Rc};

use crate::features::stack::StackLike;

#[derive(Debug, Clone, Copy)]
pub struct RangePair {
    pub begin: u64,
    pub end: u64,
}

impl RangePair {
    pub fn new(begin: u64, end: u64) -> RangePair {
        RangePair { begin, end }
    }

    // The length of a `RangePair` is the closed interval length
    pub fn length(&self) -> u64 {
        self.end - self.begin + 1
    }
}

pub type RangeList = Vec<RangePair>;

#[derive(Debug, Clone)]
pub struct SharedRangList {
    inner: Rc<RefCell<RangeList>>,
}

impl SharedRangList {
    pub fn new(rangelist: RangeList) -> SharedRangList {
        SharedRangList {
            inner: Rc::new(RefCell::new(rangelist)),
        }
    }
}

impl StackLike<RangePair> for SharedRangList {
    fn push(&mut self, pair: RangePair) {
        self.inner.borrow_mut().push(pair)
    }

    fn pop(&mut self) -> Option<RangePair> {
        self.inner.borrow_mut().pop()
    }

    fn len(&self) -> usize {
        self.inner.borrow().len()
    }
}

/// Split a close `RangePair` to many piece of pairs that each of their size is equal to
/// `chunk_size`, but the last piece size can be less then `chunk_size`.
pub fn split_pair(pair: &RangePair, chunk_size: u64) -> RangeList {
    let mut stack = Vec::new();

    let mut begin = pair.begin;
    let interval_end = pair.end;

    while begin + chunk_size - 1 <= interval_end {
        let end = begin + chunk_size - 1;
        stack.push(RangePair::new(begin, end));
        begin += chunk_size;
    }

    if begin <= interval_end {
        stack.push(RangePair::new(begin, interval_end));
    }

    stack
}
