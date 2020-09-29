use smol::Timer;
use std::time::{Instant, Duration};

#[derive(Debug)]
pub(super) struct Intval {
    count: u8,
    timer: Timer,
    init_intval: u64,
}

impl Intval {
    pub(super) fn new(init_intval: u64) -> Self {
        Self {
            count: 0,
            timer: Timer::at(Instant::now() + Duration::from_secs(init_intval)),
            init_intval: init_intval,
        }
    }
}
