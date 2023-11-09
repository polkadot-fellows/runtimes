use static_init::{constructor, dynamic, LazyAccess, Phase};

#[dynamic]
static NORMAL: Vec<i32> = vec![1, 2];

#[test]
fn normal() {
    assert_eq!(LazyAccess::phase(&NORMAL), Phase::INITIALIZED);

    assert_eq!(*LazyAccess::try_get(&NORMAL).unwrap(), vec![1, 2]);

    assert_eq!(*LazyAccess::get(&NORMAL), vec![1, 2]);

    assert_eq!(*NORMAL, vec![1, 2]);
}

#[constructor(10)]
extern "C" fn test_pre_normal() {
    assert!(LazyAccess::phase(&NORMAL).is_empty());

    assert!(LazyAccess::try_get(&NORMAL).is_err());

    assert!(LazyAccess::phase(&NORMAL).is_empty());
}

#[dynamic]
static PRE_INITED_NORMAL: Vec<i32> = vec![1, 2];

#[constructor(10)]
extern "C" fn test_pre_pre_inited_normal() {
    assert!(LazyAccess::phase(&PRE_INITED_NORMAL).is_empty());

    assert!(LazyAccess::try_get(&PRE_INITED_NORMAL).is_err());

    assert!(LazyAccess::phase(&PRE_INITED_NORMAL).is_empty());

    assert_eq!(PRE_INITED_NORMAL.len(), 2);

    assert!(LazyAccess::phase(&PRE_INITED_NORMAL) == Phase::INITIALIZED);

    assert!(LazyAccess::try_get(&PRE_INITED_NORMAL).unwrap().len() == 2);

    assert!(LazyAccess::phase(&PRE_INITED_NORMAL) == Phase::INITIALIZED);

    assert_eq!(*PRE_INITED_NORMAL, vec![1, 2]);

    assert_eq!(*LazyAccess::get(&PRE_INITED_NORMAL), vec![1, 2]);
}

#[test]
fn pre_inited_normal() {
    assert_eq!(LazyAccess::phase(&PRE_INITED_NORMAL), Phase::INITIALIZED);

    assert_eq!(
        *LazyAccess::try_get(&PRE_INITED_NORMAL).unwrap(),
        vec![1, 2]
    );

    assert_eq!(*LazyAccess::get(&PRE_INITED_NORMAL), vec![1, 2]);

    assert_eq!(*PRE_INITED_NORMAL, vec![1, 2]);
}
