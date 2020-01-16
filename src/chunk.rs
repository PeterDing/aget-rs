use std::{cell::RefCell, rc::Rc};

#[derive(Debug, Clone)]
pub struct RangePart {
    pub start: u64,
    pub end: u64,
}

impl RangePart {
    pub fn new(start: u64, end: u64) -> RangePart {
        RangePart { start, end }
    }

    pub fn length(&self) -> u64 {
        self.end - self.start + 1
    }
}

pub type RangeStack = Rc<RefCell<Vec<RangePart>>>;

/// Split a close `interval` to many piece chunk that its size is equal to `chunk_length`,
/// but the last piece size can be less then `chunk_length`.
pub fn make_range_chunks(interval: &RangePart, chunk_length: u64) -> Vec<RangePart> {
    let mut stack = Vec::new();

    let mut start = interval.start;
    let interval_end = interval.end;

    while start + chunk_length - 1 <= interval_end {
        let end = start + chunk_length - 1;
        stack.push(RangePart::new(start, end));
        start += chunk_length;
    }

    if start <= interval_end {
        stack.push(RangePart::new(start, interval_end));
    }

    stack
}
