use static_init::{dynamic, Lazy, LazyAccess, Phase};
use std::panic::catch_unwind;
use std::sync::atomic::{AtomicU32, Ordering};

#[dynamic(lazy)]
static NORMAL: Vec<i32> = vec![1, 2];

#[test]
fn normal() {
    assert!(LazyAccess::phase(&NORMAL).is_empty());

    assert!(LazyAccess::try_get(&NORMAL).is_err());

    assert!(LazyAccess::phase(&NORMAL).is_empty());

    assert_eq!(NORMAL.len(), 2);

    assert!(LazyAccess::phase(&NORMAL) == Phase::INITIALIZED);

    assert!(LazyAccess::try_get(&NORMAL).unwrap().len() == 2);

    assert!(LazyAccess::phase(&NORMAL) == Phase::INITIALIZED);

    assert_eq!(*NORMAL, vec![1, 2]);

    assert_eq!(*LazyAccess::get(&NORMAL), vec![1, 2]);
}

static UNINIT_COUNT: AtomicU32 = AtomicU32::new(0);
#[dynamic(lazy)]
static INIT_MAY_PANICK: Vec<i32> = {
    if UNINIT_COUNT.fetch_add(1, Ordering::Relaxed) < 2 {
        panic!("Should not be seen");
    }
    vec![1, 2]
};

#[test]
fn init_may_panick() {
    assert!(LazyAccess::phase(&INIT_MAY_PANICK).is_empty());

    assert!(LazyAccess::try_get(&INIT_MAY_PANICK).is_err());

    assert!(LazyAccess::phase(&INIT_MAY_PANICK).is_empty());

    assert!(catch_unwind(|| INIT_MAY_PANICK.len()).is_err());

    assert_eq!(UNINIT_COUNT.load(Ordering::Relaxed), 1);

    assert_eq!(
        LazyAccess::phase(&INIT_MAY_PANICK),
        Phase::INITIALIZATION_PANICKED | Phase::INITIALIZATION_SKIPED
    );

    assert!(catch_unwind(|| INIT_MAY_PANICK.len()).is_err());

    assert_eq!(UNINIT_COUNT.load(Ordering::Relaxed), 2);

    assert_eq!(
        LazyAccess::phase(&INIT_MAY_PANICK),
        Phase::INITIALIZATION_PANICKED | Phase::INITIALIZATION_SKIPED
    );

    assert_eq!(INIT_MAY_PANICK.len(), 2);

    assert_eq!(UNINIT_COUNT.load(Ordering::Relaxed), 3);

    assert!(LazyAccess::phase(&INIT_MAY_PANICK) == Phase::INITIALIZED);

    assert!(LazyAccess::try_get(&INIT_MAY_PANICK).unwrap().len() == 2);

    assert!(LazyAccess::phase(&INIT_MAY_PANICK) == Phase::INITIALIZED);

    assert_eq!(*INIT_MAY_PANICK, vec![1, 2]);

    assert_eq!(*LazyAccess::get(&INIT_MAY_PANICK), vec![1, 2]);
}

static UNINIT_ONCE_COUNT: AtomicU32 = AtomicU32::new(0);
#[dynamic(lazy, try_init_once)]
static UNINITIALIZABLE: Vec<i32> = {
    UNINIT_ONCE_COUNT.fetch_add(1, Ordering::Relaxed);
    panic!("Panicked on purpose")
};

#[test]
fn init_may_panick_intolerant() {
    assert!(LazyAccess::phase(&UNINITIALIZABLE).is_empty());

    assert!(LazyAccess::try_get(&UNINITIALIZABLE).is_err());

    assert!(LazyAccess::phase(&UNINITIALIZABLE).is_empty());

    assert!(catch_unwind(|| UNINITIALIZABLE.len()).is_err());

    assert_eq!(UNINIT_ONCE_COUNT.load(Ordering::Relaxed), 1);

    assert_eq!(
        LazyAccess::phase(&UNINITIALIZABLE),
        Phase::INITIALIZATION_PANICKED | Phase::INITIALIZATION_SKIPED
    );

    assert!(catch_unwind(|| UNINITIALIZABLE.len()).is_err());

    assert_eq!(UNINIT_ONCE_COUNT.load(Ordering::Relaxed), 1);

    assert_eq!(
        LazyAccess::phase(&UNINITIALIZABLE),
        Phase::INITIALIZATION_PANICKED | Phase::INITIALIZATION_SKIPED
    );

    assert_eq!(UNINIT_ONCE_COUNT.load(Ordering::Relaxed), 1);
}

#[dynamic(lazy, try_init_once)]
static NORMAL_WITH_TOLERANCE: Vec<i32> = vec![1, 2];

#[test]
fn normal_with_tolerance() {
    assert!(LazyAccess::phase(&NORMAL_WITH_TOLERANCE).is_empty());

    assert!(LazyAccess::try_get(&NORMAL_WITH_TOLERANCE).is_err());

    assert!(LazyAccess::phase(&NORMAL_WITH_TOLERANCE).is_empty());

    assert_eq!(NORMAL_WITH_TOLERANCE.len(), 2);

    assert!(LazyAccess::phase(&NORMAL_WITH_TOLERANCE) == Phase::INITIALIZED);

    assert_eq!(
        LazyAccess::try_get(&NORMAL_WITH_TOLERANCE).unwrap().len(),
        2
    );

    assert!(LazyAccess::phase(&NORMAL_WITH_TOLERANCE) == Phase::INITIALIZED);

    assert_eq!(*NORMAL_WITH_TOLERANCE, vec![1, 2]);

    assert_eq!(LazyAccess::get(&NORMAL_WITH_TOLERANCE).len(), 2);
}

#[test]
fn local_lazy() {
    let v = Lazy::new(|| vec![1, 2]);

    assert!(LazyAccess::phase(&v).is_empty());

    assert!(LazyAccess::try_get(&v).is_err());

    assert!(LazyAccess::phase(&v).is_empty());

    assert_eq!(v.len(), 2);

    assert!(LazyAccess::phase(&v) == Phase::INITIALIZED);

    assert!(LazyAccess::try_get(&v).unwrap().len() == 2);

    assert!(LazyAccess::phase(&v) == Phase::INITIALIZED);

    assert_eq!(*v, vec![1, 2]);

    assert_eq!(*LazyAccess::get(&v), vec![1, 2]);

    let mut drop_count: i32 = 0;

    struct A<'a>(&'a mut i32);
    impl<'a> Drop for A<'a> {
        fn drop(&mut self) {
            *self.0 += 1;
        }
    }
    {
        Lazy::new(|| A(&mut drop_count));
    }
    assert_eq!(drop_count, 0);
    {
        let v = Lazy::new(|| A(&mut drop_count));
        LazyAccess::init(&v);
    }
    assert_eq!(drop_count, 1);
}

#[test]
fn local_lazy_mut() {
    let mut v = Lazy::new(|| vec![1, 2]);

    assert!(LazyAccess::phase(&v).is_empty());

    assert!(LazyAccess::try_get(&v).is_err());

    assert!(Lazy::try_get_mut(&mut v).is_err());

    assert!(LazyAccess::phase(&v).is_empty());

    v.push(3);

    assert_eq!(LazyAccess::phase(&v), Phase::INITIALIZED);

    assert_eq!(LazyAccess::try_get(&v).unwrap().len(), 3);

    Lazy::try_get_mut(&mut v).unwrap().push(4);

    Lazy::get_mut(&mut v).push(5);

    assert_eq!(*v, vec![1, 2, 3, 4, 5]);
}
