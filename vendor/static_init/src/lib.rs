// Copyright 2021 Olivier Kannengieser
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

//! Safe non const initialized statics and safe mutable statics with unbeatable performance.
//!
//! Also provides code execution at program start-up/exit.
//!
//! # Feature
//!
//! - [x] non const initialized statics.
//! - [x] statics dropped at program exit.
//! - [x] safe mutable lazy statics (locked).
//! - [x] every feature with `no_std` support.
//! - [x] unbeatable performance, can be order of magnitude faster that any other solution.
//! - [x] registration of code execution at program exit without allocation (as opposed to libc::at_exit).
//! - [x] ergonomic syntax.
//! - [x] sound and safe.
//! - [x] on nigtly, `thread_locals` and safe mutable `thread_locals`, guaranteed to be
//!     dropped at thread exit with the lowest possible overhead compared to
//!     what is provided by system library thread support or the standard library!
//!
//! # Fastest Lazy Statics
//!
//! This crate provides *lazy statics* on all plateforms.
//!
//! On unixes and windows *lesser lazy statics* are *lazy* during program startup phase
//! (before `main` is called). Once main is called, those statics are all guaranteed to be
//! initialized and any access to them almost no incur any performance cost
//!
//! ```
//! use static_init::{dynamic};
//!
//! #[dynamic]
//! static L1: Vec<i32> = vec![1,2,3,4,5,6];
//!
//! #[dynamic(drop)]
//! static mut L2: Vec<i32> = {let mut v = L1.clone(); v.push(43); v};
//! ```
//!
//! Those static initialization and access can be 10x faster than
//! what is provided by the standard library or other crates.
//!
//! # Safe Mutable Statics
//!
//! Just add the `mut` keyword to have mutable locked statics.
//!
//! ```
//! use static_init::{dynamic};
//!
//! #[dynamic]
//! static mut L1: Vec<i32> = vec![1,2,3,4,5,6];
//!
//! #[dynamic(drop)]
//! static mut L2: Vec<i32> = {
//!    //get a unique lock:
//!    let mut lock = L1.write();
//!    lock.push(42);
//!    lock.clone()
//!    };
//! ```
//!
//! Those statics use an *apdaptative phase locker* that gives them surprising performance.
//!
//! # Classical Lazy statics
//!
//! By default, initialization of statics declared with the `dynamic` is forced before main
//! start on plateform that support it. If *lazyness* if a required feature, the attribute argument
//! `lazy` can be used.
//!
//! ```rust
//! use static_init::{dynamic};
//!
//! #[dynamic(lazy)]
//! static L1: Vec<i32> = vec![1,2,3,4,5,6];
//!
//! #[dynamic(lazy,drop)]
//! static mut L3: Vec<i32> =L1.clone();
//! ```
//!
//! Even if the static is not mut, dropped statics are always locked. There is also a `finalize` attribute
//! argument that can be used to run a "drop" equivalent at program exit but leaves the static unchanged.
//!
//! Those lazy also provide superior performances compared to other solutions.
//!
//! # `no_std` support
//!
//! On linux or Reddox (TBC) this library is `no_std`. The library use directly the `futex` system call
//! to place thread in a wait queue when needed.
//!
//! On other plateform `no_std` support can be gain by using the `spin_loop` feature. NB that lock strategies
//! based on spin loop are not system-fair and cause entire system slow-down.
//!
//! # Performant
//!
//! ## Under the hood
//!
//! The statics and mutable statics declared with `dynamic` attribute use what we
//! call an  *adaptative phase locker*. This is a lock that is in between a `Once`
//! and a `RwLock`. It is carefully implemented as a variation over the `RwLock`
//! algorithms of `parking_lot` crate with other tradeoff and different
//! capabilities.
//!
//! It is qualified *adaptative* because the decision to take a read lock,
//! a write lock or not to take a lock is performed while the lock attempt is
//! performed and a thread may attempt to get a write lock but decides to be waked
//! as the owner of a read lock if it is about to be placed in a wait queue.
//!
//! Statics and thread locals that need to register themselve for destruction at
//! program or thread exit are implemented as members of an intrusive list. This
//! implementation avoid heap memory allocation caused by system library support
//! (`libc::at_exit`, `glibc::__cxa_at_thread_exit`, pthread... registers use heap
//! memory allocation), and it avoid to fall on system library implementation
//! limits that may cause `thread_locals` declared with `std::thread_locals` not to
//! be dropped.
//!
//! Last but not least of the optimization, on windows and unixes (but not Mac yet)
//! `dynamic` statics initialization is forced before main start. This fact unable
//! a double check with a single boolean for all statics that is much faster other
//! double check solution.
//!
//! ## Benchmark results
//!
//! (see the README file or run benchmark with `cargo bench --feature bench_nightly`)
//!
//! # Thread local support
//!
//! On nightly `thread_local` support can be enable with the feature
//! `thread_local`. The attribute `dynamic` can be used with thread locals as with
//! regular statics. In this case, the mutable `thread_local` will behave similarly
//! to a RefCell with the same syntax as mutable lazy statics.
//!
//! ```rust
//! # #![cfg_attr(feature = "thread_local", feature(thread_local))]
//! # use static_init::{Finaly,dynamic};
//! # #[cfg(feature = "thread_local")]
//! # mod m{
//! # use static_init::{dynamic};
//!
//! #[dynamic(drop)] //guaranteed to be drop: no leak contrarily to std::thread_local
//! #[thread_local]
//! static V: Vec<i32> = vec![1,1,2,3,5];
//!
//! #[dynamic]
//! #[thread_local]
//! static mut W: Vec<i32> = V.clone();
//! # fn main() {
//! assert_ne!(W.read().len(), 0);
//! assert_ne!(W.try_read().unwrap().len(), 0);
//! # }
//! # }
//! ```
//!
//! # Unsafe Low level
//!
//! ## Unchecked statics initiliazed at program start up
//!
//! The library also provides unchecked statics, whose initialization is run before main start. Those statics
//! does not imply any memory overhead neither execution time overhead. This is the responsability of the coder
//! to be sure not to access those static before they are initialized.
//!
//! ```rust
//! use static_init::dynamic;
//!
//! #[dynamic(10)]
//! static A: Vec<i32> = vec![1,2,3];
//!
//! #[dynamic(0,drop)]
//! static mut B: Vec<i32> = unsafe {A.clone()};
//! ```
//!
//! Even if A is not declared mutable, the attribute macro convert it into a mutable static to ensure that every
//! access to it is unsafe.
//!
//! The number indicates the priority, the larger the number, the sooner the static will be initialized.
//!
//! Those statics can also be droped at program exit with the `drop` attribute argument.
//!
//! ## Program constructor destructor
//!
//! It is possible to register fonction for execution before main start/ after main returns.
//!
//!
//! ```rust
//! use static_init::{constructor, destructor};
//!
//! #[constructor(10)]
//! extern "C" fn run_first() {}
//!
//! #[constructor(0)]
//! extern "C" fn then_run() {}
//!
//! #[destructor(0)]
//! extern "C" fn pre_finish() {}
//!
//! #[destructor(10)]
//! extern "C" fn finaly() {}
//! ```
//!
//! # Debug support
//!
//! The feature `debug_order` can be activated to detect trouble with initialization order of raw
//! statics or dead locks due to lazy initialization depending on itself.

