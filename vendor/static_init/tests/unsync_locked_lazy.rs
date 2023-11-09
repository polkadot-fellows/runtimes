#![cfg(feature = "thread_local")]
#![feature(thread_local)]

use static_init::{dynamic, Phase, UnSyncLockedLazy};
use std::panic::catch_unwind;
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread::spawn;

#[dynamic(lazy)]
#[thread_local]
static mut NORMAL: Vec<i32> = vec![1, 2];
#[thread_local]
#[dynamic(lazy)]
static mut NORMAL1: Vec<i32> = vec![1, 2];
#[dynamic(lazy)]
#[thread_local]
static mut NORMAL2: Vec<i32> = vec![1, 2];
#[thread_local]
#[dynamic(lazy)]
static mut NORMAL3: Vec<i32> = vec![1, 2];
#[dynamic(lazy)]
#[thread_local]
static mut NORMAL4: Vec<i32> = vec![1, 2];

#[test]
fn normal() {
    assert!(NORMAL.phase().is_empty());

    assert!(NORMAL.try_read().is_err());

    assert!(NORMAL.try_write().is_err());

    assert!(NORMAL.fast_try_read().unwrap().is_err());

    assert!(NORMAL.fast_try_write().unwrap().is_err());

    assert!(NORMAL.phase().is_empty());

    assert_eq!(NORMAL.read().len(), 2);

    assert_eq!(NORMAL.write().len(), 2);

    assert_eq!(NORMAL.fast_read().unwrap().len(), 2);

    assert_eq!(NORMAL.fast_write().unwrap().len(), 2);

    assert_eq!(NORMAL1.write().len(), 2);

    assert_eq!(NORMAL2.fast_read().unwrap().len(), 2);

    assert_eq!(NORMAL3.fast_write().unwrap().len(), 2);

    NORMAL4.init();

    assert_eq!(NORMAL4.fast_try_read().unwrap().unwrap().len(), 2);

    assert!(NORMAL.phase() == Phase::INITIALIZED);

    assert!(NORMAL.try_read().unwrap().len() == 2);

    assert!(NORMAL.try_write().unwrap().len() == 2);

    assert!(NORMAL.fast_try_write().unwrap().unwrap().len() == 2);

    assert!(NORMAL.fast_try_read().unwrap().unwrap().len() == 2);

    assert!(NORMAL.phase() == Phase::INITIALIZED);

    assert_eq!(*NORMAL.read(), vec![1, 2]);

    NORMAL.write().push(3);

    assert_eq!(*NORMAL.read(), vec![1, 2, 3]);

    spawn(|| assert_eq!(*NORMAL.read(), vec![1, 2]))
        .join()
        .unwrap();
}

static UNINIT_COUNT: AtomicU32 = AtomicU32::new(0);

#[dynamic(lazy)]
#[thread_local]
static mut INIT_MAY_PANICK: Vec<i32> = {
    if UNINIT_COUNT.fetch_add(1, Ordering::Relaxed) < 2 {
        panic!("Should not be seen");
    }
    vec![1, 2]
};

#[test]
fn init_may_panick() {
    assert!(INIT_MAY_PANICK.phase().is_empty());

    assert!(INIT_MAY_PANICK.try_read().is_err());

    assert!(INIT_MAY_PANICK.phase().is_empty());

    assert!(catch_unwind(|| INIT_MAY_PANICK.write().len()).is_err());

    assert_eq!(UNINIT_COUNT.load(Ordering::Relaxed), 1);

    assert_eq!(
        INIT_MAY_PANICK.phase(),
        Phase::INITIALIZATION_PANICKED | Phase::INITIALIZATION_SKIPED
    );

    assert!(catch_unwind(|| INIT_MAY_PANICK.read().len()).is_err());

    assert_eq!(UNINIT_COUNT.load(Ordering::Relaxed), 2);

    assert_eq!(
        INIT_MAY_PANICK.phase(),
        Phase::INITIALIZATION_PANICKED | Phase::INITIALIZATION_SKIPED
    );

    assert_eq!(INIT_MAY_PANICK.read().len(), 2);

    assert_eq!(UNINIT_COUNT.load(Ordering::Relaxed), 3);

    assert!(INIT_MAY_PANICK.phase() == Phase::INITIALIZED);

    assert!(INIT_MAY_PANICK.try_read().unwrap().len() == 2);

    assert!(INIT_MAY_PANICK.phase() == Phase::INITIALIZED);

    assert_eq!(*INIT_MAY_PANICK.read(), vec![1, 2]);

    assert_eq!(*INIT_MAY_PANICK.write(), vec![1, 2]);
}

