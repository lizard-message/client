use smol::Timer;
use std::time::{Duration, Instant};
use std::u64::MAX;

#[derive(Debug)]
pub(super) struct Intval {
    count: u8,
    init_intval: u64,
}

impl Intval {
    pub(super) fn new(init_intval: u64) -> Self {
        Self {
            count: 0,
            init_intval: init_intval,
        }
    }

    // 推迟执行时间
    pub(super) fn delay(&mut self) {
        if self.count < 5 {
            self.count += 1;
        }
    }

    // 重置执行
    pub(super) fn reset(&mut self) {
        self.count = 0;
    }

    pub(super) async fn run(&mut self) -> Instant {
        Timer::at(Instant::now() + Duration::from_secs(self.init_intval + (self.count * 10) as u64))
            .await
    }
}
