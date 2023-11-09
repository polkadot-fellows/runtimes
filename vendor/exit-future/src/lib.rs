use std::pin::Pin;
use std::task::{Poll, Context};
use futures::{Future, FutureExt, channel::oneshot, future::{select, Either, Shared, FusedFuture}, executor::block_on};

/// Future that resolves when the exit signal has fired.
#[derive(Clone)]
pub struct Exit(Shared<oneshot::Receiver<()>>);

impl Future for Exit {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let receiver = &mut Pin::into_inner(self).0;

        if receiver.is_terminated() {
            Poll::Ready(())
        } else {
            Pin::new(receiver).poll(cx).map(drop)
        }
    }
}

impl Exit {
     /// Perform given work until complete.
    pub fn until<F: Future + Unpin>(self, future: F) -> impl Future<Output = Option<F::Output>> {
        select(self, future)
            .map(|either| match either {
                Either::Left(_) => None,
                Either::Right((output, _)) => Some(output)
            })
    }

    /// Block the current thread until complete.
    pub fn wait(self) {
        block_on(self)
    }
}

/// Exit signal that fires either manually or on drop.
pub struct Signal(oneshot::Sender<()>);

impl Signal {
    /// Fire the signal manually.
    pub fn fire(self) -> Result<(), ()> {
        self.0.send(())
    }
}

/// Create a signal and exit pair. `Exit` is a future that resolves when the `Signal` object is
/// either dropped or has `fire` called on it.
pub fn signal() -> (Signal, Exit) {
    let (sender, receiver) = oneshot::channel();
    (Signal(sender), Exit(receiver.shared()))
}

#[cfg(test)]
mod tests {
    use futures::future::{join3, ready, pending, lazy};
    use std::thread::{spawn, sleep};
    use std::time::Duration;
    use std::sync::Arc;
    use super::*;

    #[test]
    fn it_works() {
        let (signal, exit_a) = signal();
        let exit_b = exit_a.clone();
        let exit_c = exit_b.clone();

        let barrier = Arc::new(::std::sync::Barrier::new(2));
        let thread_barrier = barrier.clone();
        let handle = spawn(move || {
            let barrier = ::futures::future::lazy(move |_| {
                thread_barrier.wait();
            });

            block_on(join3(exit_a, exit_b, barrier));
        });

        barrier.wait();
        signal.fire().unwrap();

        let _ = handle.join();
        exit_c.wait()
    }

    #[test]
    fn drop_signal() {
        let (signal, exit) = signal();

        let thread = spawn(move || {
            sleep(Duration::from_secs(1));
            drop(signal)
        });

        thread.join().unwrap();
        exit.wait()
    }

    #[test]
    fn many_exit_signals() {
        let mut handles = Vec::new();
        let (signal, exit) = signal();

        for _ in 0 .. 100 {
            let exit = exit.clone();
            handles.push(spawn(move || {
                sleep(Duration::from_secs(1));
                exit.wait();
            }));
        }

        signal.fire().unwrap();

        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn exit_signal_are_send_and_sync() {
        fn is_send_and_sync<T: Send + Sync>() {}

        is_send_and_sync::<Exit>();
        is_send_and_sync::<Signal>();
    }

    #[test]
    fn work_until() {
        let (signal, exit) = signal();
        let work_a = exit.clone().until(ready(5));
        assert_eq!(block_on(work_a), Some(5));

        signal.fire().unwrap();
        let work_b = exit.until(pending::<()>());
        assert_eq!(block_on(work_b), None);
    }

    #[test]
    fn works_from_other_thread() {
        let (signal, exit) = signal();

        ::std::thread::spawn(move || {
            ::std::thread::sleep(::std::time::Duration::from_millis(2500));
            signal.fire().unwrap();
        });

        block_on(exit);
    }

    #[test]
    fn clone_works() {
        let (_signal, mut exit) = signal();

        let future = lazy(move |cx| {
            let _ = Pin::new(&mut exit).poll(cx);

            let mut exit2 = exit.clone();
            let _ = Pin::new(&mut exit2).poll(cx);
        });

        block_on(future)
    }
}
