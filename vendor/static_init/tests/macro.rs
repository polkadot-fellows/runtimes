// Copyright 2021 Olivier Kannengieser
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
#![cfg_attr(feature = "thread_local", feature(thread_local))]

extern crate static_init;
use static_init::{constructor, destructor, dynamic, Finaly, Uninit};

static mut DEST: i32 = 0;

#[destructor(0)]
extern "C" fn dest_0() {
    unsafe {
        assert_eq!(DEST, 0);
        DEST += 1;
    }
}

#[destructor(1)]
extern "C" fn dest_1() {
    unsafe {
        assert_eq!(DEST, 1);
        DEST += 1;
    }
}
#[destructor(100)]
extern "C" fn dest_2() {
    unsafe {
        assert_eq!(DEST, 2);
        DEST += 1;
    }
}

static mut INI: i32 = 0;

#[constructor(200)]
extern "C" fn init_2() {
    unsafe {
        assert_eq!(INI, 0);
        INI += 1;
    }
}
#[constructor(1)]
extern "C" fn init_1() {
    unsafe {
        assert_eq!(INI, 1);
        INI += 1;
    }
}
#[constructor(0)]
extern "C" fn init_0() {
    unsafe {
        assert_eq!(INI, 2);
        INI += 1;
    }
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
mod gnu {
    use super::constructor;
    use std::env::args_os;
    use std::ffi::{CStr, OsStr};
    use std::os::unix::ffi::OsStrExt;

    #[constructor]
    extern "C" fn get_args_env(argc: i32, mut argv: *const *const u8, _env: *const *const u8) {
        let mut argc_counted = 0;
        unsafe {
            while !(*argv).is_null() {
                assert!(
                    args_os()
                        .any(|x| x
                            == OsStr::from_bytes(CStr::from_ptr(*argv as *const i8).to_bytes()))
                );
                argv = argv.add(1);
                argc_counted += 1
            }
        }
        assert_eq!(argc_counted, argc);
    }
}

#[derive(Debug, Eq, PartialEq)]
struct A(i32);

impl A {
    fn new(v: i32) -> A {
        A(v)
    }
}
impl Drop for A {
    fn drop(&mut self) {
        assert_eq!(self.0, 33)
    }
}
impl Finaly for A {
    fn finaly(&self) {
        assert_eq!(self.0, 33)
    }
}
impl Uninit for A {
    fn uninit(&mut self) {
        assert_eq!(self.0, 33)
    }
}

#[test]
#[cfg(not(miri))] //miri do not know about program constructors
fn inner_static() {
    #[dynamic(0)]
    static IX: usize = unsafe { &IX as *const _ as usize };
    #[dynamic(0)]
    static IX2: usize = unsafe { &IX2 as *const _ as usize };

    static mut I: i32 = 0;

    #[constructor]
    extern "C" fn f() {
        unsafe { I = 3 }
    }

    unsafe {
        assert_eq!(*IX, &IX as *const _ as usize);
        assert_eq!(*IX2, &IX2 as *const _ as usize);
        assert_eq!(I, 3)
    };
}

#[dynamic(0)]
static mut V0: A = A::new(unsafe { V1.0 } - 5);

#[dynamic(20)]
static mut V2: A = A::new(12);

#[dynamic(10)]
static V1: A = A::new(unsafe { V2.0 } - 2);

#[dynamic(init = 20)]
static mut V3: A = A::new(12);

#[dynamic(init = 10)]
static V4: A = A::new(unsafe { V2.0 } - 2);

#[dynamic(init = 5, drop)]
static V5: A = A::new(unsafe { V4.0 } + 23);

#[dynamic(drop_only = 0)]
static V6: A = A(33);

#[dynamic(init = 2, drop = 10)]
static V7: A = A::new(unsafe { V5.0 });

#[test]
#[cfg(not(miri))] //miri do not know about program constructors
fn dynamic_init() {
    unsafe {
        assert_eq!(V0.0, 5);
        assert_eq!(V1.0, 10);
        assert_eq!(V2.0, 12);
        V2.0 = 8;
        assert_eq!(V2.0, 8);
        assert_eq!(V4.0, 10);
        assert_eq!(V3.0, 12);
        assert_eq!(V5.0, 33);
        assert_eq!(V6.0, 33);
    }
}

mod lazy {
    #[cfg(any(feature = "thread_local"))]
    use super::A;
    #[cfg(any(feature = "thread_local"))]
    use static_init::dynamic;

    #[cfg(feature = "thread_local")]
    #[test]
    fn thread_local() {
        #[thread_local]
        #[dynamic(lazy)]
        static mut TH_LOCAL: A = A::new(3);

        #[dynamic(prime)]
        #[thread_local]
        static mut L4: i32 = match INIT {
            PRIME => 42,
            DYN => 33,
        };

        assert_eq!(TH_LOCAL.read().0, 3);

        match L4.primed_read_non_initializing() {
            Ok(_) => panic!("Unexpected"),
            Err(x) => assert_eq!(*x, 42),
        }
        assert_eq!(*L4.read(), 33);

        TH_LOCAL.write().0 = 42;

        assert_eq!(TH_LOCAL.read().0, 42);

        std::thread::spawn(|| {
            assert_eq!(TH_LOCAL.read().0, 3);
            match L4.primed_read_non_initializing() {
                Ok(_) => panic!("Unexpected"),
                Err(x) => assert_eq!(*x, 42),
            }
            assert_eq!(*L4.read(), 33);
        })
        .join()
        .unwrap();
    }

    #[cfg(all(feature = "thread_local"))]
    #[test]
    fn thread_local_drop() {
        use core::sync::atomic::{AtomicI32, Ordering};
        #[thread_local]
        #[dynamic(lazy, drop)]
        static TH_LOCAL_UNSAFE: i32 = 10;

        assert_eq!(*TH_LOCAL_UNSAFE, 10);

        static DROP_COUNT: AtomicI32 = AtomicI32::new(0);

        struct B;

        impl Drop for B {
            fn drop(&mut self) {
                DROP_COUNT.fetch_add(1, Ordering::Relaxed);
            }
        }

        #[thread_local]
        #[dynamic(lazy, drop)]
        static B1: B = B;

        #[thread_local]
        #[dynamic(lazy, drop)]
        static mut B2: B = B;

        std::thread::spawn(|| {
            let _ = &*B1;
            let _ = &*B2.read();
        })
        .join()
        .unwrap();
        std::thread::spawn(|| ()).join().unwrap();
        std::thread::spawn(|| {
            &*B1;
            &*B2.read();
        })
        .join()
        .unwrap();
        assert_eq!(DROP_COUNT.load(Ordering::Relaxed), 4);
    }

    mod global_lazy {
        use super::super::A;
        use static_init::dynamic;
        #[dynamic(lazy)]
        static L1: A = A::new(L0.read().0 + 1);

        #[dynamic(lazy)]
        static mut L0: A = A::new(10);

        #[dynamic(finalize)]
        static L3: A = A::new(33);

        #[dynamic(lesser_lazy, finalize)]
        static mut L2: A = A::new(L3.0);

        #[dynamic(lazy, prime)]
        static mut L4: i32 = match INIT {
            PRIME => 42,
            DYN => 33,
        };

        #[dynamic(lazy, prime, drop)]
        static mut L5: A = match INIT {
            PRIME => A(33),
            DYN => A::new(12),
        };

        #[test]
        fn lazy_init() {
            assert_eq!(L0.read().0, 10);
            assert_eq!(L1.0, 11);
            assert_eq!(L2.read().0, 33);
            assert_eq!(L3.0, 33);
            match L4.primed_read_non_initializing() {
                Ok(_) => panic!("Unexpected"),
                Err(x) => assert_eq!(*x, 42),
            }
            assert_eq!(*L4.read(), 33);
            match L5.primed_read_non_initializing() {
                Ok(_) => panic!("Unexpected"),
                Err(x) => assert_eq!(x.0, 33),
            }
            assert_eq!(L5.read().0, 12);
            L5.write().0 = 33;
        }
    }
}
