// Copyright 2021 Olivier Kannengieser
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
#![cfg_attr(feature = "thread_local", feature(thread_local))]
#![cfg(not(miri))]
//miri does not start any test and block, dont know why...

extern crate static_init;
use static_init::{constructor, destructor, dynamic, Finaly};

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread::spawn;

//use std::thread::sleep;
//use std::time::Duration;

struct A(i32);

impl A {
    fn new(v: i32) -> Self {
        for _ in 1..100000 {
            std::hint::spin_loop();
        }
        A(v)
    }
}

static FINALY_COUNT: AtomicUsize = AtomicUsize::new(0);

impl Finaly for A {
    fn finaly(&self) {
        FINALY_COUNT.fetch_add(1, Ordering::Relaxed);
    }
}
static DROP_COUNT: AtomicUsize = AtomicUsize::new(0);

impl Drop for A {
    fn drop(&mut self) {
        DROP_COUNT.fetch_add(1, Ordering::Relaxed);
    }
}

#[cfg(not(feature = "thread_local"))]
const FINALY_COUNT_EXPECTED: usize = 12;
#[cfg(not(feature = "thread_local"))]
const DROP_COUNT_EXPECTED: usize = 6;
#[cfg(feature = "thread_local")]
const FINALY_COUNT_EXPECTED: usize = 34;
#[cfg(feature = "thread_local")]
const DROP_COUNT_EXPECTED: usize = 28;

#[destructor(10)]
extern "C" fn test_d_counts() {
    let c = FINALY_COUNT.load(Ordering::Relaxed);
    if c != FINALY_COUNT_EXPECTED {
        eprintln!("Wrong finaly count {}", c);
        unsafe { libc::_exit(1) };
    }
    let c = DROP_COUNT.load(Ordering::Relaxed);
    if c != DROP_COUNT_EXPECTED {
        eprintln!("Wrong drop count {}", c);
        unsafe { libc::_exit(1) };
    }
}

macro_rules! make_test {
    ($name:ident,$acc:ident, $($att:ident)+ $(,$mut:ident)? $(=>$thread_local:ident)?) => {

        #[test]
        fn $name() {

            #[dynamic($($att),*)]
            $(#[$thread_local])?
            static $($mut)? X0: A = A::new(42);
            assert_eq!($acc!(X0),42);

            #[dynamic($($att),*)]
            $(#[$thread_local])?
            static $($mut)? XPRE: A = A::new(42);

            #[constructor(10)]
            extern fn test_pre() {
                assert_eq!($acc!(XPRE),42);
            }

            $(
            use static_init::{Phased,Phase};
            #[allow(unused)]
            let $thread_local = 0;
            eprintln!("tested");
            assert!(!Phased::phase(&XPRE).intersects(Phase::INITIALIZED)
            , "Assertions of this test are valid as long as the tests are not run in the same thread as the \
                main thread. Please remove the `--test-threads=1` test option");
            )?

            assert_eq!($acc!(XPRE),42);

            #[dynamic($($att),*)]
            $(#[$thread_local])?
            static $($mut)? XCONC: A = A::new(42);

            static START: AtomicBool = AtomicBool::new(false);

            static STARTED: AtomicUsize = AtomicUsize::new(0);

            fn test_conc() {
                STARTED.fetch_add(1,Ordering::Relaxed);
                while START.compare_exchange_weak(true,true,Ordering::Relaxed,Ordering::Relaxed).is_err() {core::hint::spin_loop()}
                assert_eq!($acc!(XCONC),42);
            }
            const NT: usize = 8;
            let mut spawned = vec![];
            for _ in 0..NT {
                spawned.push(spawn(test_conc));
            }
            while STARTED.compare_exchange_weak(NT,NT,Ordering::Relaxed,Ordering::Relaxed).is_ok() {core::hint::spin_loop()}
            START.store(true,Ordering::Relaxed);
            spawned.into_iter().for_each(|t| {assert!(t.join().is_ok());});
        }
    }
}

macro_rules! acc0 {
    ($x:ident) => {
        $x.0
    };
}
macro_rules! accr {
    ($x:ident) => {
        $x.read().0
    };
}

make_test!(lazy, acc0, lazy);
make_test!(lesser_lazy, acc0, lazy);
make_test!(lazy_finalize, acc0, lazy finalize); //F:3
make_test!(lesser_lazy_finalize, acc0, lesser_lazy finalize); //F:3

make_test!(mut_lazy, accr, lazy, mut);
make_test!(lesser_mut_lazy, accr, lazy, mut);
make_test!(mut_lazy_finalize, accr, lazy finalize, mut); //F:3
make_test!(lesser_mut_lazy_finalize, accr, lesser_lazy finalize, mut); //F:3
make_test!(mut_lazy_drop, accr, lazy drop, mut); //D:3
make_test!(lesser_mut_lazy_drop, accr, lesser_lazy drop, mut); //D:3

#[cfg(feature = "thread_local")]
make_test!(thread_local_lazy, acc0, lazy => thread_local);
#[cfg(feature = "thread_local")]
make_test!(thread_local_lazy_finalize, acc0, lazy finalize => thread_local); //F:3+8
#[cfg(feature = "thread_local")]
make_test!(thread_local_lazy_drop, acc0, lazy drop => thread_local); //D:3+8

#[cfg(feature = "thread_local")]
make_test!(thread_local_mut_lazy, accr, lazy, mut => thread_local);
#[cfg(feature = "thread_local")]
make_test!(thread_local_mut_lazy_finalize, accr, lazy finalize, mut => thread_local); //F:3+8
#[cfg(feature = "thread_local")]
make_test!(thread_local_mut_lazy_drop, accr, lazy drop, mut => thread_local); //D: 3+8
