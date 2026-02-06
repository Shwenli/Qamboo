
use std::time::Instant;

pub struct DebugTimer {
    start: Instant,
    last: Instant,
}

impl DebugTimer {
    pub fn new() -> Self {
        let now = Instant::now();
        Self { start: now, last: now }
    }

    // 打点方法：输出当前步骤耗时和总耗时
    pub fn mark(&mut self, msg: &str) {
        let now = Instant::now();
        tracing::info!("⏱️ [{}]: Lap {:?}, Total {:?}", msg, now.duration_since(self.last), now.duration_since(self.start));
        self.last = now;
    }
}