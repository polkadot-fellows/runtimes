use static_init::{constructor, destructor, dynamic, Finaly, LazyAccess, Phase};
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
static NORMAL: A = A::new(33);

#[constructor(10)]
extern "C" fn test_pre_normal() {
    assert!(LazyAccess::phase(&NORMAL).is_empty());

    assert!(LazyAccess::try_get(&NORMAL).is_err());

    assert!(LazyAccess::phase(&NORMAL).is_empty());
}

#[test]
fn normal() {
    assert!(LazyAccess::phase(&NORMAL) == Phase::INITIALIZED | Phase::REGISTERED);

    assert_eq!(LazyAccess::try_get(&NORMAL).unwrap().0, 33);

    assert!(LazyAccess::phase(&NORMAL) == Phase::INITIALIZED | Phase::REGISTERED);

    assert_eq!(NORMAL.0, 33);

    assert_eq!(LazyAccess::get(&NORMAL).0, 33);
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
static PRE_INITED_NORMAL: B = B::new(12);

#[constructor(10)]
extern "C" fn test_pre_pre_inited_normal() {
    assert!(LazyAccess::phase(&PRE_INITED_NORMAL).is_empty());

    assert!(LazyAccess::try_get(&PRE_INITED_NORMAL).is_err());

    assert!(LazyAccess::phase(&PRE_INITED_NORMAL).is_empty());

    assert_eq!(PRE_INITED_NORMAL.0, 12);

    assert!(LazyAccess::phase(&PRE_INITED_NORMAL) == Phase::INITIALIZED | Phase::REGISTERED);

    assert_eq!(LazyAccess::try_get(&PRE_INITED_NORMAL).unwrap().0, 12);

    assert!(LazyAccess::phase(&PRE_INITED_NORMAL) == Phase::INITIALIZED | Phase::REGISTERED);

    assert_eq!(PRE_INITED_NORMAL.0, 12);

    assert_eq!(LazyAccess::get(&PRE_INITED_NORMAL).0, 12);
}

#[test]
fn pre_inited_normal() {
    assert_eq!(
        LazyAccess::phase(&PRE_INITED_NORMAL),
        Phase::INITIALIZED | Phase::REGISTERED
    );

    assert_eq!(LazyAccess::try_get(&PRE_INITED_NORMAL).unwrap().0, 12);

    assert_eq!(LazyAccess::get(&PRE_INITED_NORMAL).0, 12);

    assert_eq!(PRE_INITED_NORMAL.0, 12);
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
static NORMAL_WITH_TOLERANCE: C = C::new(33);

#[test]
fn normal_with_tolerance() {
    assert_eq!(NORMAL_WITH_TOLERANCE.0, 33);

    assert!(LazyAccess::phase(&NORMAL_WITH_TOLERANCE) == Phase::INITIALIZED | Phase::REGISTERED);

    assert_eq!(LazyAccess::try_get(&NORMAL_WITH_TOLERANCE).unwrap().0, 33);

    assert!(LazyAccess::phase(&NORMAL_WITH_TOLERANCE) == Phase::INITIALIZED | Phase::REGISTERED);

    assert_eq!(NORMAL_WITH_TOLERANCE.0, 33);

    assert_eq!(LazyAccess::get(&NORMAL_WITH_TOLERANCE).0, 33);
}