// TODO:
//          - bencher les thread locals
//          - revoir la doc
//          - voir si specializer le phase locker pour les cas non mut / mut lazy
//          - renomer new_static => from_generator
//

// Notes on rust rt
//
// On unixes-linux:
//   -args are initialized with .init_array(99)
//   -but also just before main to help miri
// On other unixes
//   - args/env is set just before main start
//    => it will look like there are no arg and no env. => safe.
//
// On unixes
//   - sys::init() :
//      - initialize standard streams
//      - sigpip reset
//
//   - guard actualy does noting just return expect stack size
//   - otherwise map memory and prohibit access with mprotect
//      => so this is not secure and no point in minding about that
//   - then sys::stack_overflow install a signal handler to handle
//     signal => once again not secure no point in minding about that
//   - then set thread_info => so use of thread::current will panic
//   in constructor or destructor
//
// At exit args/env may be cleaned, also stack_guard
// stdio will not be buffered
//
// windows init maybe called at .CRT$XCU
//
//
//
// On windows there are no sys::init(), it does nothing
// no guard,
// args always accessbile
// the same as unix with thread_info
//
#![cfg_attr(
    all(
        not(any(feature = "parking_lot_core", debug_mode)),
        any(target_os = "linux", target_os = "android")
    ),
    no_std
)]
#![cfg_attr(all(elf, feature = "thread_local"), feature(linkage))]
#![cfg_attr(
    feature = "thread_local",
    feature(thread_local),
    feature(cfg_target_thread_local)
)]
#![cfg_attr(docsrs, feature(doc_cfg))]

/// # Details and implementation documentation.
///
/// ## Mac
///   - [MACH_O specification](https://www.cnblogs.com/sunkang/archive/2011/05/24/2055635.html)
///   - GCC source code gcc/config/darwin.c indicates that priorities are not supported.
///
///   Initialization functions pointers are placed in section "__DATA,__mod_init_func" and
///   "__DATA,__mod_term_func"
///
///   std::env is not initialized in any constructor.
///
/// ## ELF plateforms:
///  - `info ld`
///  - linker script: `ld --verbose`
///  - [ELF specification](https://docs.oracle.com/cd/E23824_01/html/819-0690/chapter7-1.html#scrolltoc)
///
///  The runtime will run fonctions pointers of section ".init_array" at startup and function
///  pointers in ".fini_array" at program exit. The linker place in the target object file
///  sectio .init_array all sections from the source objects whose name is of the form
///  .init_array.NNNNN in lexicographical order then the .init_array sections of those same source
///  objects. It does equivalently with .fini_array and .fini_array.NNNN sections.
///
///  Usage can be seen in gcc source gcc/config/pru.c
///
///  Resources of libstdc++ are initialized with priority 65535-100 (see gcc source libstdc++-v3/c++17/default_resource.h)
///  The rust standard library function that capture the environment and executable arguments is
///  executed at priority 65535-99 on gnu platform variants. On other elf plateform they are not accessbile in any constructors. Nevertheless
///  one can read into /proc/self directory to retrieve the command line.
///  Some callbacks constructors and destructors with priority 65535 are
///  registered by rust/rtlibrary.
///  Static C++ objects are usually initialized with no priority (TBC). lib-c resources are
///  initialized by the C-runtime before any function in the init_array (whatever the priority) are executed.
///
/// ## Windows
///
///   std::env is initialized before any constructors.
///
///  - [this blog post](https://www.cnblogs.com/sunkang/archive/2011/05/24/2055635.html)
///
///  At start up, any functions pointer between sections ".CRT$XIA" and ".CRT$XIZ"
///  and then any functions between ".CRT$XCA" and ".CRT$XCZ". It happens that the C library
///  initialization functions pointer are placed in ".CRT$XIU" and C++ statics functions initialization
///  pointers are placed in ".CRT$XCU". At program finish the pointers between sections
///  ".CRT$XPA" and ".CRT$XPZ" are run first then those between ".CRT$XTA" and ".CRT$XTZ".
///
///  Some reverse engineering was necessary to find out a way to implement
///  constructor/destructor priority.
///
///  Contrarily to what is reported in this blog post, msvc linker
///  only performs a lexicographicall ordering of section whose name
///  is of the form "\<prefix\>$\<suffix\>" and have the same \<prefix\>.
///  For example "RUST$01" and "RUST$02" will be ordered but those two
///  sections will not be ordered with "RHUM" section.
///
///  Moreover, it seems that section name of the form \<prefix\>$\<suffix\> are
///  not limited to 8 characters.
///
///  So static initialization function pointers are placed in section ".CRT$XCU" and
///  those with a priority `p` in `format!(".CRT$XCTZ{:05}",65535-p)`. Destructors without priority
///  are placed in ".CRT$XPU" and those with a priority in `format!(".CRT$XPTZ{:05}",65535-p)`.
///
///
mod details {}

