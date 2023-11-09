use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time;

use super::instant::*;

/// A service to periodically call `Instant::update()`
#[derive(Debug)]
pub struct Updater {
    period: time::Duration,
    running: Arc<AtomicBool>,
    th: Option<thread::JoinHandle<()>>,
}

impl Updater {
    /// Spawns a background task to call `Instant::update()` periodically
    pub fn start(mut self) -> Result<Self, io::Error> {
        let period = self.period;
        let running = self.running.clone();
        running.store(true, Ordering::Relaxed);
        let th: thread::JoinHandle<()> = thread::Builder::new()
            .name("coarsetime".to_string())
            .spawn(move || {
                while running.load(Ordering::Relaxed) {
                    thread::sleep(period);
                    Instant::update();
                }
            })?;
        self.th = Some(th);
        Instant::update();
        Ok(self)
    }

    /// Stops the periodic updates
    pub fn stop(mut self) -> Result<(), io::Error> {
        self.running.store(false, Ordering::Relaxed);
        self.th
            .take()
            .expect("updater is not running")
            .join()
            .map_err(|_| {
                io::Error::new(io::ErrorKind::Other, "failed to properly stop the updater")
            })
    }

    /// Creates a new `Updater` with the specified update period, in
    /// milliseconds.
    pub fn new(period_millis: u64) -> Updater {
        Updater {
            period: time::Duration::from_millis(period_millis),
            running: Arc::new(AtomicBool::new(false)),
            th: None,
        }
    }
}
