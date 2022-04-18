use std::time::Instant;

/// `RateStatus` records the rate of adding number
pub struct RateStatus {
    /// Total number
    total: u64,

    /// The number at an one tick interval
    count: u64,

    /// The interval of one tick
    tick: Instant,
}

impl RateStatus {
    pub fn new() -> RateStatus {
        RateStatus::default()
    }

    pub fn total(&self) -> u64 {
        self.total
    }

    pub fn set_total(&mut self, total: u64) {
        self.total = total;
    }

    pub fn count(&self) -> u64 {
        self.count
    }

    pub fn rate(&self) -> f64 {
        let interval = self.tick.elapsed().as_secs_f64();
        self.count as f64 / interval
    }

    pub fn add(&mut self, incr: u64) {
        self.total += incr;
        self.count += incr;
    }

    pub fn reset(&mut self) {
        self.total = 0;
        self.count = 0;
        self.tick = Instant::now();
    }

    pub fn clean(&mut self) {
        self.count = 0;
        self.tick = Instant::now();
    }
}

impl Default for RateStatus {
    fn default() -> RateStatus {
        RateStatus {
            total: 0,
            count: 0,
            tick: Instant::now(),
        }
    }
}
