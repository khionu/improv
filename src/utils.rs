use std::time::Instant;
use std::sync::atomic::{AtomicU16, Ordering};

pub struct SnowflakeProducer {
    epoch: Instant,
    increment: AtomicU16,
}

impl SnowflakeProducer {
    pub fn produce(&self) -> u64 {
        let inc = self.increment.fetch_add(1, Ordering::Acquire);
        let dur = Instant::now().duration_since(self.epoch).as_millis() as u64;

        ((dur << 16) | inc as u64)
    }
}

impl Default for SnowflakeProducer {
    fn default() -> Self {
        Self {
            epoch: Instant::now(),
            increment: Default::default(),
        }
    }
}