use core::cell::Cell;

/// A trait for objects that are intinded to transition between phasis.
///
/// A type that implement [`Sequential`] ensured that its `data` will traverse a sequence of
/// [phases](Phase). The phase does not participates to the value of the type. The phase describes
/// the *lifetime* of the object: initialized, droped,...
///
/// # Safety
///
/// The trait is unsafe because the implementor should ensure that the reference returned by
/// [`sequentializer`](Self::sequentializer) and the reference returned by [`data`](Self::data) refer to two subobject of a same object.
///
unsafe trait Sequential {
    type Data;
    type Sequentializer;
    fn sequentializer(this: &Self) -> &Self::Sequentializer;
    fn data(this: &Self) -> &Self::Data;
    fn sequentializer_data_mut(this: &mut Self) -> (&mut Self::Sequentializer, &mut Self::Data);
}

/// Trait for objects that know in which [phase](Phase) they are.
pub trait Phased {
    /// return the current phase
    fn phase(this: &Self) -> Phase;
}

/// A type that implement Sequentializer aims at [phase](Phase) sequencement.
///
/// The method [`Sequential::sequentializer`] should return an object that implement
/// this trait.
///
/// # Safety
///
/// The trait is unsafe because the lock should ensure the following lock semantic:
///  - if the implementor also implement Sync, the read/write lock semantic should be synchronized
///  and if no lock is taken, the call to lock should synchronize with the end of the phase
///  transition that put the target object in its current phase.
///  - if the implementor is not Sync then the lock should panic if any attempt is made
///    to take another lock while a write lock is alive or to take a write lock while there
///    is already a read_lock.(the lock should behave as a RefCell).
unsafe trait Sequentializer<'a, T: Sequential + 'a>: Sized + Phased {
    type ReadGuard;
    type WriteGuard;
    /// Lock the phases of an object in order to ensure atomic phase transition.
    ///
    /// The nature of the lock depend on the phase in which is the object, and is determined
    /// by the `lock_nature` argument.
    fn lock(
        target: &'a T,
        lock_nature: impl Fn(Phase) -> LockNature,
        hint: Phase,
    ) -> LockResult<Self::ReadGuard, Self::WriteGuard>;
    /// Try to lock the phases of an object in order to ensure atomic phase transition.
    ///
    /// The nature of the lock depend on the phase in which is the object, and is determined
    /// by the `lock_nature` argument. If it is impossible to lock because of another lock
    /// the result is None.
    fn try_lock(
        target: &'a T,
        lock_nature: impl Fn(Phase) -> LockNature,
        hint: Phase,
    ) -> Option<LockResult<Self::ReadGuard, Self::WriteGuard>>;

