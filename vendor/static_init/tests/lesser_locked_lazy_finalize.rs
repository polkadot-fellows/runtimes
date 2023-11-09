use static_init::{constructor, destructor, dynamic, Finaly, Phase};
use std::sync::atomic::{AtomicU32, Ordering};

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

#[dynamic(finalize)]
static mut NORMAL: A = A::new(33);

#[constructor(10)]
extern "C" fn test_pre_normal() {
    assert!(NORMAL.phase().is_empty());

    assert!(NORMAL.try_read().is_err());

    assert!(NORMAL.try_write().is_err());

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

    assert!(NORMAL.phase() == Phase::INITIALIZED | Phase::REGISTERED);

    assert_eq!(NORMAL.read().0, 33);

    NORMAL.write().0 = 12;

    assert_eq!(NORMAL.read().0, 12);
}

#[destructor(10)]
extern "C" fn check_a_finalized() {
    assert_eq!(FINALIZE_A_COUNT.load(Ordering::Relaxed), 1)
}

static FINALIZE_B_COUNT: AtomicU32 = AtomicU32::new(0);

struct B(u32);

impl B {
    fn new(v: u32) -> B {
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

#[dynamic(finalize)]
static mut PRE_INITED_NORMAL: B = B::new(12);

#[constructor(10)]
extern "C" fn test_pre_pre_inited_normal() {
    assert!(PRE_INITED_NORMAL.phase().is_empty());

    assert!(PRE_INITED_NORMAL.try_read().is_err());

    assert!(PRE_INITED_NORMAL.phase().is_empty());

    assert_eq!(PRE_INITED_NORMAL.read().0, 12);

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

    assert_eq!(PRE_INITED_NORMAL.read().0, 33);
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

#[dynamic(finalize, try_init_once)]
static mut NORMAL_WITH_TOLERANCE: C = C::new(33);

#[test]
fn normal_with_tolerance() {
    assert_eq!(NORMAL_WITH_TOLERANCE.read().0, 33);

    assert!(NORMAL_WITH_TOLERANCE.phase() == Phase::INITIALIZED | Phase::REGISTERED);

    assert_eq!(NORMAL_WITH_TOLERANCE.try_read().unwrap().0, 33);

    assert!(NORMAL_WITH_TOLERANCE.phase() == Phase::INITIALIZED | Phase::REGISTERED);

    assert_eq!(NORMAL_WITH_TOLERANCE.read().0, 33);

    assert_eq!(NORMAL_WITH_TOLERANCE.write().0, 33);
}
