#![cfg(feature = "thread_local")]
#![feature(thread_local)]

use static_init::{destructor, dynamic, Finaly, Phase};
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
static mut NORMAL: A = A::new(33);

#[test]
fn normal() {
    assert!(NORMAL.phase().is_empty());

    assert!(NORMAL.try_read().is_err());

    assert!(NORMAL.try_write().is_err());

    assert!(NORMAL.fast_try_read().unwrap().is_err());

    assert!(NORMAL.fast_try_write().unwrap().is_err());

    assert!(NORMAL.phase().is_empty());

    assert_eq!(NORMAL.read().0, 33);

    assert_eq!(NORMAL.write().0, 33);

    assert_eq!(NORMAL.fast_write().unwrap().0, 33);

    assert_eq!(NORMAL.fast_read().unwrap().0, 33);

    assert_eq!(NORMAL.fast_try_write().unwrap().unwrap().0, 33);

    assert_eq!(NORMAL.fast_try_read().unwrap().unwrap().0, 33);

    assert!(NORMAL.phase() == Phase::INITIALIZED | Phase::REGISTERED);

    assert_eq!(NORMAL.try_read().unwrap().0, 33);

    assert!(NORMAL.phase() == Phase::INITIALIZED | Phase::REGISTERED);

    NORMAL.write().0 = 12;

    assert_eq!(NORMAL.read().0, 12);

    NORMAL.fast_write().unwrap().0 += 21;

    assert_eq!(NORMAL.read().0, 33);

    NORMAL.fast_try_write().unwrap().unwrap().0 += 9;

    assert_eq!(NORMAL.read().0, 42);

    spawn(|| assert_eq!(NORMAL.read().0, 33)).join().unwrap();
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
static mut INIT_MAY_PANICK: B = {
    if UNINIT_COUNT.fetch_add(1, Ordering::Relaxed) < 2 {
        panic!("Should not be seen");
    }
    B::new(42)
};

#[test]
fn init_may_panick() {
    assert!(INIT_MAY_PANICK.phase().is_empty());

    assert!(INIT_MAY_PANICK.try_read().is_err());

    assert!(INIT_MAY_PANICK.phase().is_empty());

    assert!(catch_unwind(|| INIT_MAY_PANICK.read().0).is_err());

    assert_eq!(UNINIT_COUNT.load(Ordering::Relaxed), 1);

    assert_eq!(
        INIT_MAY_PANICK.phase(),
        Phase::REGISTERED | Phase::INITIALIZATION_PANICKED | Phase::INITIALIZATION_SKIPED
    );

    assert!(catch_unwind(|| INIT_MAY_PANICK.read().0).is_err());

    assert_eq!(UNINIT_COUNT.load(Ordering::Relaxed), 2);

    assert_eq!(
        INIT_MAY_PANICK.phase(),
        Phase::REGISTERED | Phase::INITIALIZATION_PANICKED | Phase::INITIALIZATION_SKIPED
    );

    assert_eq!(INIT_MAY_PANICK.read().0, 42);

    assert_eq!(UNINIT_COUNT.load(Ordering::Relaxed), 3);

    assert!(INIT_MAY_PANICK.phase() == Phase::REGISTERED | Phase::INITIALIZED);

    assert_eq!(INIT_MAY_PANICK.try_read().unwrap().0, 42);

    assert!(INIT_MAY_PANICK.phase() == Phase::REGISTERED | Phase::INITIALIZED);

    assert_eq!(INIT_MAY_PANICK.write().0, 42);

    assert_eq!(INIT_MAY_PANICK.read().0, 42);
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
static mut UNINITIALIZABLE: C = {
    UNINIT_ONCE_COUNT.fetch_add(1, Ordering::Relaxed);
    panic!("Panicked on purpose")
};

#[test]
fn init_may_panick_intolerant() {
    assert!(UNINITIALIZABLE.phase().is_empty());

    assert!(UNINITIALIZABLE.try_read().is_err());

    assert!(UNINITIALIZABLE.phase().is_empty());

    assert!(catch_unwind(|| UNINITIALIZABLE.read().0).is_err());

    assert_eq!(UNINIT_ONCE_COUNT.load(Ordering::Relaxed), 1);

    assert_eq!(
        UNINITIALIZABLE.phase(),
        Phase::REGISTERED | Phase::INITIALIZATION_PANICKED | Phase::INITIALIZATION_SKIPED
    );

    assert!(catch_unwind(|| UNINITIALIZABLE.fast_write().unwrap().0).is_err());

    assert_eq!(UNINIT_ONCE_COUNT.load(Ordering::Relaxed), 1);

    assert_eq!(
        UNINITIALIZABLE.phase(),
        Phase::REGISTERED | Phase::INITIALIZATION_PANICKED | Phase::INITIALIZATION_SKIPED
    );

    assert_eq!(UNINIT_ONCE_COUNT.load(Ordering::Relaxed), 1);
}

#[dynamic(lazy, finalize, try_init_once)]
#[thread_local]
static mut NORMAL_WITH_TOLERANCE: C = C::new(33);

#[test]
fn normal_with_tolerance() {
    assert!(NORMAL_WITH_TOLERANCE.phase().is_empty());

    assert!(NORMAL_WITH_TOLERANCE.try_read().is_err());

    assert!(NORMAL_WITH_TOLERANCE.phase().is_empty());

    assert_eq!(NORMAL_WITH_TOLERANCE.read().0, 33);

    assert!(NORMAL_WITH_TOLERANCE.phase() == Phase::INITIALIZED | Phase::REGISTERED);

    assert_eq!(NORMAL_WITH_TOLERANCE.try_read().unwrap().0, 33);

    assert!(NORMAL_WITH_TOLERANCE.phase() == Phase::INITIALIZED | Phase::REGISTERED);

    assert_eq!(NORMAL_WITH_TOLERANCE.read().0, 33);

    assert_eq!(NORMAL_WITH_TOLERANCE.write().0, 33);
}
