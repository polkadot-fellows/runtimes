use static_init::{constructor, destructor, dynamic, Phase, Uninit};
use std::sync::atomic::{AtomicU32, Ordering};

static FINALIZE_A_COUNT: AtomicU32 = AtomicU32::new(0);

#[derive(Debug)]
struct A(u32);

impl A {
    fn new(v: u32) -> A {
        A(v)
    }
}

impl Uninit for A {
    fn uninit(&mut self) {
        FINALIZE_A_COUNT.fetch_add(1, Ordering::Relaxed);
    }
}

#[dynamic(prime, drop)]
static mut NORMAL: A = match INIT {
    PRIME => A(1),
    DYN => A::new(33),
};

#[constructor(10)]
extern "C" fn test_pre_normal() {
    assert!(NORMAL.phase().is_empty());

    assert!(NORMAL.try_read().is_err());

    assert!(NORMAL.try_write().is_err());

    assert_eq!(NORMAL.primed_read_non_initializing().unwrap_err().0, 1);

    assert_eq!(NORMAL.primed_write_non_initializing().unwrap_err().0, 1);

    assert!(NORMAL.fast_try_read().unwrap().is_err());

    assert!(NORMAL.fast_try_write().unwrap().is_err());

    assert!(NORMAL.phase().is_empty());
}

#[test]
fn normal() {
    assert!(NORMAL.phase() == Phase::INITIALIZED | Phase::REGISTERED);

    assert_eq!(NORMAL.try_read().unwrap().0, 33);

    assert_eq!(NORMAL.try_write().unwrap().0, 33);

    assert_eq!(NORMAL.fast_try_read().unwrap().unwrap().0, 33);

    assert_eq!(NORMAL.fast_try_write().unwrap().unwrap().0, 33);

    assert_eq!(NORMAL.primed_read_non_initializing().unwrap().0, 33);

    assert_eq!(NORMAL.primed_write_non_initializing().unwrap().0, 33);

    assert!(NORMAL.phase() == Phase::INITIALIZED | Phase::REGISTERED);

    assert_eq!(NORMAL.read().0, 33);

    NORMAL.write().0 = 12;

    assert_eq!(NORMAL.read().0, 12);

    NORMAL.primed_write().unwrap().0 = 42;

    assert_eq!(NORMAL.primed_read().unwrap().0, 42);
}

#[destructor(10)]
extern "C" fn check_a_finalized() {
    assert_eq!(FINALIZE_A_COUNT.load(Ordering::Relaxed), 1)
}

static FINALIZE_B_COUNT: AtomicU32 = AtomicU32::new(0);

#[derive(Debug)]
struct B(u32);

impl B {
    fn new(v: u32) -> B {
        B(v)
    }
}

impl Uninit for B {
    fn uninit(&mut self) {
        FINALIZE_B_COUNT.fetch_add(1, Ordering::Relaxed);
    }
}

#[destructor(10)]
extern "C" fn check_b_finalized() {
    assert_eq!(FINALIZE_B_COUNT.load(Ordering::Relaxed), 1)
}

#[dynamic(prime, drop)]
static mut PRE_INITED_NORMAL: B = match INIT {
    PRIME => B(1),
    DYN => B::new(12),
};

#[constructor(10)]
extern "C" fn test_pre_pre_inited_normal() {
    assert!(PRE_INITED_NORMAL.phase().is_empty());

    assert!(PRE_INITED_NORMAL.try_read().is_err());

    assert_eq!(
        PRE_INITED_NORMAL
            .primed_read_non_initializing()
            .unwrap_err()
            .0,
        1
    );

    assert_eq!(
        PRE_INITED_NORMAL
            .primed_write_non_initializing()
            .unwrap_err()
            .0,
        1
    );

    assert!(PRE_INITED_NORMAL.phase().is_empty());

    assert_eq!(PRE_INITED_NORMAL.primed_read().unwrap().0, 12);

    assert!(PRE_INITED_NORMAL.phase() == Phase::INITIALIZED | Phase::REGISTERED);

    assert_eq!(PRE_INITED_NORMAL.try_read().unwrap().0, 12);

    assert!(PRE_INITED_NORMAL.phase() == Phase::INITIALIZED | Phase::REGISTERED);

    assert_eq!(PRE_INITED_NORMAL.read().0, 12);

    PRE_INITED_NORMAL.write().0 = 33;
}

#[test]
fn pre_inited_normal() {
    assert_eq!(
        PRE_INITED_NORMAL.phase(),
        Phase::INITIALIZED | Phase::REGISTERED
    );

    assert_eq!(PRE_INITED_NORMAL.try_read().unwrap().0, 33);

    assert_eq!(PRE_INITED_NORMAL.primed_read().unwrap().0, 33);

    assert_eq!(PRE_INITED_NORMAL.read().0, 33);
}

static FINALIZE_C_COUNT: AtomicU32 = AtomicU32::new(0);

#[derive(Debug)]
struct C(u32);

impl C {
    fn new(v: u32) -> C {
        C(v)
    }
}

impl Uninit for C {
    fn uninit(&mut self) {
        FINALIZE_C_COUNT.fetch_add(1, Ordering::Relaxed);
    }
}

#[destructor(10)]
extern "C" fn check_c_finalized() {
    assert_eq!(FINALIZE_C_COUNT.load(Ordering::Relaxed), 1)
}

#[dynamic(prime, drop, try_init_once)]
static mut NORMAL_WITH_TOLERANCE: C = match INIT {
    PRIME => C(1),
    DYN => C::new(33),
};

#[test]
fn normal_with_tolerance() {
    assert_eq!(NORMAL_WITH_TOLERANCE.read().0, 33);

    assert_eq!(NORMAL_WITH_TOLERANCE.primed_read().unwrap().0, 33);

    assert!(NORMAL_WITH_TOLERANCE.phase() == Phase::INITIALIZED | Phase::REGISTERED);

    assert_eq!(NORMAL_WITH_TOLERANCE.try_read().unwrap().0, 33);

    assert!(NORMAL_WITH_TOLERANCE.phase() == Phase::INITIALIZED | Phase::REGISTERED);

    assert_eq!(NORMAL_WITH_TOLERANCE.read().0, 33);

    assert_eq!(NORMAL_WITH_TOLERANCE.write().0, 33);
}
