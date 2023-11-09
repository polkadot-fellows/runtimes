#![cfg(feature = "thread_local")]
#![feature(thread_local)]

use static_init::{destructor, dynamic, Finaly, LazyAccess, Phase};
use std::panic::catch_unwind;
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread::spawn;

static FINALIZE_A_COUNT: AtomicU32 = AtomicU32::new(0);

struct A(u32);

impl A {
    fn new(v: u32) -> A {
        A(v)
    }
}

impl Finaly for A {
    fn finaly(&self) {
        FINALIZE_A_COUNT.fetch_add(1, Ordering::Relaxed);
    }
}

#[dynamic(lazy, finalize)]
#[thread_local]
static NORMAL: A = A::new(33);

#[test]
fn normal() {
    assert!(LazyAccess::phase(&NORMAL).is_empty());

    assert!(LazyAccess::try_get(&NORMAL).is_err());

    assert!(LazyAccess::phase(&NORMAL).is_empty());

    assert_eq!(NORMAL.0, 33);

    assert!(LazyAccess::phase(&NORMAL) == Phase::INITIALIZED | Phase::REGISTERED);

    assert_eq!(LazyAccess::try_get(&NORMAL).unwrap().0, 33);

    assert!(LazyAccess::phase(&NORMAL) == Phase::INITIALIZED | Phase::REGISTERED);

    assert_eq!(NORMAL.0, 33);

    assert_eq!(LazyAccess::get(&NORMAL).0, 33);

    spawn(|| {
        assert!(LazyAccess::phase(&NORMAL).is_empty());
        assert_eq!(NORMAL.0, 33);
        assert!(LazyAccess::phase(&NORMAL) == Phase::INITIALIZED | Phase::REGISTERED);
    })
    .join()
    .unwrap();
}

#[destructor(10)]
extern "C" fn check_a_finalized() {
    assert_eq!(FINALIZE_A_COUNT.load(Ordering::Relaxed), 2)
}

static FINALIZE_B_COUNT: AtomicU32 = AtomicU32::new(0);

struct B(u32);

impl B {
    fn new(v: u32) -> Self {
        B(v)
    }
}

impl Finaly for B {
    fn finaly(&self) {
        FINALIZE_B_COUNT.fetch_add(1, Ordering::Relaxed);
    }
}

#[destructor(10)]
extern "C" fn check_b_finalized() {
    assert_eq!(FINALIZE_B_COUNT.load(Ordering::Relaxed), 1)
}

static UNINIT_COUNT: AtomicU32 = AtomicU32::new(0);

#[dynamic(lazy, finalize)]
#[thread_local]
static INIT_MAY_PANICK: B = {
    if UNINIT_COUNT.fetch_add(1, Ordering::Relaxed) < 2 {
        panic!("Should not be seen");
    }
    B::new(42)
};

#[test]
fn init_may_panick() {
    assert!(LazyAccess::phase(&INIT_MAY_PANICK).is_empty());

    assert!(LazyAccess::try_get(&INIT_MAY_PANICK).is_err());

    assert!(LazyAccess::phase(&INIT_MAY_PANICK).is_empty());

    assert!(catch_unwind(|| INIT_MAY_PANICK.0).is_err());

    assert_eq!(UNINIT_COUNT.load(Ordering::Relaxed), 1);

    assert_eq!(
        LazyAccess::phase(&INIT_MAY_PANICK),
        Phase::REGISTERED | Phase::INITIALIZATION_PANICKED | Phase::INITIALIZATION_SKIPED
    );

    assert!(catch_unwind(|| INIT_MAY_PANICK.0).is_err());

    assert_eq!(UNINIT_COUNT.load(Ordering::Relaxed), 2);

    assert_eq!(
        LazyAccess::phase(&INIT_MAY_PANICK),
        Phase::REGISTERED | Phase::INITIALIZATION_PANICKED | Phase::INITIALIZATION_SKIPED
    );

    assert_eq!(INIT_MAY_PANICK.0, 42);

    assert_eq!(UNINIT_COUNT.load(Ordering::Relaxed), 3);

    assert!(LazyAccess::phase(&INIT_MAY_PANICK) == Phase::REGISTERED | Phase::INITIALIZED);

    assert_eq!(LazyAccess::try_get(&INIT_MAY_PANICK).unwrap().0, 42);

    assert!(LazyAccess::phase(&INIT_MAY_PANICK) == Phase::REGISTERED | Phase::INITIALIZED);

    assert_eq!(INIT_MAY_PANICK.0, 42);

    assert_eq!(LazyAccess::get(&INIT_MAY_PANICK).0, 42);
}

