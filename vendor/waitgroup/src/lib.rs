//! A WaitGroup waits for a collection of task to finish.
//!
//! ## Examples
//! ```rust
//! use waitgroup::WaitGroup;
//! use async_std::task;
//! # task::block_on(
//! async {
//!     let wg = WaitGroup::new();
//!     for _ in 0..100 {
//!         let w = wg.worker();
//!         task::spawn(async move {
//!             // do work
//!             drop(w); // drop w means task finished
//!         });
//!     }
//!
//!     wg.wait().await;
//! }
//! # );
//! ```
//!

use atomic_waker::AtomicWaker;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Weak};
use std::task::{Context, Poll};

pub struct WaitGroup {
    inner: Arc<Inner>,
}

#[derive(Clone)]
pub struct Worker {
    inner: Arc<Inner>,
}

pub struct WaitGroupFuture {
    inner: Weak<Inner>,
}

struct Inner {
    waker: AtomicWaker,
}

impl Drop for Inner {
    fn drop(&mut self) {
        self.waker.wake();
    }
}

impl WaitGroup {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                waker: AtomicWaker::new(),
            }),
        }
    }

    pub fn worker(&self) -> Worker {
        Worker {
            inner: self.inner.clone(),
        }
    }

    pub fn wait(self) -> WaitGroupFuture {
        WaitGroupFuture {
            inner: Arc::downgrade(&self.inner),
        }
    }
}

impl Default for WaitGroup {
    fn default() -> Self {
        Self::new()
    }
}

/*
IntoFuture tracking issue: https://github.com/rust-lang/rust/issues/67644
impl IntoFuture for WaitGroup {
    type Output = ();
    type Future = WaitGroupFuture;

    fn into_future(self) -> Self::Future {
        WaitGroupFuture { inner: Arc::downgrade(&self.inner) }
    }
}
*/

impl Future for WaitGroupFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.inner.upgrade() {
            Some(inner) => {
                inner.waker.register(cx.waker());
                Poll::Pending
            }
            None => Poll::Ready(()),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use async_std::task;

    #[async_std::test]
    async fn smoke() {
        let wg = WaitGroup::new();

        for _ in 0..100 {
            let w = wg.worker();
            task::spawn(async move {
                drop(w);
            });
        }

        wg.wait().await;
    }
}
