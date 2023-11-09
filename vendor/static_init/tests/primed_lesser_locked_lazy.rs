use static_init::{constructor, dynamic, Phase};

#[dynamic(prime)]
static mut NORMAL: Vec<i32> = match INIT {
    PRIME => vec![],
    DYN => vec![1, 2],
};

#[test]
fn normal() {
    assert_eq!(NORMAL.phase(), Phase::INITIALIZED);

    assert_eq!(*NORMAL.try_read().unwrap(), vec![1, 2]);

    assert_eq!(*NORMAL.try_write().unwrap(), vec![1, 2]);

    assert_eq!(*NORMAL.fast_try_read().unwrap().unwrap(), vec![1, 2]);

    assert_eq!(*NORMAL.fast_try_write().unwrap().unwrap(), vec![1, 2]);

    assert_eq!(*NORMAL.primed_write().unwrap(), vec![1, 2]);

    assert_eq!(*NORMAL.primed_read().unwrap(), vec![1, 2]);

    assert_eq!(*NORMAL.primed_write_non_initializing().unwrap(), vec![1, 2]);

    assert_eq!(*NORMAL.primed_read_non_initializing().unwrap(), vec![1, 2]);

    assert_eq!(*NORMAL.read(), vec![1, 2]);

    NORMAL.write().push(3);

    NORMAL.try_write().unwrap().push(4);

    NORMAL.fast_try_write().unwrap().unwrap().push(5);

    assert_eq!(*NORMAL.read(), vec![1, 2, 3, 4, 5]);
}

#[constructor(10)]
extern "C" fn test_pre_normal() {
    assert!(NORMAL.phase().is_empty());

    assert!(NORMAL.try_read().is_err());

    assert_eq!(NORMAL.primed_read_non_initializing().unwrap_err().len(), 0);

    assert_eq!(NORMAL.primed_write_non_initializing().unwrap_err().len(), 0);

    assert!(NORMAL.phase().is_empty());
}

#[dynamic(prime)]
static mut PRE_INITED_NORMAL: Vec<i32> = match INIT {
    PRIME => vec![],
    DYN => vec![1, 2],
};

#[constructor(10)]
extern "C" fn test_pre_pre_inited_normal() {
    assert!(PRE_INITED_NORMAL.phase().is_empty());

    assert!(PRE_INITED_NORMAL.try_read().is_err());

    assert!(PRE_INITED_NORMAL.phase().is_empty());

    assert_eq!(PRE_INITED_NORMAL.primed_read().unwrap().len(), 2);

    assert!(PRE_INITED_NORMAL.phase() == Phase::INITIALIZED);

    assert!(PRE_INITED_NORMAL.try_read().unwrap().len() == 2);

    assert!(PRE_INITED_NORMAL.phase() == Phase::INITIALIZED);

    assert_eq!(*PRE_INITED_NORMAL.read(), vec![1, 2]);

    assert_eq!(*PRE_INITED_NORMAL.write(), vec![1, 2]);

    PRE_INITED_NORMAL.primed_write().unwrap().push(3);
}

#[test]
fn pre_inited_normal() {
    assert_eq!(PRE_INITED_NORMAL.phase(), Phase::INITIALIZED);

    assert_eq!(*PRE_INITED_NORMAL.try_read().unwrap(), vec![1, 2, 3]);

    assert_eq!(*PRE_INITED_NORMAL.read(), vec![1, 2, 3]);

    assert_eq!(*PRE_INITED_NORMAL.write(), vec![1, 2, 3]);
}
