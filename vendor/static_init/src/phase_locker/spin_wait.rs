use core::hint;
// Extracted from parking_lot_core
//
#[inline]
fn cpu_relax(iterations: u32) {
    for _ in 0..iterations {
        hint::spin_loop()
    }
}

/// A counter used to perform exponential backoff in spin loops.
#[derive(Default)]
pub(super) struct SpinWait {
    counter: u32,
}

impl SpinWait {
    /// Creates a new `SpinWait`.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Resets a `SpinWait` to its initial state.
    #[inline]
    #[cfg(not(feature = "spin_loop"))]
    pub fn reset(&mut self) {
        self.counter = 0;
    }

    /// Spins until the sleep threshold has been reached.
    ///
    /// This function returns whether the sleep threshold has been reached, at
    /// which point further spinning has diminishing returns and the thread
    /// should be parked instead.
    ///
    /// The spin strategy will initially use a CPU-bound loop but will fall back
    /// to yielding the CPU to the OS after a few iterations.
    #[inline]
    #[cfg(not(feature = "spin_loop"))]
    pub fn spin(&mut self) -> bool {
        if self.counter >= 10
        /*16*/
        {
            return false;
        }
        self.counter += 1;

        if self.counter <= 3
        /*4*/
        {
            cpu_relax(1 << self.counter);
        } else {
            yield_now();
        }
        true
    }

    /// Spins without yielding the thread to the OS.
    ///
    /// Instead, the backoff is simply capped at a maximum value. This can be
    /// used to improve throughput in `compare_exchange` loops that have high
    /// contention.
    #[inline]
    pub fn spin_no_yield(&mut self) -> bool {
        self.counter += 1;
        if self.counter > 10 {
            self.counter = 10;
        }
        cpu_relax(1 << self.counter);
        true
    }
}

#[cfg(all(
    not(feature = "parking_lot_core"),
    not(feature = "spin_loop"),
    any(target_os = "linux", target_os = "android")
))]
fn yield_now() {
    unsafe {
        libc::sched_yield();
    }
}
#[cfg(all(
    not(feature = "spin_loop"),
    not(all(
        not(feature = "parking_lot_core"),
        any(target_os = "linux", target_os = "android")
    ))
))]
use std::thread::yield_now;
