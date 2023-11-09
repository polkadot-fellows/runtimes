[![Documentation](https://docs.rs/coarsetime/badge.svg)](https://docs.rs/coarsetime)
[![Build Status](https://travis-ci.org/jedisct1/rust-coarsetime.svg?branch=master)](https://travis-ci.org/jedisct1/rust-coarsetime?branch=master)
[![Windows build status](https://ci.appveyor.com/api/projects/status/xlbhk9850dvl5ylh?svg=true)](https://ci.appveyor.com/project/jedisct1/rust-coarsetime)
# coarsetime

A Rust crate to make time measurements, that focuses on speed.

This crate is a partial replacement for the `Time` and `Duration` structures
from the standard library, with the following differences:

* Speed is privileged over accuracy. In particular, `CLOCK_MONOTONIC_COARSE` is
used to retrieve the clock value on Linux systems, and transformations avoid
operations that can be slow on non-Intel systems.
* The number of system calls can be kept to a minimum. The "most recent
timestamp" is always kept in memory. It can be read with just a load operation,
and can be updated only as frequently as necessary.

# Installation

`coarsetime` is available on [crates.io](https://crates.io/crates/coarsetime)
and works on Rust stable, beta, and nightly.

Windows and Unix-like systems are supported.

Available feature:

* `nightly`: rust compile is rust-nightly - This is required to run
benchmarks.

# Documentation

[API documentation](https://docs.rs/coarsetime)

# Example

```rust
extern crate coarsetime;

use coarsetime::{Duration, Instant, Updater};

// Get the current instant. This may require a system call, but it may also
// be faster than the stdlib equivalent.
let now = Instant::now();

// Get the latest known instant. This operation is super fast.
// In this case, the value will be identical to `now`, because we haven't
// updated the latest known instant yet.
let ts1 = Instant::recent();

// Update the latest known instant. This may require a system call.
// Note that a call to `Instant::now()` also updates the stored instant.
Instant::update();

// Now, we may get a different instant. This call is also super fast.
let ts2 = Instant::recent();

// Compute the time elapsed between ts2 and ts1.
let elapsed_ts2_ts1 = ts2.duration_since(ts1);

// Operations such as `+` and `-` between `Instant` and `Duration` are also
// available.
let elapsed_ts2_ts1 = ts2 - ts1;

// Returns the time elapsed since ts1.
// This retrieves the actual current time, and may require a system call.
let elapsed_since_ts1 = ts1.elapsed();

// Returns the approximate time elapsed since ts1.
// This uses the latest known instant, and is super fast.
let elapsed_since_recent = ts1.elapsed_since_recent();

// Instant::update() should be called periodically, for example using an
// event loop. Alternatively, the crate provides an easy way to spawn a
// background task that will periodically update the latest known instant.
// Here, the update will happen every 250ms.
let updater = Updater::new(250).start().unwrap();

// From now on, Instant::recent() will always return an approximation of the
// current instant.
let ts3 = Instant::recent();

// Stop the task.
updater.stop().unwrap();

// Returns the elapsed time since the UNIX epoch
let unix_timestamp = Clock::now_since_epoch();

// Returns an approximation of the elapsed time since the UNIX epoch, based on
// the latest time update
let unix_timestamp_approx = Clock::recent_since_epoch();
```
