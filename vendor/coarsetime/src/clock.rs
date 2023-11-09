#[cfg(not(all(
    any(target_arch = "wasm32", target_arch = "wasm64"),
    target_os = "unknown"
)))]
use std::time;

use once_cell::sync::Lazy;
#[cfg(all(
    any(target_arch = "wasm32", target_arch = "wasm64"),
    target_os = "unknown"
))]
use wasm_bindgen::prelude::*;

use super::{Duration, Instant};

#[cfg(all(
    any(target_arch = "wasm32", target_arch = "wasm64"),
    target_os = "unknown"
))]
#[wasm_bindgen]
extern "C" {
    type Date;

    #[wasm_bindgen(static_method_of = Date)]
    pub fn now() -> f64;
}

/// System time
#[derive(Debug)]
pub struct Clock;

/// Alias for `Duration`.
pub type UnixTimeStamp = Duration;

static CLOCK_OFFSET: Lazy<u64> = Lazy::new(clock_offset);

impl Clock {
    /// Returns the elapsed time since the UNIX epoch
    #[inline]
    pub fn now_since_epoch() -> UnixTimeStamp {
        let offset = *CLOCK_OFFSET;
        let unix_ts_now = Instant::now().as_u64().wrapping_sub(offset);
        Duration::from_u64(unix_ts_now)
    }

    /// Returns the elapsed time since the UNIX epoch, based on the latest
    /// explicit time update
    #[inline]
    pub fn recent_since_epoch() -> UnixTimeStamp {
        let offset = *CLOCK_OFFSET;
        let unix_ts_now = Instant::recent().as_u64().wrapping_sub(offset);
        Duration::from_u64(unix_ts_now)
    }

    /// Updates the system time - This is completely equivalent to calling
    /// Instant::update()
    #[inline]
    pub fn update() {
        Instant::update()
    }
}

#[cfg(all(
    any(target_arch = "wasm32", target_arch = "wasm64"),
    target_os = "unknown"
))]
#[inline]
fn unix_ts() -> u64 {
    let unix_ts_now_sys = (Date::now() / 1000.0).round() as u64;
    let unix_ts_now = Duration::from_secs(unix_ts_now_sys);
    unix_ts_now.as_u64()
}

#[cfg(not(all(
    any(target_arch = "wasm32", target_arch = "wasm64"),
    target_os = "unknown"
)))]
#[inline]
fn unix_ts() -> u64 {
    let unix_ts_now_sys = time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .expect("The system clock is not properly set");
    let unix_ts_now = Duration::from(unix_ts_now_sys);
    unix_ts_now.as_u64()
}

fn clock_offset() -> u64 {
    let unix_ts_now = unix_ts();
    let instant_now = Instant::now().as_u64();
    instant_now.wrapping_sub(unix_ts_now)
}