    /// Lock the phases of an object in order to ensure atomic phase transition.
    fn lock_mut(target: &'a mut T) -> Self::WriteGuard;
}

/// A [`LazySequentializer`] sequentialize the [phases](Phase) of a target object to ensure
/// atomic initialization and finalization.
///
/// # Safety
///
/// The trait is unsafe because the implementor must ensure that:
///
///  - if the implementor also implement Sync, the read/write lock semantic should be synchronized
///  and if no lock is taken, the call to lock should synchronize with the end of the phase
///  transition that put the target object in its current phase.
///  - if the implementor is not Sync then the lock should panic if any attempt is made
///    to take another lock while a write lock is alive or to take a write lock while there
///    is already a read_lock.(the lock should behave as a RefCell).
unsafe trait LazySequentializer<'a, T: Sequential + 'a>: Sequentializer<'a, T> {
    const INITIALIZED_HINT: Phase;
    /// if `shall_init` return true for the target [`Sequential`] object, it initialize
    /// the data of the target object using `init`
    ///
    /// The implementor may also proceed to registration of the finalizing method (drop)
    /// in order to drop the object on the occurence of singular event (thread exit, or program
    /// exit). If this registration fails and if `init_on_reg_failure` is `true` then the object
    /// will be initialized, otherwise it will not.
    fn init(
        target: &'a T,
        shall_init: impl Fn(Phase) -> bool,
        init: impl FnOnce(&'a <T as Sequential>::Data),
    ) -> Phase;
    //fn init_unique(
    //    target: &'a mut T,
    //    shall_init: impl Fn(Phase) -> bool,
    //    init: impl FnOnce(&'a <T as Sequential>::Data),
    //) -> Phase;
    /// Similar to [init](Self::init) but returns a lock that prevents the phase of the object
    /// to change (Read Lock). The returned lock may be shared.
    fn init_then_read_guard(
        target: &'a T,
        shall_init: impl Fn(Phase) -> bool,
        init: impl FnOnce(&'a <T as Sequential>::Data),
    ) -> Self::ReadGuard;
    /// Similar to [init](Self::init) but returns a lock that prevents the phase of the object
    /// to change accepts through the returned lock guard (Write Lock). The lock is exculisive.
    fn init_then_write_guard(
        target: &'a T,
        shall_init: impl Fn(Phase) -> bool,
        init: impl FnOnce(&'a <T as Sequential>::Data),
    ) -> Self::WriteGuard;
    /// Similar to [init_then_read_guard](Self::init_then_read_guard) but will return None
    /// if any lock is taken on the lazy or if it is beiing initialized
    fn try_init_then_read_guard(
        target: &'a T,
        shall_init: impl Fn(Phase) -> bool,
        init: impl FnOnce(&'a <T as Sequential>::Data),
    ) -> Option<Self::ReadGuard>;
    /// Similar to [init_then_write_guard](Self::init_then_write_guard) but will return None
    /// if any lock is taken on the lazy or if it is beiing initialized
    fn try_init_then_write_guard(
        target: &'a T,
        shall_init: impl Fn(Phase) -> bool,
        init: impl FnOnce(&'a <T as Sequential>::Data),
    ) -> Option<Self::WriteGuard>;
}

trait UniqueLazySequentializer<T: Sequential> {
    /// if `shall_init` return true for the target [`Sequential`] object, it initialize
    /// the data of the target object using `init`
    ///
    /// The implementor may also proceed to registration of the finalizing method (drop)
    /// in order to drop the object on the occurence of singular event (thread exit, or program
    /// exit). If this registration fails and if `init_on_reg_failure` is `true` then the object
    /// will be initialized, otherwise it will not.
    fn init_unique(
        target: &mut T,
        shall_init: impl Fn(Phase) -> bool,
        init: impl FnOnce(&mut <T as Sequential>::Data),
    ) -> Phase;
}

/// A [FinalizableLazySequentializer] sequentialize the [phase](Phase) of an object to
/// ensure atomic initialization and finalization.
///
/// A sequentializer that implement this trait is not able to register the finalization
/// for latter call on program exit or thread exit.
///
/// # Safety
///
/// The trait is unsafe because the implementor must ensure that:
///
///  - either the implementor is Sync and the initialization is performed atomically
///  - or the implementor is not Sync and any attempt to perform an initialization while
///    an initialization is running will cause a panic.
unsafe trait FinalizableLazySequentializer<'a, T: 'a + Sequential>:
    Sequentializer<'a, T>
{
    /// if `shall_init` return true for the target [`Sequential`] object, it initialize
    /// the data of the target object using `init`
    ///
    /// Before initialization of the object, the fonction `reg` is call with the target
    /// object as argument. This function should proceed to registration of the
    /// [finalize_callback](Self::finalize_callback) for latter call at program exit or
    /// thread exit.
    fn init(
        target: &'a T,
        shall_init: impl Fn(Phase) -> bool,
        init: impl FnOnce(&'a <T as Sequential>::Data),
        reg: impl FnOnce(&'a T) -> bool,
    ) -> Phase;
    /// Similar to [init](Self::init) but returns a lock that prevents the phase of the object
    /// to change (Read Lock). The returned lock may be shared.
    fn init_then_read_guard(
        target: &'a T,
        shall_init: impl Fn(Phase) -> bool,
        init: impl FnOnce(&'a <T as Sequential>::Data),
        reg: impl FnOnce(&'a T) -> bool,
    ) -> Self::ReadGuard;
    /// Similar to [init](Self::init) but returns a lock that prevents the phase of the object
    /// to change accepts through the returned lock guard (Write Lock). The lock is exculisive.
    fn init_then_write_guard(
        target: &'a T,
        shall_init: impl Fn(Phase) -> bool,
        init: impl FnOnce(&'a <T as Sequential>::Data),
        reg: impl FnOnce(&'a T) -> bool,
    ) -> Self::WriteGuard;
    /// Similar to [init_then_read_guard](Self::init_then_read_guard) but will return None
    /// if any lock is taken on the lazy or if it is beiing initialized
    fn try_init_then_read_guard(
        target: &'a T,
        shall_init: impl Fn(Phase) -> bool,
        init: impl FnOnce(&'a <T as Sequential>::Data),
        reg: impl FnOnce(&'a T) -> bool,
    ) -> Option<Self::ReadGuard>;
    /// Similar to [init_then_write_guard](Self::init_then_write_guard) but will return None
    /// if any lock is taken on the lazy or if it is beiing initialized
    fn try_init_then_write_guard(
        target: &'a T,
        shall_init: impl Fn(Phase) -> bool,
        init: impl FnOnce(&'a <T as Sequential>::Data),
        reg: impl FnOnce(&'a T) -> bool,
    ) -> Option<Self::WriteGuard>;
    /// A callback that is intened to be stored by the `reg` argument of `init` method.
    fn finalize_callback(s: &'a T, f: impl FnOnce(&'a T::Data));
}

pub trait GeneratorTolerance {
    const INIT_FAILURE: bool;
    const FINAL_REGISTRATION_FAILURE: bool;
}

/// Generates a value of type `T`
pub trait Generator<T>: GeneratorTolerance {
    fn generate(&self) -> T;
}

impl<U, T: Fn() -> U> Generator<U> for T {
    fn generate(&self) -> U {
        self()
    }
}

impl<U, T: Fn() -> U> GeneratorTolerance for T {
    const INIT_FAILURE: bool = true;
    const FINAL_REGISTRATION_FAILURE: bool = false;
}

impl<U, T: FnOnce() -> U> Generator<U> for Cell<Option<T>> {
    fn generate(&self) -> U {
        match self.take() {
            Some(v) => v(),
            None => panic!("Cannot call this generator twice"),
        }
    }
}

impl<U, T: FnOnce() -> U> GeneratorTolerance for Cell<Option<T>> {
    const INIT_FAILURE: bool = false;
    const FINAL_REGISTRATION_FAILURE: bool = false;
}

/// Trait that must be implemented by #[dynamic(finalize)] statics.
pub trait Finaly {
    /// This method is called when program or thread exit and the lazy
    /// was initialized
    fn finaly(&self);
}

/// Trait that must be implemented by #[dynamic(prime)] mutable statics.
pub trait Uninit {
    /// This method is called when program or thread exit and the lazy
    /// was initialized
    ///
    /// It should leave the target objet in a valid state as it could
    /// be accessed throud `primed_<read|write>` method familly.
    fn uninit(&mut self);
}

#[cfg_attr(docsrs, doc(cfg(debug_mode)))]
#[cfg(debug_mode)]
#[doc(hidden)]
#[derive(Debug)]
/// Used to passe errors
pub struct CyclicPanic;

/// phases and bits to manipulate them;
pub mod phase {

    use core::fmt::{self, Display, Formatter};

    use bitflags::bitflags;
    #[cfg(not(feature = "spin_loop"))]
    pub(crate) const WRITE_WAITER_BIT: u32 = 0b1000_0000_0000_0000_0000_0000_0000_0000;
    #[cfg(not(feature = "spin_loop"))]
    pub(crate) const READ_WAITER_BIT: u32 = 0b0100_0000_0000_0000_0000_0000_0000_0000;
    pub(crate) const LOCKED_BIT: u32 = 0b0010_0000_0000_0000_0000_0000_0000_0000;
    pub(crate) const READER_BITS: u32 = 0b0000_1111_1111_1111_1111_1111_0000_0000;
    pub(crate) const READER_OVERF: u32 = 0b0001_0000_0000_0000_0000_0000_0000_0000;
    pub(crate) const READER_UNITY: u32 = 0b0000_0000_0000_0000_0000_0001_0000_0000;
    #[cfg(not(feature = "spin_loop"))]
    pub(crate) const MAX_WAKED_READERS: usize = (READER_OVERF / READER_UNITY) as usize;
    // Although some flags exclude others, Phase is represented by
    // a bitflag to allow xor bit tricks that eases atomic phase
    // changes in the implementation of SyncPhaseLocker.
    bitflags! {
        /// The lifetime phase of an object, this indicate weither the object was initialized
        /// finalized (droped),...
        ///
        /// The registration phase is a phase that precede the initialization phase and is meant
        /// to register a callback that will proceed to the finalization (drop) of the object at
        /// program exit or thread exit. Depending on the plateform this registration may fail.
        pub struct Phase: u32 {
            const INITIALIZED               = 0b0000_0000_0000_0000_0000_0000_0000_0001;
            const INITIALIZATION_PANICKED   = 0b0000_0000_0000_0000_0000_0000_0000_0010;
            const INITIALIZATION_SKIPED     = 0b0000_0000_0000_0000_0000_0000_0000_0100;


            const REGISTERED                = 0b0000_0000_0000_0000_0000_0000_0000_1000;
            const REGISTRATION_PANICKED     = 0b0000_0000_0000_0000_0000_0000_0001_0000;
            const REGISTRATION_REFUSED      = 0b0000_0000_0000_0000_0000_0000_0010_0000;

            const FINALIZED                 = 0b0000_0000_0000_0000_0000_0000_0100_0000;
            const FINALIZATION_PANICKED     = 0b0000_0000_0000_0000_0000_0000_1000_0000;

            const INITIALIZED_AND_REGISTERED     = Self::INITIALIZED.bits | Self::REGISTERED.bits;
        }
    }

    impl Display for Phase {
        fn fmt(&self, ft: &mut Formatter<'_>) -> fmt::Result {
            if self.is_empty() {
                write!(ft, "Phase (not initialized)")?;
            } else {
                write!(ft, "Phase (")?;
                let mut is_first = true;
                let mut write = |s| {
                    if is_first {
                        is_first = false;
                        ft.write_str(s)
                    } else {
                        write!(ft, " | {}", s)
                    }
                };
                if self.intersects(Phase::INITIALIZED) {
                    write("Initialized")?;
                }
                if self.intersects(Phase::INITIALIZATION_PANICKED) {
                    write("Initialization panicked")?;
                }
                if self.intersects(Phase::INITIALIZATION_SKIPED) {
                    write("Initialization skiped")?;
                }
                if self.intersects(Phase::REGISTERED) {
                    write("Registered")?;
                }
                if self.intersects(Phase::REGISTRATION_PANICKED) {
                    write("Registration panicked")?;
                }
                if self.intersects(Phase::REGISTRATION_REFUSED) {
                    write("Registration refused")?;
                }
                if self.intersects(Phase::FINALIZED) {
                    write("Finalized")?;
                }
                if self.intersects(Phase::FINALIZATION_PANICKED) {
                    write("Finalization panicked")?;
                }
                write!(ft, ")")?
            }
            Ok(())
        }
    }
}
#[doc(inline)]
pub use phase::Phase;

/// Attribute for functions run at program initialization (before main).
///
/// ```
/// # use static_init::constructor;
/// #[constructor]
/// extern "C" fn initer () {
/// // run before main start
/// }
/// ```
///
/// The execution order of constructors is unspecified. Nevertheless on ELF plateform (linux, any unixes but mac) and
/// windows plateform a priority can be specified using the syntax `constructor(<num>)` where
/// `<num>` is a number included in the range [0 ; 2<sup>16</sup>-1].
///
/// Constructors with a priority of 65535 are run first (in unspecified order), then constructors
/// with priority 65534 are run ...  then constructors
/// with priority number 0
///
/// An abscence of priority is equivalent to a priority of 0.
///
/// ```
/// # use static_init::constructor;
/// #[constructor(0)]
/// extern "C" fn first () {
/// // run before main start
/// }
///
/// #[constructor(1)]
/// extern "C" fn then () {
/// // run before main start
/// }
/// ```
///
///
/// # Safety
///
/// Any access to *raw statics* with an equal or lower
/// initialization priority will cause undefined behavior. (NB: usual static data and *lazy
/// statics* are always safe to access.
///
/// # About rust standard library runtime
///
/// During program constructions some functionnality of the standard library will be missing on
/// unixes:
///
/// - program argument as returned by `std::env::args` and environment variables as returned by
/// `std::env::vars` will be emty on unixes other than linux-gnu. On linux/gnu they will be
///  empty above priority
///  65436.
///
/// - call to `std::thread::current` will panick.
///
/// - standard streams may not be initialized.
///
/// - Some signal handler installed by the standard library may not be installed
///
/// On windows all the standard library should appear initialized.
///
/// # Constructor signature
///
/// Constructor function should have type `extern "C" fn() -> ()`.
///
/// On plateform where the program is linked
/// with the gnu variant of libc (which covers all gnu variant platforms) constructor functions
/// can take (or not) `argc: i32, argv: **const u8, env: **const u8` as arguments.
/// `argc` is the size of the argv
/// sequence, `argv` and `env` both refer to null terminated contiguous sequence of pointer
/// to c-string (c-strings are null terminated sequence of u8).
///
/// Also after the null terminating `*const * const u8` of the environment variable list is found
/// the auxilary vector that are information provided by the kernel. It is possible to retrieve
/// from that vector information about the process and location of syscalls implemented in the
/// vdso.
/// ```
/// # use static_init::constructor;
/// # #[cfg(all(linux, target_env = "gnu"))]
/// # mod m {
/// #[constructor]
/// extern "C" fn get_args_env(argc: i32, mut argv: *const *const u8, env: *const *const u8) {}
/// # }
pub use static_init_macro::constructor;

/// Attribute for functions run at program termination (after main)
///
/// ```
/// # use static_init::destructor;
/// #[destructor]
/// extern "C" fn droper () {
/// // run after main return
/// }
/// ```
///
/// The execution order of destructors is unspecified. Nevertheless on ELF plateform (linux,any unixes but mac) and
/// windows plateform a priority can be specified using the syntax `destructor(<num>)` where
/// `<num>` is a number included in the range [0 ; 2<sup>16</sup>-1].
///
/// Destructors with priority 0 are run first (in unspecified order),
/// then destructors with priority number 1,... finaly destructors with priority 65535 are run.
///
/// An abscence of priority is equivalent to a priority of 0.
///
/// ```
/// # use static_init::destructor;
/// #[destructor(1)]
/// extern "C" fn first () {
/// // run after main return
/// }
///
/// #[destructor(0)]
/// extern "C" fn then () {
/// // run after main return
/// }
/// ```
/// # About rust runtime
///
/// After main exit the standard streams are not buffered.
///
/// # Destructor signature
///
/// Destructor function should have type `unsafe extern "C" fn() -> ()`.
pub use static_init_macro::destructor;

/// Declare statics that can be initialized with non const fonctions and safe mutable statics
///
/// Statics on which this attribute is applied will be be initialized at run time (optionaly see
/// bellow), before main start. This allow statics initialization with non const expressions.
///
/// There are two main categories of statics:
///
/// - *lazy statics* which are statics that are always safe to use. They may be initialized on
/// first acces or before main is called;
///
/// - *locked lazy statics* which are the mutable version of lazy statics.
///
/// - *raw statics*, which are initialized at program start-up and absolutely unchecked. Any
/// access to them requires `unsafe` block;
///
/// # Lazy statics
///
/// Those statics are initialized on first access. An optimization implemented by *lesser lazy statics*
/// initialize the static before main is called on all tier1 plateform but mach.
///
/// The declared object is encapsulated in a type that implement `Deref`.
///
/// Other access functionnality and state information are accessible through the `LazyAccess`
/// trait.
///
/// Those lazy can be used with regular statics and thread locals.
///
/// ```
/// # #![cfg_attr(feature = "thread_local",feature(thread_local))]
/// # use static_init::dynamic;
///
/// #[dynamic]
/// static A :Vec<i32> = vec![1,2];
///
/// # #[cfg(feature = "thread_local")]
/// # mod m {
/// # use static_init::dynamic;
/// #[dynamic]
/// #[thread_local]
/// static TL :Vec<i32> = vec![1,2];
/// # }
///
/// ```
///
/// ## Lesser Lazy Statics
///
/// They are declared with the `#[dynamic]` attribute (or equivalently `#[dynamic(lesser_lazy)]`.
/// They are either initialized on first access or before main is called. They provide
/// unsurpassable access performance: their access time is comparable to const initialized statics
/// but they support non const initialization:
///
/// ```
/// # use static_init::dynamic;
/// #[dynamic]
/// static V :Vec<i32> = vec![1,2];
///
/// assert_eq!(V.len(), 2);
/// ```
///
/// ## Realy lazy Statics
///
/// When initialization on first access is a requirement, the static shall be attributed with
/// `#[dynamic(lazy)]`
///
/// ```
/// # use static_init::dynamic;
/// #[dynamic(lazy)]
/// static V :Vec<i32> = vec![1,2];
///
/// assert_eq!(*V, vec![1,2]);
/// ```
///
/// ## Finalized statics
///
/// The attribute argument `finalize` can be used if the declared type of
/// the static implement `Finaly` trait. The finalize method is called at
/// program exit or at thread exit for thread locals. (NB: mutable lazy also
/// support drop, see below)
///
/// ```
/// # #![cfg_attr(feature = "thread_local", feature(thread_local))]
/// # use static_init::{Finaly,dynamic};
///
/// # fn main(){}
///
/// struct A(i32);
///
/// impl Finaly for A {
///     fn finaly(&self){/* some clean up code */ }
/// }
///
/// #[dynamic(finalize)] //finalize execute at program exit
/// static X :A = A(33);
///
/// # #[cfg(feature="thread_local")]
/// # mod m{
/// # use static_init::{dynamic};
/// # use super::A;
/// #[dynamic(lazy,finalize)] //finalize executed at thread exit
/// #[thread_local]
/// static Y :A = A(33);
/// # }
/// ```
///
/// ## Tolerances
///
/// ### Initialization fault tolerance
///
/// By default if the initialization of a lazy panic, initialization will be attempted
/// once again on the next access attempt. If this is not desired the lazy should be declared
/// with attribute argument `try_init_once`, in which case, the lazy will be poisonned if
/// initialization panics.
///
/// ```
/// # use static_init::{dynamic};
/// #[dynamic(try_init_once)]
/// static X :Vec<i32> = vec![1,2];
///
/// #[dynamic(lazy,try_init_once)] //attribute argument can be combined
/// static Y :Vec<i32> = vec![1,2];
/// ```
///
/// ### Registration for finalization tolerance
///
/// By default lazy that intended to be finalized (because they use the `finalize` or `drop`
/// attribute argument) refuse to initialize if registration of the finalization or drop at
/// program exit or thread exit fails.
///
/// If this is not desired, the `tolerate_leak` attribute argument can be used.
///
/// ```
/// # use static_init::{Finaly,dynamic};
/// struct A(i32);
///
/// impl Finaly for A {
///     fn finaly(&self){/* some clean up code */ }
/// }
///
/// #[dynamic(finalize,tolerate_leak)]
/// static X :A = A(21);
/// //the initialization may succeed even if it is impossible to register
/// //the call to finaly at program exit
/// ```
///
/// # Locked lazy statics
///
/// Those statics are mutable statics, initialized on the first acces and protected behind
/// a kind of read/write lock specialy designed for them.
///
/// The are declared as *lazy statics* but with the `mut` keyword. The macro will actualy remove
/// the `mut` keyword and use a r/w locked wrapper type:
///
/// ```
/// # use static_init::{Finaly,dynamic};
///
/// #[dynamic]
/// static mut V: Vec<i32> = vec![1,2];
///
/// V.write().push(3);
///
/// assert_eq!(*V.read(), vec![1,2,3]);
/// ```
///
/// Those statics provides different methods to access the target object. See the documentation of
/// [LockedLazy] for exemple. All *locked lazy* types provide the same methods.
///
/// Locked lazy statics support all attribute arguments supported by *lazy statics*: `finalize`,
/// `try_init_once`, `tolerate_leak`. Moreover they support two other arguments:
///
/// - `drop` in which case the static will be dropped at program exit:
/// - `prime` which is a static that support access before it is actualy initialized
/// and after it is droped;
///
/// ## Dropped locked lazy statics
///
/// Locked lazy statics can be droped at program exit or thread exit when declared with
/// the `drop` attribute argument
///
/// ```
/// # #![cfg_attr(feature = "thread_local", feature(thread_local))]
/// # use static_init::{Finaly,dynamic};
///
/// # fn main(){}
///
/// # #[cfg(feature="thread_local")]
/// # mod m{
/// # use static_init::{dynamic};
///
/// #[dynamic(drop)]
/// #[thread_local]
/// static mut V: Vec<i32> = vec![1,2];
/// # }
///
/// #[dynamic(lazy,drop,tolerate_leak)]
/// static mut V2: Vec<i32> = vec![1,2];
/// ```
///
/// ## Primed locked lazy statics
///
/// Those statics model the case where an object should have a
/// standard behavior and a fallback behavior after ressources
/// are release or not yet acquired.
///
/// Those statics are initialized in two steps:
///
/// - a const initialization that happens at compile time
///
/// - a dynamic intialization that happens the first time they are accessed if
/// if is declared with `lazy` attribute argument or just before.
///
/// More over they are conceptualy uninitialized if the type of the statics
/// implement the `Uninit` trait and is declared with the `drop` attribute argument.
///
/// They must be initialized with a match expression as exemplified bellow:
///
/// ```
/// use static_init::{dynamic, Uninit};
///
/// #[dynamic(prime)]
/// static mut O: Option<Vec<i32>> = match INIT {
///     PRIME => None,
///     DYN => Some(vec![1,2]),
///     };
///
/// #[dynamic(lazy,prime)]
/// static mut OLAZY: Option<Vec<i32>> = match INIT {
///     PRIME => None,
///     DYN => Some(vec![1,2]),
///     };
///
///
/// struct A(Option<Vec<i32>>);
///
/// impl Uninit for A {
///     fn uninit(&mut self) {
///         self.0.take();
///     }
/// }
///
/// #[dynamic(prime,finalize)]//finalize/drop actualy means uninit for primed lazy
/// static mut P: A = match INIT {
///     PRIME => A(None),
///     DYN => A(Some(vec![1,2])),
///     };
///
/// match P.primed_read() {
///     Ok(read_lock) => (),/*a read lock that refers to the initialized statics */
///     Err(read_lock) => (),/* post finalization access, uninit has already been called*/
///     }
///
/// match P.primed_write() {
///     Ok(write_lock) => (),/*a write lock that refers to the initialized statics */
///     Err(read_lock) => (),/* post finalization access, uninit has already been called*/
///     }
/// ```
///
/// # Raw statics
///
/// Those statics will be initialized at program startup, without ordering, accept between those
/// that have different priorities on plateform that support priorities. Those statics are
/// supported on unixes and windows with priorities and mac without priorities.
///
/// ## Safety
///
/// During initialization, any access to other
/// "dynamic" statics initialized with a lower priority will cause undefined behavior. Similarly,
/// during drop any access to a "dynamic" static dropped with a lower priority will cause undefined
/// behavior. For this reason those statics are always turn into mutable statics to ensure that all
/// access attempt is unsafe.
///
/// Those statics are interesting only to get the optimalest performance at the price of unsafety.
///
/// ```
/// # use static_init::dynamic;
/// #[dynamic(0)]
/// static V :Vec<i32> = vec![1,2];
///
/// assert!(unsafe{*V == vec![1,2]})
/// ```
///
/// ## Execution Order
///
/// The execution order of raw static initializations is unspecified. Nevertheless on ELF plateform (linux,any unixes but mac) and
/// windows plateform a priority can be specified using the syntax `dynamic(<num>)` where
/// `<num>` is a number included in the range [0 ; 2<sup>16</sup>-1].
///
/// Statics with priority number 65535 are initialized first (in unspecified order), then statics
/// with priority number 65534 are initialized ...  then statics
/// with priority number 0.
///
/// ```
/// # use static_init::dynamic;
/// //V1 must be initialized first
/// //because V2 uses the value of V1.
///
/// #[dynamic(20)]
/// static mut V1 :Vec<i32> = vec![1,2];
///
/// #[dynamic(10)]
/// static V2 :Vec<i32> = unsafe{V1.push(3); V1.clone()};
/// ```
///
/// ## Drop
///
/// Those statics can use the `drop` attribute argument. In this case
/// the static will be droped at program exit
///
/// ```
/// # use static_init::dynamic;
/// #[dynamic(0, drop)]
/// static mut V1 :Vec<i32> = vec![1,2];
/// ```
///
/// The drop priority can be specified with the `drop=<priority>` syntax. If no priority
/// is given, the drop priority will equal the one of the initialization priority.
///
/// ```
/// # use static_init::dynamic;
///
/// #[dynamic(10, drop)] //equivalent to #[dynamic(10,drop=10)]
/// //or longer #[dynamic(init=10,drop=10)]
/// static mut V1 :Vec<i32> = vec![1,2];
///
/// #[dynamic(42, drop=33)]
/// static mut V2 :Vec<i32> = vec![1,2];
/// ```
///
/// The drop priorities are sequenced in the reverse order of initialization priority. The smaller
/// is the priority the sooner is droped the static.
///
/// Finaly the `drop_only=<priority>` is equivalent to `#[dynamic(0,drop=<priority>)]` except that the
/// static will be const initialized.
///
/// ```
/// # use static_init::dynamic;
/// struct A;
/// impl Drop for A {
///   fn drop(&mut self) {}
///   }
///
/// #[dynamic(drop_only=33)]
/// static V2: A = A;
/// ```
pub use static_init_macro::dynamic;

/// Provides PhaseLockers, that are phase tagged *adaptative* read-write lock types: during the lock loop, the nature of the lock that
/// is attempted to be taken variates depending on the phase.
///
/// The major difference with a RwLock is that decision to read lock, to write lock or not to lock
/// is taken within the lock loop: on each attempt to take the lock,
/// the PhaseLocker may change its locking strategy or abandon any further attempt to take the lock.
mod phase_locker;
use phase_locker::{LockNature, LockResult};

/// Provides two lazy sequentializers, one that is Sync, and the other that is not Sync, that are
/// able to sequentialize the target object initialization but cannot register its finalization
/// callback.
mod lazy_sequentializer;

#[cfg(any(elf, mach_o, coff))]
/// Provides two lazy sequentializers, one that will finalize the target object at program exit and
/// the other at thread exit.
mod exit_sequentializer;

/// Provides policy types for implementation of various lazily initialized types.
mod generic_lazy;

#[doc(inline)]
pub use generic_lazy::AccessError;

/// Provides various implementation of lazily initialized types
pub mod lazy;
#[doc(inline)]
pub use lazy::{Lazy, LazyAccess, LockedLazy};
#[doc(inline)]
pub use lazy::{UnSyncLazy, UnSyncLockedLazy};

#[cfg(any(elf, mach_o, coff))]
/// Provides types for statics that are meant to run code before main start or after it exit.
pub mod raw_static;

#[derive(Debug)]
#[doc(hidden)]
pub enum InitMode {
    Const,
    Lazy,
    LesserLazy,
    ProgramConstructor(u16),
}

#[derive(Debug)]
#[doc(hidden)]
pub enum FinalyMode {
    None,
    Drop,
    Finalize,
    ProgramDestructor(u16),
}

#[derive(Debug)]
#[doc(hidden)]
pub struct StaticInfo {
    pub variable_name: &'static str,
    pub file_name: &'static str,
    pub line: u32,
    pub column: u32,
    pub init_mode: InitMode,
    pub drop_mode: FinalyMode,
}

#[cfg(all(feature = "lock_statistics", not(feature = "spin_loop")))]
pub use phase_locker::LockStatistics;
