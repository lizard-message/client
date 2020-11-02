use smol::Timer;
use std::time::{Duration, Instant};
use smol::Task;
use smol::spawn;
use std::mem::replace;

#[derive(Debug)]
pub(super) struct Intval {
    count: u8,
    init_intval: u64,
    interval_task: Task<Instant>,
    timeout_task: Task<Instant>,
}

impl Intval {
    pub(super) fn new(init_intval: u64) -> Self {
        Self {
            count: 0,
            init_intval: init_intval,
            interval_task: spawn(Timer::at(Instant::now())),
            timeout_task: spawn(Timer::at(Instant::now())),
        }
    }

    // 推迟执行时间
    pub(super) fn delay(&mut self) {
        if self.count < 5 {
            self.count += 1;
        }
    }

    // 重置执行
    pub(super) async fn reset(&mut self) {
        self.count = 0;
        
        let task = replace(&mut self.interval_task, spawn(Timer::at(Instant::now())));

        task.cancel().await;
    }


    pub(super) async fn run(&mut self) -> Instant {
        Timer::at(Instant::now() + Duration::from_secs(self.init_intval + (self.count * 10) as u64))
            .await
    }
}