static UNINIT_ONCE_COUNT: AtomicU32 = AtomicU32::new(0);

#[dynamic(lazy, try_init_once)]
#[thread_local]
static mut UNINITIALIZABLE: Vec<i32> = {
    UNINIT_ONCE_COUNT.fetch_add(1, Ordering::Relaxed);
    panic!("Panicked on purpose")
};

#[test]
fn init_may_panick_intolerant() {
    assert!(UNINITIALIZABLE.phase().is_empty());

    assert!(UNINITIALIZABLE.try_read().is_err());

    assert!(UNINITIALIZABLE.phase().is_empty());

    assert!(catch_unwind(|| UNINITIALIZABLE.fast_read().unwrap().len()).is_err());

    assert_eq!(UNINIT_ONCE_COUNT.load(Ordering::Relaxed), 1);

    assert_eq!(
        UNINITIALIZABLE.phase(),
        Phase::INITIALIZATION_PANICKED | Phase::INITIALIZATION_SKIPED
    );

    assert!(catch_unwind(|| UNINITIALIZABLE.write().len()).is_err());

    assert_eq!(UNINIT_ONCE_COUNT.load(Ordering::Relaxed), 1);

    assert_eq!(
        UNINITIALIZABLE.phase(),
        Phase::INITIALIZATION_PANICKED | Phase::INITIALIZATION_SKIPED
    );

    assert_eq!(UNINIT_ONCE_COUNT.load(Ordering::Relaxed), 1);
}

#[dynamic(lazy, try_init_once)]
#[thread_local]
static mut NORMAL_WITH_TOLERANCE: Vec<i32> = vec![1, 2];

#[test]
fn normal_with_tolerance() {
    assert!(NORMAL_WITH_TOLERANCE.phase().is_empty());

    assert!(NORMAL_WITH_TOLERANCE.try_read().is_err());

    assert!(NORMAL_WITH_TOLERANCE.phase().is_empty());

    assert_eq!(NORMAL_WITH_TOLERANCE.read().len(), 2);

    assert!(NORMAL_WITH_TOLERANCE.phase() == Phase::INITIALIZED);

    assert_eq!(NORMAL_WITH_TOLERANCE.try_read().unwrap().len(), 2);

    assert!(NORMAL_WITH_TOLERANCE.phase() == Phase::INITIALIZED);

    assert_eq!(*NORMAL_WITH_TOLERANCE.write(), vec![1, 2]);

    assert_eq!(NORMAL_WITH_TOLERANCE.read().len(), 2);
}

#[test]
fn local_lazy() {
    let v = UnSyncLockedLazy::new(|| vec![1, 2]);

    assert!(v.phase().is_empty());

    assert!(v.try_read().is_err());

    assert!(v.phase().is_empty());

    assert_eq!(v.read().len(), 2);

    assert!(v.phase() == Phase::INITIALIZED);

    assert!(v.try_read().unwrap().len() == 2);

    assert!(v.phase() == Phase::INITIALIZED);

    assert_eq!(*v.read(), vec![1, 2]);

    assert_eq!(*v.fast_write().unwrap(), vec![1, 2]);

    let mut drop_count: i32 = 0;

    struct A<'a>(&'a mut i32);
    impl<'a> Drop for A<'a> {
        fn drop(&mut self) {
            *self.0 += 1;
        }
    }
    {
        UnSyncLockedLazy::new(|| A(&mut drop_count));
    }
    assert_eq!(drop_count, 0);
    {
        let v = UnSyncLockedLazy::new(|| A(&mut drop_count));
        v.init();
    }
    assert_eq!(drop_count, 1);
}

#[test]
fn local_lazy_mut() {
    let mut v = UnSyncLockedLazy::new(|| vec![1, 2]);

    assert!(v.phase().is_empty());

    assert!(v.try_read().is_err());

    assert!(v.try_get_mut().is_err());

    assert!(v.phase().is_empty());

    v.get_mut().push(3);

    assert_eq!(v.phase(), Phase::INITIALIZED);

    assert_eq!(v.try_read().unwrap().len(), 3);

    v.try_get_mut().unwrap().push(4);

    v.get_mut().push(5);

    assert_eq!(*v.get_mut(), vec![1, 2, 3, 4, 5]);
}