static FINALIZE_C_COUNT: AtomicU32 = AtomicU32::new(0);

struct C(u32);

impl C {
    fn new(v: u32) -> C {
        C(v)
    }
}

impl Finaly for C {
    fn finaly(&self) {
        FINALIZE_C_COUNT.fetch_add(1, Ordering::Relaxed);
    }
}

#[destructor(10)]
extern "C" fn check_c_finalized() {
    assert_eq!(FINALIZE_C_COUNT.load(Ordering::Relaxed), 1)
}

static UNINIT_ONCE_COUNT: AtomicU32 = AtomicU32::new(0);

#[dynamic(lazy, finalize, try_init_once)]
#[thread_local]
static UNINITIALIZABLE: C = {
    UNINIT_ONCE_COUNT.fetch_add(1, Ordering::Relaxed);
    panic!("Panicked on purpose")
};

#[test]
fn init_may_panick_intolerant() {
    assert!(LazyAccess::phase(&UNINITIALIZABLE).is_empty());

    assert!(LazyAccess::try_get(&UNINITIALIZABLE).is_err());

    assert!(LazyAccess::phase(&UNINITIALIZABLE).is_empty());

    assert!(catch_unwind(|| UNINITIALIZABLE.0).is_err());

    assert_eq!(UNINIT_ONCE_COUNT.load(Ordering::Relaxed), 1);

    assert_eq!(
        LazyAccess::phase(&UNINITIALIZABLE),
        Phase::REGISTERED | Phase::INITIALIZATION_PANICKED | Phase::INITIALIZATION_SKIPED
    );

    assert!(catch_unwind(|| UNINITIALIZABLE.0).is_err());

    assert_eq!(UNINIT_ONCE_COUNT.load(Ordering::Relaxed), 1);

    assert_eq!(
        LazyAccess::phase(&UNINITIALIZABLE),
        Phase::REGISTERED | Phase::INITIALIZATION_PANICKED | Phase::INITIALIZATION_SKIPED
    );

    assert_eq!(UNINIT_ONCE_COUNT.load(Ordering::Relaxed), 1);
}

#[dynamic(lazy, finalize, try_init_once)]
#[thread_local]
static NORMAL_WITH_TOLERANCE: C = C::new(33);

#[test]
fn normal_with_tolerance() {
    assert!(LazyAccess::phase(&NORMAL_WITH_TOLERANCE).is_empty());

    assert!(LazyAccess::try_get(&NORMAL_WITH_TOLERANCE).is_err());

    assert!(LazyAccess::phase(&NORMAL_WITH_TOLERANCE).is_empty());

    assert_eq!(NORMAL_WITH_TOLERANCE.0, 33);

    assert!(LazyAccess::phase(&NORMAL_WITH_TOLERANCE) == Phase::INITIALIZED | Phase::REGISTERED);

    assert_eq!(LazyAccess::try_get(&NORMAL_WITH_TOLERANCE).unwrap().0, 33);

    assert!(LazyAccess::phase(&NORMAL_WITH_TOLERANCE) == Phase::INITIALIZED | Phase::REGISTERED);

    assert_eq!(NORMAL_WITH_TOLERANCE.0, 33);

    assert_eq!(LazyAccess::get(&NORMAL_WITH_TOLERANCE).0, 33);
}
