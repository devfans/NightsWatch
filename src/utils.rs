use std::time::{SystemTime, UNIX_EPOCH};

#[allow(dead_code)]
#[inline]
pub fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}
