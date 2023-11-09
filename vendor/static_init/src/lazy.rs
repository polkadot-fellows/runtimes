use crate::phase_locker::{
    SyncPhaseGuard, SyncPhaseLocker, SyncReadPhaseGuard, UnSyncPhaseGuard, UnSyncPhaseLocker,
    UnSyncReadPhaseGuard,
};
use crate::{
    generic_lazy::{
        self, AccessError, DropedUnInited, GenericLazy, GenericLockedLazy, LazyData, LazyPolicy,
        Primed, UnInited,
    },
    lazy_sequentializer::UnSyncSequentializer,
    Finaly, Generator, GeneratorTolerance, Phase, Phased, StaticInfo, Uninit,
};

#[cfg(feature = "thread_local")]
use crate::exit_sequentializer::ThreadExitSequentializer;

use crate::{exit_sequentializer::ExitSequentializer, lazy_sequentializer::SyncSequentializer};

use core::cell::Cell;
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};

struct InitializedChecker<T>(PhantomData<T>);

impl<Tol: GeneratorTolerance> LazyPolicy for InitializedChecker<Tol> {
    #[inline(always)]
    fn shall_init(p: Phase) -> bool {
        if Tol::INIT_FAILURE {
            !p.intersects(Phase::INITIALIZED)
        } else {
            p.is_empty()
        }
    }
    #[inline(always)]
    fn is_accessible(p: Phase) -> bool {
        p.intersects(Phase::INITIALIZED)
    }
    #[inline(always)]
    fn post_init_is_accessible(p: Phase) -> bool {
        if Tol::INIT_FAILURE {
            Self::initialized_is_accessible(p)
        } else {
            Self::is_accessible(p)
        }
    }
    #[inline(always)]
    fn initialized_is_accessible(_: Phase) -> bool {
        true
    }
}

struct InitializedSoftFinalizedCheckerGeneric<T, const REG_ALWAYS: bool>(PhantomData<T>);

impl<Tol: GeneratorTolerance, const REG_ALWAYS: bool> LazyPolicy
    for InitializedSoftFinalizedCheckerGeneric<Tol, REG_ALWAYS>
{
    #[inline(always)]
    fn shall_init(p: Phase) -> bool {
        if Tol::INIT_FAILURE {
            !p.intersects(Phase::INITIALIZED)
        } else {
            p.is_empty()
        }
    }
    #[inline(always)]
    fn is_accessible(p: Phase) -> bool {
        p.intersects(Phase::INITIALIZED)
    }
    #[inline(always)]
    fn post_init_is_accessible(p: Phase) -> bool {
        if Tol::INIT_FAILURE && (REG_ALWAYS || Tol::FINAL_REGISTRATION_FAILURE) {
            debug_assert!(!REG_ALWAYS || p.intersects(Phase::REGISTERED));
            Self::initialized_is_accessible(p)
        } else {
            Self::is_accessible(p)
        }
    }
    #[inline(always)]
    fn initialized_is_accessible(_: Phase) -> bool {
        true
    }
}

struct InitializedHardFinalizedCheckerGeneric<T, const REG_ALWAYS: bool>(PhantomData<T>);

impl<Tol: GeneratorTolerance, const REG_ALWAYS: bool> LazyPolicy
    for InitializedHardFinalizedCheckerGeneric<Tol, REG_ALWAYS>
{
    #[inline(always)]
    fn shall_init(p: Phase) -> bool {
        if Tol::INIT_FAILURE {
            !p.intersects(Phase::INITIALIZED)
        } else {
            p.is_empty()
        }
    }
    #[inline(always)]
    fn is_accessible(p: Phase) -> bool {
        p.intersects(Phase::INITIALIZED) && Self::initialized_is_accessible(p)
    }
    #[inline(always)]
    fn post_init_is_accessible(p: Phase) -> bool {
        if Tol::INIT_FAILURE && (REG_ALWAYS || Tol::FINAL_REGISTRATION_FAILURE) {
            debug_assert!(!REG_ALWAYS || p.intersects(Phase::REGISTERED));
            Self::initialized_is_accessible(p)
        } else {
            Self::is_accessible(p)
        }
    }
    #[inline(always)]
    fn initialized_is_accessible(p: Phase) -> bool {
        !p.intersects(Phase::FINALIZED | Phase::FINALIZATION_PANICKED)
    }
}

type InitializedSoftFinalizedChecker<T> = InitializedSoftFinalizedCheckerGeneric<T, false>;

type InitializedHardFinalizedChecker<T> = InitializedHardFinalizedCheckerGeneric<T, false>;

//Lesser lazy are initializer before main so registration will always succeed
type InitializedSoftFinalizedCheckerLesser<T> = InitializedSoftFinalizedCheckerGeneric<T, true>;

type InitializedHardFinalizedCheckerLesser<T> = InitializedHardFinalizedCheckerGeneric<T, true>;

/// Thread local final registration always succeed for thread local on glibc plateforms
#[cfg(all(feature = "thread_local", cxa_thread_at_exit))]
type InitializedSoftFinalizedTLChecker<T> = InitializedSoftFinalizedCheckerGeneric<T, true>;

#[cfg(all(feature = "thread_local", cxa_thread_at_exit))]
type InitializedHardFinalizedTLChecker<T> = InitializedHardFinalizedCheckerGeneric<T, true>;

#[cfg(all(feature = "thread_local", not(cxa_thread_at_exit)))]
type InitializedSoftFinalizedTLChecker<T> = InitializedSoftFinalizedCheckerGeneric<T, false>;

#[cfg(all(feature = "thread_local", not(cxa_thread_at_exit)))]
type InitializedHardFinalizedTLChecker<T> = InitializedHardFinalizedCheckerGeneric<T, false>;

/// Helper trait to ease access static lazy associated functions
pub trait LazyAccess: Sized {
    type Target;
    /// Initialize if necessary then return a reference to the target.
    ///
    /// # Panics
    ///
    /// Panic if previous attempt to initialize has panicked and the lazy policy does not
    /// tolorate further initialization attempt or if initialization
    /// panic.
    fn get(this: Self) -> Self::Target;
    /// Return a reference to the target if initialized otherwise return an error.
    fn try_get(this: Self) -> Result<Self::Target, AccessError>;
    /// The current phase of the static
    fn phase(this: Self) -> Phase;
    /// Initialize the static if there were no previous attempt to initialize it.
    fn init(this: Self) -> Phase;
}

macro_rules! impl_lazy {
    ($tp:ident, $man:ident$(<$x:ident>)?, $checker:ident, $data:path, $locker:ty $(,T: $tr: ident)?$(,G: $trg:ident)?, $doc:literal $(cfg($attr:meta))?) => {
        impl_lazy! {@proc $tp,$man$(<$x>)?,$checker,$data,$locker $(,T:$tr)?$(,G:$trg)?,$doc $(cfg($attr))?}
        impl_lazy! {@deref $tp,$data$(,T:$tr)?$(,G:$trg)?}
    };
    (global $tp:ident, $man:ident$(<$x:ident>)?, $checker:ident, $data:ty, $locker:ty $(,T: $tr: ident)?$(,G: $trg:ident)?,$doc:literal $(cfg($attr:meta))?) => {
        impl_lazy! {@proc $tp,$man$(<$x>)?,$checker,$data,$locker$(,T:$tr)?$(,G:$trg)?,$doc $(cfg($attr))?, unsafe,'static}
        impl_lazy! {@deref_global $tp,$data$(,T:$tr)?$(,G:$trg)?}
    };
    (static $tp:ident, $man:ident$(<$x:ident>)?, $checker: ident, $data:ty, $locker:ty $(,T: $tr: ident)?$(,G: $trg:ident)?,$doc:literal $(cfg($attr:meta))?) => {
        impl_lazy! {@proc $tp,$man$(<$x>)?,$checker,$data,$locker$(,T:$tr)?$(,G:$trg)?,$doc $(cfg($attr))?, unsafe,'static}
        impl_lazy! {@deref_static $tp,$data$(,T:$tr)?$(,G:$trg)?}
    };
    (thread_local_static $tp:ident, $man:ident$(<$x:ident>)?, $checker: ident, $data:ty, $locker:ty $(,T: $tr: ident)?$(,G: $trg:ident)?,$doc:literal $(cfg($attr:meta))?) => {
        impl_lazy! {@proc $tp,$man$(<$x>)?,$checker,$data,$locker$(,T:$tr)?$(,G:$trg)?,$doc $(cfg($attr))?, unsafe,'static}
        impl_lazy! {@deref_thread_local $tp,$data$(,T:$tr)?$(,G:$trg)?}
    };
    (@deref $tp:ident, $data:ty $(,T: $tr: ident)?$(,G: $trg:ident)?) => {
        impl<T, G> $tp<T, G>
        where G: Generator<T>,
        $(G:$trg, T:Sync,)?
        $(T:$tr,)?
        {
            #[inline(always)]
            /// Initialize if necessary then return a reference to the target.
            ///
            /// # Panics
            ///
            /// Panic if previous attempt to initialize has panicked and the lazy policy does not
            /// tolorate further initialization attempt or if initialization
            /// panic.
            pub fn get(this: &Self) -> &T {
                this.__private.init_then_get()
            }
            #[inline(always)]
            /// Return a reference to the target if initialized otherwise return an error.
            pub fn try_get(this: &Self) -> Result<&'_ T,AccessError> {
                this.__private.try_get()
            }
            #[inline(always)]
            /// Initialize and return a mutable reference to the target
            ///
            /// This method is extremly efficient as it does not requires any
            /// form of locking when initializing
            pub fn get_mut(this: &mut Self) -> &mut T {
                this.__private.only_init_then_get_mut()
            }
            #[inline(always)]
            /// Return a mutable reference to the target if initialized otherwise return an error.
            ///
            /// This method is extremly efficient as it does not requires any
            /// form of locking when initializing
            pub fn try_get_mut(this: &mut Self) -> Result<&'_ mut T,AccessError> {
                this.__private.try_get_mut()
            }
            #[inline(always)]
            /// Return the phase
            pub fn phase(this: & Self) -> Phase {
                Phased::phase(&this.__private)
            }
            #[inline(always)]
            /// Initialize the lazy if not yet initialized
            ///
            /// # Panic
            ///
            /// Panic if the generator panics
            pub fn init(this: & Self) -> Phase {
                GenericLazy::init(&this.__private)
            }
        }
        impl<T, G> Deref for $tp<T, G>
        where G: Generator<T>,
        $(G:$trg, T:Sync,)?
        $(T:$tr,)?
        {
            type Target = T;
            #[inline(always)]
            fn deref(&self) -> &Self::Target {
                Self::get(self)
            }
        }

        impl<T, G> DerefMut for $tp<T, G>
        where G: Generator<T>,
        $(G:$trg, T:Sync,)?
        $(T:$tr,)?
        {
            #[inline(always)]
            fn deref_mut(&mut self) -> &mut Self::Target {
                Self::get_mut(self)
            }
        }

        impl<'a,T,G> LazyAccess for &'a $tp<T,G>
            where G: Generator<T>,
            $(G:$trg, T:Sync,)?
            $(T:$tr,)?
            {
            type Target = &'a T;
             #[inline(always)]
             fn get(this: Self) -> &'a T {
                 $tp::get(this)
             }
             #[inline(always)]
             fn try_get(this: Self) -> Result<&'a T,AccessError>{
                 $tp::try_get(this)
             }
             #[inline(always)]
             fn phase(this: Self) -> Phase{
                 $tp::phase(this)
             }
             #[inline(always)]
             fn init(this: Self) -> Phase {
                 $tp::init(this)
             }
        }

    };
    (@deref_static $tp:ident, $data:ty $(,T: $tr: ident)?$(,G: $trg:ident)?) => {
        impl<T, G> $tp<T, G>
        where G: 'static + Generator<T>,
        $(G:$trg, T:Sync,)?
        $(T:$tr,)?
        {
            #[inline(always)]
            /// Initialize if necessary then return a reference to the target.
            ///
            /// # Panics
            ///
            /// Panic if previous attempt to initialize has panicked and the lazy policy does not
            /// tolorate further initialization attempt or if initialization
            /// panic.
            pub fn get(this: &'static Self) -> &'static T {
                 // SAFETY The object is required to have 'static lifetime by construction
                 this.__private.init_then_get()
            }
            #[inline(always)]
            /// Return a reference to the target if initialized otherwise return an error.
            pub fn try_get(this: &'static Self) -> Result<&'static T,AccessError> {
                 // SAFETY The object is required to have 'static lifetime by construction
                 this.__private.try_get()
            }
            #[inline(always)]
            /// Return the phase
            pub fn phase(this: &'static Self) -> Phase {
                Phased::phase(&this.__private)
            }
            #[inline(always)]
            /// Initialize the lazy if not yet initialized
            ///
            /// # Panic
            ///
            /// Panic if the generator panics
            pub fn init(this: &'static Self) -> Phase {
                GenericLazy::init(&this.__private)
            }
        }
        impl<T, G> Deref for $tp<T, G>
        where G: 'static + Generator<T>,
        T:'static,
        $(G:$trg, T:Sync,)?
        $(T:$tr,)?
        {
            type Target = T;
            #[inline(always)]
            fn deref(&self) -> &Self::Target {
                 // SAFETY The object is required to have 'static lifetime by construction
                 Self::get(unsafe{as_static(self)})
            }
        }

        impl<T,G> LazyAccess for &'static $tp<T,G>
            where G: 'static + Generator<T>,
            $(G:$trg, T:Sync,)?
            $(T:$tr,)?
            {
            type Target = &'static T;
             #[inline(always)]
             fn get(this: Self) -> &'static T {
                 $tp::get(this)
             }
             #[inline(always)]
             fn try_get(this: Self) -> Result<&'static T,AccessError>{
                 $tp::try_get(this)
             }
             #[inline(always)]
             fn phase(this: Self) -> Phase{
                 $tp::phase(this)
             }
             #[inline(always)]
             fn init(this: Self) -> Phase {
                 $tp::init(this)
             }
        }

    };
    (@deref_global $tp:ident, $data:ty $(,T: $tr: ident)?$(,G: $trg:ident)?) => {
        impl<T, G> $tp<T, G>
        where G: 'static + Generator<T>,
        $(G:$trg, T:Sync,)?
        $(T:$tr,)?
        {
            #[inline(always)]
            /// Return a reference to the target if initialized otherwise return an error.
            pub fn try_get(this: &'static Self) -> Result<&'static T, AccessError> {
                if inited::global_inited_hint() {
                    // SAFETY The object is initialized a program start-up as long
                    // as it is constructed through the macros #[dynamic(quasi_lazy)]
                    // If initialization failed, the program terminates before the
                    // global_inited_hint is set. So if the global_initied_hint is
                    // set all LesserLazy are guaranteed to be initialized
                    // Moreover global lazy are never dropped
                    // TODO: get_unchecked
                    Ok(unsafe{this.__private.get_unchecked()})
                } else {
                    this.__private.try_get()
                }
            }
            #[inline(always)]
            /// Initialize if necessary then return a reference to the target.
            ///
            /// # Panics
            ///
            /// Panic if previous attempt to initialize has panicked and the lazy policy does not
            /// tolorate further initialization attempt or if initialization
            /// panic.
            pub fn get(this: &'static Self) -> &'static T {
                if inited::global_inited_hint() {
                    // SAFETY The object is initialized a program start-up as long
                    // as it is constructed through the macros #[dynamic(quasi_lazy)]
                    // If initialization failed, the program terminates before the
                    // global_inited_hint is set. So if the global_initied_hint is
                    // set all LesserLazy are guaranteed to be initialized
                    // Moreover global lazy are never dropped
                    unsafe{this.__private.get_unchecked()}
                } else {
                    this.__private.init_then_get()
                }
            }
            #[inline(always)]
            /// Return the phase
            pub fn phase(this: &'static Self) -> Phase {
                Phased::phase(&this.__private)
            }
            #[inline(always)]
            /// Initialize the lazy if not yet initialized
            ///
            /// # Panic
            ///
            /// Panic if the generator panics
            pub fn init(this: &'static Self) -> Phase {
                GenericLazy::init(&this.__private)
            }
        }
        impl<T, G> Deref for $tp<T, G>
        where G: 'static + Generator<T>,
        T:'static,
        $(G:$trg, T:Sync,)?
        $(T:$tr,)?
        {
            type Target = T;
            #[inline(always)]
            fn deref(&self) -> &Self::Target {
                // SAFETY The object is initialized a program start-up as long
                // as it is constructed through the macros #[dynamic(quasi_lazy)]
                // If initialization failed, the program terminates before the
                // global_inited_hint is set. So if the global_initied_hint is
                // set all LesserLazy are guaranteed to be initialized
                Self::get(unsafe{as_static(self)})
            }
        }
        impl<T,G> LazyAccess for &'static $tp<T,G>
            where G: 'static + Generator<T>,
            $(G:$trg, T:Sync,)?
            $(T:$tr,)?
            {
            type Target = &'static T;
             #[inline(always)]
             fn get(this: Self) -> &'static T {
                 $tp::get(this)
             }
             #[inline(always)]
             fn try_get(this: Self) -> Result<&'static T,AccessError>{
                 $tp::try_get(this)
             }
             #[inline(always)]
             fn phase(this: Self) -> Phase{
                 $tp::phase(this)
             }
             #[inline(always)]
             fn init(this: Self) -> Phase{
                 $tp::init(this)
             }
        }

    };
    (@deref_thread_local $tp:ident, $data:ty $(,T: $tr: ident)?$(,G: $trg:ident)?) => {
        impl<T, G> $tp<T, G>
        //where $data: 'static + LazyData<Target=T>,
        where G: 'static + Generator<T>,
        T:'static,
        $(G:$trg, T:Sync,)?
        $(T:$tr,)?
        {
            #[inline(always)]
            /// Initialize if necessary then return a reference to the target.
            ///
            /// # Panics
            ///
            /// Panic if previous attempt to initialize has panicked and the lazy policy does not
            /// tolorate further initialization attempt or if initialization
            /// panic.
            pub fn get(this: &Self) -> &T {
                 // SAFETY The object is required to have 'static lifetime by construction
                 unsafe {as_static(&this.__private).init_then_get()}
            }
            #[inline(always)]
            /// Return a reference to the target if initialized otherwise return an error.
            pub fn try_get(this: &Self) -> Result<&T,AccessError> {
                 // SAFETY The object is required to have 'static lifetime by construction
                 unsafe{as_static(&this.__private).try_get()}
            }
            #[inline(always)]
            /// Return the phase
            pub fn phase(this: &Self) -> Phase {
                Phased::phase(unsafe{as_static(&this.__private)})
            }
            #[inline(always)]
            /// Initialize the lazy if not yet initialized
            ///
            /// # Panic
            ///
            /// Panic if the generator panics
            pub fn init(this: &Self) -> Phase {
                GenericLazy::init(unsafe{as_static(&this.__private)})
            }
        }

        impl<T, G> Deref for $tp<T, G>
        where G: 'static + Generator<T>,
        T:'static,
        $(G:$trg, T:Sync,)?
        $(T:$tr,)?
        {
            type Target = T;
            #[inline(always)]
            fn deref(&self) -> &Self::Target {
                 Self::get(self)
            }
        }

        impl<'a,T,G> LazyAccess for &'a $tp<T,G>
            where G: 'static + Generator<T>,
            T:'static,
            $(G:$trg, T:Sync,)?
            $(T:$tr,)?
            {
            type Target = &'a T;
             #[inline(always)]
             fn get(this: Self) -> &'a T {
                 $tp::get(this)
             }
             #[inline(always)]
             fn try_get(this: Self) -> Result<&'a T,AccessError>{
                 $tp::try_get(this)
             }
             #[inline(always)]
             fn phase(this: Self) -> Phase{
                 $tp::phase(this)
             }
             #[inline(always)]
             fn init(this: Self) -> Phase {
                 $tp::init(this)
             }
        }

    };
    (@proc $tp:ident, $man:ident$(<$x:ident>)?, $checker:ident, $data:ty,$locker:ty $(,T: $tr: ident)?$(,G: $trg:ident)?,$doc:literal $(cfg($attr:meta))? $(,$safe:ident)?$(,$static:lifetime)?) => {
        #[doc=$doc]
        $(#[cfg_attr(docsrs,doc(cfg($attr)))])?
        pub struct $tp<T, G = fn() -> T> {
            __private: GenericLazy<$data, G, $man$(::<$x>)?, $checker::<G>>,
        }
        impl<T, G> Phased for $tp<T, G>
        //where $data: $($static +)? LazyData<Target=T>,
        where G: $($static +)? Generator<T>,
        $(G:$trg, T:Sync,)?
        $(T:$tr,)?
        {
            fn phase(this: &Self) -> Phase {
                Phased::phase(&this.__private)
            }
        }

        impl<T, G> $tp<T, G> {
            #[inline(always)]
            /// Build a new static object
            ///
            /// # Safety
            ///
            /// This function may be unsafe if building any thing else than a thread local object
            /// or a static will be the cause of undefined behavior
            pub const $($safe)? fn from_generator(f: G) -> Self {
                #[allow(unused_unsafe)]
                Self {

                    __private: unsafe{GenericLazy::new(f, $man::new(<$locker>::new(Phase::empty())),<$data>::INIT)},
                }
            }
            #[inline(always)]
            /// Build a new static object with debug information
            ///
            /// # Safety
            ///
            /// This function may be unsafe if building any thing else than a thread local object
            /// or a static will be the cause of undefined behavior
            pub const $($safe)?  fn from_generator_with_info(f: G, info: StaticInfo) -> Self {
                #[allow(unused_unsafe)]
                Self {
                    __private: unsafe{GenericLazy::new_with_info(f, $man::new(<$locker>::new(Phase::empty())), <$data>::INIT,info)},
                }
            }
        }

    };
}

impl_lazy! {Lazy,SyncSequentializer<G>,InitializedChecker,UnInited::<T>,SyncPhaseLocker,
"A type that initialize itself only once on the first access"}

impl_lazy! {global LesserLazy,SyncSequentializer<G>,InitializedChecker,UnInited::<T>,SyncPhaseLocker,
"The actual type of statics attributed with [#[dynamic]](macro@crate::dynamic). \
\
The method [from_generator](Self::from_generator) is unsafe because this kind of static \
can only safely be used through this attribute macros."
}

impl_lazy! {static LazyFinalize,ExitSequentializer<G>,InitializedSoftFinalizedChecker,UnInited::<T>,SyncPhaseLocker,T:Finaly,G:Sync,
"The actual type of statics attributed with [#[dynamic(lazy,finalize)]](macro@crate::dynamic) \
\
The method [from_generator](Self::from_generator) is unsafe as the object must be a non mutable static."
}

impl_lazy! {global LesserLazyFinalize,ExitSequentializer<G>,InitializedSoftFinalizedCheckerLesser,UnInited::<T>,SyncPhaseLocker,T:Finaly,G:Sync,
"The actual type of statics attributed with [#[dynamic(finalize)]](macro@crate::dynamic). \
\
The method [from_generator](Self::from_generator) is unsafe because this kind of static \
can only safely be used through this attribute macros."
}

impl_lazy! {UnSyncLazy,UnSyncSequentializer<G>,InitializedChecker,UnInited::<T>,UnSyncPhaseLocker,
"A version of [Lazy] whose reference can not be passed to other thread"
}

#[cfg(feature = "thread_local")]
impl_lazy! {thread_local_static UnSyncLazyFinalize,ThreadExitSequentializer<G>,InitializedSoftFinalizedTLChecker,UnInited::<T>,UnSyncPhaseLocker,T:Finaly,
"The actual type of thread_local statics attributed with [#[dynamic(finalize)]](macro@crate::dynamic) \
\
The method [from_generator](Self::from_generator) is unsafe as the object must be a non mutable static." cfg(feature="thread_local")
}
#[cfg(feature = "thread_local")]
impl_lazy! {thread_local_static UnSyncLazyDroped,ThreadExitSequentializer<G>,InitializedHardFinalizedTLChecker,DropedUnInited::<T>,UnSyncPhaseLocker,
"The actual type of thread_local statics attributed with [#[dynamic(drop)]](macro@crate::dynamic) \
\
The method [from_generator](Self::from_generator) is unsafe as the object must be a non mutable static." cfg(feature="thread_local")
}

use core::fmt::{self, Debug, Formatter};
macro_rules! non_static_debug {
    ($tp:ident, $data:ty $(,T: $tr: ident)?$(,G: $trg:ident)?) => {
        impl<T:Debug, G> Debug for $tp<T, G>
            //where $data: LazyData<Target=T>,
            where G: Generator<T>,
            $(G:$trg, T:Sync,)?
            $(T:$tr,)?
        {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                if ($tp::phase(self) & Phase::INITIALIZED).is_empty() {
                    write!(f,"UnInitialized")
                } else {
                    write!(f,"{:?}",**self)
                }
            }
        }
    }
}
macro_rules! non_static_impls {
    ($tp:ident, $data:ty $(,T: $tr:ident)? $(,G: $trg:ident)?) => {
        impl<T, G> $tp<T, Cell<Option<G>>>
        where
            G: FnOnce() -> T,
        {
            #[inline(always)]
            pub fn new(g: G) -> Self {
                Self::from_generator(Cell::new(Some(g)))
            }
        }
        impl<T: Default> Default for $tp<T, fn() -> T> {
            #[inline(always)]
            fn default() -> Self {
                Self::from_generator(T::default)
            }
        }
    };
}
non_static_impls! {Lazy,UnInited::<T>}
non_static_debug! {Lazy,UnInited::<T>}
non_static_impls! {UnSyncLazy,UnInited::<T>}
non_static_debug! {UnSyncLazy,UnInited::<T>}

impl<T, G> Drop for Lazy<T, G> {
    #[inline(always)]
    fn drop(&mut self) {
        if Phased::phase(GenericLazy::sequentializer(&self.__private))
            .intersects(Phase::INITIALIZED)
        {
            unsafe {
                GenericLazy::get_raw_data(&self.__private)
                    .get()
                    .drop_in_place()
            }
        }
    }
}
impl<T, G> Drop for UnSyncLazy<T, G> {
    #[inline(always)]
    fn drop(&mut self) {
        if Phased::phase(GenericLazy::sequentializer(&self.__private))
            .intersects(Phase::INITIALIZED)
        {
            unsafe {
                GenericLazy::get_raw_data(&self.__private)
                    .get()
                    .drop_in_place()
            }
        }
    }
}

macro_rules! non_static_mut_debug {
    ($tp:ident, $data:ty $(,T: $tr: ident)?$(,G: $trg:ident)?) => {
        impl<T:Debug, G> Debug for $tp<T, G>
            where G: Generator<T>,
            $(G:$trg, T:Sync,)?
            $(T:$tr,)?
        {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                if ($tp::phase(self) & Phase::INITIALIZED).is_empty() {
                    write!(f,"UnInitialized")
                } else {
                    write!(f,"{:?}",*self.read())
                }
            }
        }
    }
}

macro_rules! extend_locked_lazy {
    () => {
        non_static_impls! {LockedLazy,UnInited::<T>}
        non_static_mut_debug! {LockedLazy,UnInited::<T>}
        impl<T: Send, G: Generator<T>> LockedLazy<T, G> {
            #[inline(always)]
            /// Initialize and return a mutable reference to the target
            ///
            /// This method is extremly efficient as it does not requires any
            /// form of locking when initializing
            pub fn get_mut(&mut self) -> &mut T {
                self.__private.only_init_then_get_mut()
            }
            #[inline(always)]
            /// Return a mutable reference to the target if initialized otherwise return an error.
            ///
            /// This method is extremly efficient as it does not requires any
            /// form of locking when initializing
            pub fn try_get_mut(&mut self) -> Result<&mut T, AccessError> {
                self.__private.try_get_mut()
            }
        }
        impl<T, G> Drop for LockedLazy<T, G> {
            #[inline(always)]
            fn drop(&mut self) {
                if Phased::phase(GenericLockedLazy::sequentializer(&self.__private))
                    .intersects(Phase::INITIALIZED)
                {
                    unsafe { (&*self.__private).get().drop_in_place() }
                }
            }
        }
    };
}
macro_rules! extend_unsync_locked_lazy {
    () => {
        non_static_impls! {UnSyncLockedLazy,UnInited::<T>}
        non_static_mut_debug! {UnSyncLockedLazy,UnInited::<T>}

        impl<T, G: Generator<T>> UnSyncLockedLazy<T, G> {
            #[inline(always)]
            /// Initialize and return a mutable reference to the target
            ///
            /// This method is extremly efficient as it does not requires any
            /// form of locking when initializing
            pub fn get_mut(&mut self) -> &mut T {
                self.__private.only_init_then_get_mut()
            }
            #[inline(always)]
            /// Return a mutable reference to the target if initialized otherwise return an error.
            ///
            /// This method is extremly efficient as it does not requires any
            /// form of locking when initializing
            pub fn try_get_mut(&mut self) -> Result<&mut T, AccessError> {
                self.__private.try_get_mut()
            }
        }

        impl<T, G> Drop for UnSyncLockedLazy<T, G> {
            #[inline(always)]
            fn drop(&mut self) {
                if Phased::phase(GenericLockedLazy::sequentializer(&self.__private))
                    .intersects(Phase::INITIALIZED)
                {
                    unsafe { (&*self.__private).get().drop_in_place() }
                }
            }
        }
    };
}

macro_rules! impl_mut_lazy {
    ($mod: ident $(:$extension:ident)?, $tp:ident, $man:ident$(<$x:ident>)?, $checker:ident, $data:ty, $locker:ty, $gdw: ident, $gd: ident $(,T: $tr: ident)?$(,G: $trg:ident)?, $doc:literal $(cfg($attr:meta))?) => {
        pub mod $mod {
            use super::*;
        impl_mut_lazy! {@proc $tp,$man$(<$x>)?,$checker,$data,$locker,$gdw,$gd$(,T:$tr)?$(,G:$trg)?,$doc $(cfg($attr))?}
        impl_mut_lazy! {@lock $tp,$data,$gdw,$gd$(,T:$tr)?$(,G:$trg)?}
        impl_mut_lazy! {@uninited $tp, $man$(<$x>)?, $data, $locker}
            $($extension!{})?
        }
        #[doc(inline)]
        pub use $mod::$tp;
    };
    (static $mod: ident, $tp:ident, $man:ident$(<$x:ident>)?, $checker:ident, $data:ty, $locker: ty, $gdw: ident,$gd:ident  $(,T: $tr: ident)?$(,G: $trg:ident)?, $doc:literal $(cfg($attr:meta))?) => {
        pub mod $mod {
            use super::*;
        impl_mut_lazy! {@proc $tp,$man$(<$x>)?,$checker,$data,$locker,$gdw,$gd$(,T:$tr)?$(,G:$trg)?,$doc $(cfg($attr))?, 'static}
        impl_mut_lazy! {@lock $tp,$data,$gdw,$gd$(,T:$tr)?$(,G:$trg)? , 'static}
        impl_mut_lazy! {@uninited $tp, $man$(<$x>)?, $data, $locker}
        }
        #[doc(inline)]
        pub use $mod::$tp;
    };
    (const_static $mod: ident, $tp:ident, $man:ident$(<$x:ident>)?, $checker:ident, $data:ty, $locker: ty, $gdw: ident,$gd:ident  $(,T: $tr: ident)?$(,G: $trg:ident)?, $doc:literal $(cfg($attr:meta))?) => {
        pub mod $mod {
            use super::*;
        impl_mut_lazy! {@proc $tp,$man$(<$x>)?,$checker,$data,$locker,$gdw,$gd$(,T:$tr)?$(,G:$trg)?,$doc $(cfg($attr))?, 'static}
        impl_mut_lazy! {@const_lock $tp,$checker, $data,$gdw,$gd$(,T:$tr)?$(,G:$trg)? , 'static}
        impl_mut_lazy! {@prime $tp, $man$(<$x>)?, $data, $locker}
        }
        #[doc(inline)]
        pub use $mod::$tp;
    };
    (thread_local $mod: ident $(:$extension:ident)?, $tp:ident, $man:ident$(<$x:ident>)?, $checker:ident, $data:ty,$locker: ty,  $gdw: ident,$gd:ident  $(,T: $tr: ident)?$(,G: $trg:ident)?, $doc:literal $(cfg($attr:meta))?) => {
        pub mod $mod {
            use super::*;
        impl_mut_lazy! {@proc $tp,$man$(<$x>)?,$checker,$data,$locker,$gdw,$gd$(,T:$tr)?$(,G:$trg)?,$doc $(cfg($attr))?, unsafe}
        impl_mut_lazy! {@lock_thread_local $tp,$data,$gdw,$gd$(,T:$tr)?$(,G:$trg)?}
        impl_mut_lazy! {@uninited $tp, $man$(<$x>)?, $data, $locker, unsafe}
            $($extension!{})?
        }
        #[doc(inline)]
        pub use $mod::$tp;
    };
    (global $mod: ident, $tp:ident, $man:ident$(<$x:ident>)?, $checker:ident, $data:ty,$locker: ty,  $gdw: ident,$gd:ident$(,T: $tr: ident)?$(,G: $trg:ident)?, $doc:literal $(cfg($attr:meta))?) => {
        pub mod $mod {
            use super::*;
        impl_mut_lazy! {@proc $tp,$man$(<$x>)?,$checker,$data,$locker,$gdw,$gd$(,T:$tr)?$(,G:$trg)?,$doc $(cfg($attr))?, unsafe, 'static}
        impl_mut_lazy! {@lock_global $tp,$checker,$data,$gdw,$gd$(,T:$tr)?$(,G:$trg)?}
        impl_mut_lazy! {@uninited $tp, $man$(<$x>)?, $data, $locker, unsafe}
        }
        #[doc(inline)]
        pub use $mod::$tp;
    };
    (primed_static $mod: ident, $tp:ident, $man:ident$(<$x:ident>)?, $checker:ident, $data:ty, $locker:ty, $gdw: ident, $gd: ident $(,T: $tr: ident)?$(,G: $trg:ident)?, $doc:literal $(cfg($attr:meta))?) => {
        pub mod $mod {
            use super::*;
        impl_mut_lazy! {@proc $tp,$man$(<$x>)?,$checker,$data,$locker,$gdw,$gd$(,T:$tr)?$(,G:$trg)?,$doc $(cfg($attr))?, 'static}
        impl_mut_lazy! {@lock $tp,$data,$gdw,$gd$(,T:$tr)?$(,G:$trg)?, 'static}
        impl_mut_lazy! {@prime $tp, $man$(<$x>)?, $data, $locker}
        impl_mut_lazy! {@prime_static $tp, $checker, $data, $gdw, $gd$(,T:$tr)?$(,G:$trg)?}
        }
        #[doc(inline)]
        pub use $mod::$tp;
    };
    (global_primed_static $mod: ident, $tp:ident, $man:ident$(<$x:ident>)?, $checker:ident, $data:ty, $locker:ty, $gdw: ident, $gd: ident $(,T: $tr: ident)?$(,G: $trg:ident)?, $doc:literal $(cfg($attr:meta))?) => {
        pub mod $mod {
            use super::*;
        impl_mut_lazy! {@proc $tp,$man$(<$x>)?,$checker,$data,$locker,$gdw,$gd$(,T:$tr)?$(,G:$trg)?,$doc $(cfg($attr))?, 'static}
        impl_mut_lazy! {@lock_global $tp,$checker,$data,$gdw,$gd$(,T:$tr)?$(,G:$trg)?}
        impl_mut_lazy! {@prime $tp, $man$(<$x>)?, $data, $locker}
        impl_mut_lazy! {@prime_global $tp, $checker, $data, $gdw, $gd$(,T:$tr)?$(,G:$trg)?}
        }
        #[doc(inline)]
        pub use $mod::$tp;
    };
    (primed_thread_local $mod: ident, $tp:ident, $man:ident$(<$x:ident>)?, $checker:ident, $data:ty,$locker: ty,  $gdw: ident,$gd:ident  $(,T: $tr: ident)?$(,G: $trg:ident)?, $doc:literal $(cfg($attr:meta))?) => {
        pub mod $mod {
            use super::*;
        impl_mut_lazy! {@proc $tp,$man$(<$x>)?,$checker,$data,$locker,$gdw,$gd$(,T:$tr)?$(,G:$trg)?,$doc $(cfg($attr))?, unsafe}
        impl_mut_lazy! {@lock_thread_local $tp,$data,$gdw,$gd$(,T:$tr)?$(,G:$trg)?}
        impl_mut_lazy! {@prime $tp, $man$(<$x>)?, $data, $locker, unsafe}
        impl_mut_lazy! {@prime_thread_local $tp, $checker, $data, $gdw, $gd$(,T:$tr)?$(,G:$trg)?}
        }
        #[doc(inline)]
        pub use $mod::$tp;
    };
    (@lock $tp:ident, $data:ty, $gdw: ident, $gd:ident$(,T: $tr: ident)?$(,G: $trg:ident)? $(,$static:lifetime)?) => {
        impl<T, G> $tp<T, G>
        //where $data: $($static+)? LazyData<Target=T>,
        where G:$($static +)? Generator<T>,
        $(T: $static,)?
        $(G:$trg, T:Send,)?
        $(T:$tr,)?
        {
            #[inline(always)]
            /// Initialize if necessary and returns a read lock
            ///
            /// # Panic
            ///
            /// Panics if initialization panics or if initialization has panicked in a previous attempt to initialize.
            pub fn read(&$($static)? self) -> ReadGuard<'_,T> {
               ReadGuard(GenericLockedLazy::init_then_read_lock(&self.__private))
            }
            #[inline(always)]
            /// Initialize if necessary and returns some read lock if the lazy is not
            /// already write locked. If the lazy is already write locked it returns `None`
            ///
            /// # Panic
            ///
            /// If locks succeeds, panics if initialization panics or if initialization has panicked in a previous attempt to initialize.
            pub fn fast_read(&$($static)? self) -> Option<ReadGuard<'_,T>> {
               GenericLockedLazy::fast_init_then_read_lock(&self.__private).map(ReadGuard)
            }
            #[inline(always)]
            /// Get a read lock if the lazy is initialized or an [AccessError]
            pub fn try_read(&$($static)? self) -> Result<ReadGuard<'_,T>,AccessError> {
               GenericLockedLazy::try_read_lock(&self.__private).map(ReadGuard)
            }
            #[inline(always)]
            /// if the lazy is not already write locked: get a read lock if the lazy is initialized or an [AccessError].
            /// Otherwise returns `None`
            pub fn fast_try_read(&$($static)? self) -> Option<Result<ReadGuard<'_,T>,AccessError>> {
               GenericLockedLazy::fast_try_read_lock(&self.__private).map(|r| r.map(ReadGuard))
            }
            #[inline(always)]
            /// Initialize if necessary and returns a write lock
            ///
            /// # Panic
            ///
            /// Panics if initialization panics or if initialization has panicked in a previous attempt to initialize.
            pub fn write(&$($static)? self) -> WriteGuard<'_,T> {
               WriteGuard(GenericLockedLazy::init_then_write_lock(&self.__private))
            }
            #[inline(always)]
            /// Initialize if necessary and returns some write lock if the lazy is not
            /// already write locked. If the lazy is already read or write locked it returns `None`
            ///
            /// # Panic
            ///
            /// If locks succeeds, panics if initialization panics or if initialization has panicked in a previous attempt to initialize.
            pub fn fast_write(&$($static)? self) -> Option<WriteGuard<'_,T>> {
               GenericLockedLazy::fast_init_then_write_lock(&self.__private).map(WriteGuard)
            }
            #[inline(always)]
            /// Get a read lock if the lazy is initialized or an [AccessError]
            pub fn try_write(&$($static)? self) -> Result<WriteGuard<'_,T>,AccessError> {
               GenericLockedLazy::try_write_lock(&self.__private).map(WriteGuard)
            }
            #[inline(always)]
            /// if the lazy is not already read or write locked: get a write lock if the lazy is initialized or an [AccessError] . Otherwise returns `None`
            pub fn fast_try_write(&$($static)? self) -> Option<Result<WriteGuard<'_,T>,AccessError>> {
               GenericLockedLazy::fast_try_write_lock(&self.__private).map(|r| r.map(WriteGuard))
            }
            #[inline(always)]
            /// Initialize the lazy if no previous attempt to initialized it where performed
            pub fn init(&$($static)? self) {
                let _ = GenericLockedLazy::init_then_write_lock(&self.__private);
            }
        }

    };
    (@const_lock $tp:ident, $checker: ident, $data:ty, $gdw: ident, $gd:ident$(,T: $tr: ident)?$(,G: $trg:ident)? $(,$static:lifetime)?) => {
        impl<T, G> $tp<T, G>
        where G: $($static +)? Generator<T>,
        T:Uninit,
        $(T:$static ,)?
        $(G:$trg, T:Send,)?
        $(T:$tr,)?
        {
            #[inline(always)]
            /// get read lock
            ///
            /// # Panic
            ///
            /// Panics if the lazy was droped
            pub fn read(&'static self) -> ReadGuard<'_,T> {
                    let l = unsafe{GenericLockedLazy::read_lock_unchecked(&self.__private)};
                    assert!(<$checker::<G>>::initialized_is_accessible(Phased::phase(&l)));
                    ReadGuard(l)
            }
            /// Returns some read lock if the lazy is not
            /// already write locked. If the lazy is already write locked it returns `None`
            ///
            /// # Panic
            ///
            /// If locks succeeds, panics if the lazy was droped
            #[inline(always)]
            pub fn fast_read(&'static self) -> Option<ReadGuard<'_,T>> {
                    let l = unsafe{GenericLockedLazy::fast_read_lock_unchecked(&self.__private)};
                    if let Some(l) = &l {
                        assert!(<$checker::<G>>::initialized_is_accessible(Phased::phase(l)));
                    }
                    l.map(ReadGuard)
            }
            #[inline(always)]
            /// Return a read lock to the initialized value or an
            /// error containing a read lock to the post uninited value
            pub fn primed_read(&'static self) -> Result<ReadGuard<'_,T>,ReadGuard<'_,T>> {
               let l = unsafe {GenericLockedLazy::read_lock_unchecked(&self.__private)};
               let p = Phased::phase(&l);
               if <$checker::<G>>::initialized_is_accessible(p) {
                   Ok(ReadGuard(l))
               } else {
                   Err(ReadGuard(l))
               }
            }
        }

    };
    (@lock_thread_local $tp:ident, $data:ty,$gdw:ident,$gd:ident$(,T: $tr: ident)?$(,G: $trg:ident)?) => {

        use super::as_static;

        impl<T, G> $tp<T, G>
        //where $data: 'static + LazyData<Target=T>,
        where G: 'static + Generator<T>,
        T: 'static,
        $(G:$trg, T:Send,)?
        $(T:$tr,)?
        {
            #[inline(always)]
            /// Initialize if necessary and returns a read lock
            ///
            /// # Panic
            ///
            /// Panics if initialization panics or if initialization has panicked in a previous
            /// attempt to initialize.
            pub fn read(&self) -> ReadGuard<'_,T> {
                ReadGuard(GenericLockedLazy::init_then_read_lock(unsafe{as_static(&self.__private)}))
            }
            #[inline(always)]
            /// Initialize if necessary and returns some read lock if the lazy is not already write
            /// locked. If the lazy is already write locked it returns `None`
            ///
            /// # Panic
            ///
            /// If locks succeeds, panics if initialization panics or if initialization has
            /// panicked in a previous attempt to initialize.
            pub fn fast_read(&self) -> Option<ReadGuard<'_,T>> {
               GenericLockedLazy::fast_init_then_read_lock(unsafe{as_static(&self.__private)}).map(ReadGuard)
            }
            #[inline(always)]
            /// Get a read lock if the lazy is initialized or an [AccessError]
            pub fn try_read(&self) -> Result<ReadGuard<'_,T>,AccessError> {
               GenericLockedLazy::try_read_lock(unsafe{as_static(&self.__private)}).map(ReadGuard)
            }
            #[inline(always)]
            /// if the lazy is not already write locked: get a read lock if the lazy is initialized
            /// or an [AccessError]. Otherwise returns `None`
            pub fn fast_try_read(&self) -> Option<Result<ReadGuard<'_,T>,AccessError>> {
               GenericLockedLazy::fast_try_read_lock(unsafe{as_static(&self.__private)}).map(|r| r.map(ReadGuard))
            }
            #[inline(always)]
            /// Initialize if necessary and returns a write lock
            ///
            /// # Panic
            ///
            /// Panics if initialization panics or if initialization has panicked in a previous
            /// attempt to initialize.
            pub fn write(&self) -> WriteGuard<'_,T> {
                WriteGuard(GenericLockedLazy::init_then_write_lock(unsafe{as_static(&self.__private)}))
            }
            #[inline(always)]
            /// Initialize if necessary and returns some write lock if the lazy is not
            /// already write locked. If the lazy is already read or write locked it returns `None`
            ///
            /// # Panic
            ///
            /// If locks succeeds, panics if initialization panics or if initialization has
            /// panicked in a previous attempt to initialize.
            pub fn fast_write(&self) -> Option<WriteGuard<'_,T>> {
               GenericLockedLazy::fast_init_then_write_lock(unsafe{as_static(&self.__private)}).map(WriteGuard)
            }
            #[inline(always)]
            /// Get a read lock if the lazy is initialized or an [AccessError]
            pub fn try_write(&self) -> Result<WriteGuard<'_,T>,AccessError> {
               GenericLockedLazy::try_write_lock(unsafe{as_static(&self.__private)}).map(WriteGuard)
            }
            #[inline(always)]
            /// if the lazy is not already read or write locked: get a write lock if the lazy is
            /// initialized or an [AccessError] . Otherwise returns `None`
            pub fn fast_try_write(&self) ->
               Option<Result<WriteGuard<'_,T>,AccessError>> {
               GenericLockedLazy::fast_try_write_lock(unsafe{as_static(&self.__private)}).map(|r| r.map(WriteGuard))
            }
            #[inline(always)]
            /// Initialize the lazy if no previous attempt to initialized it where performed
            pub fn init(&self) -> Phase {
                let l = GenericLockedLazy::init_then_write_lock(unsafe{as_static(&self.__private)});
                Phased::phase(&l)
            }
        }

    };
    (@lock_global $tp:ident, $checker:ident, $data:ty,$gdw:ident,$gd:ident$(,T: $tr: ident)?$(,G: $trg:ident)?) => {

        use super::inited;

        impl<T, G> $tp<T, G>
        //where $data: 'static + LazyData<Target=T>,
        where G: 'static + Generator<T>,
        T: 'static,
        $(G:$trg, T:Send,)?
        $(T:$tr,)?
        {
            #[inline(always)]
            /// Initialize if necessary and returns a read lock
            ///
            /// # Panic
            ///
            /// Panics if initialization panics or if initialization has panicked in a previous attempt to initialize.
            pub fn read(&'static self) -> ReadGuard<'_,T> {
                if inited::global_inited_hint() {
                    let l = unsafe{GenericLockedLazy::read_lock_unchecked(&self.__private)};
                    assert!(<$checker::<G>>::initialized_is_accessible(Phased::phase(&l)));
                    ReadGuard(l)
                } else {
                    ReadGuard(GenericLockedLazy::init_then_read_lock(&self.__private))
                }
            }
            /// Initialize if necessary and returns some read lock if the lazy is not
            /// already write locked. If the lazy is already write locked it returns `None`
            ///
            /// # Panic
            ///
            /// If locks succeeds, panics if initialization panics or if initialization has panicked in a previous attempt to initialize.
            #[inline(always)]
            pub fn fast_read(&'static self) -> Option<ReadGuard<'_,T>> {
                if inited::global_inited_hint() {
                    let l = unsafe{GenericLockedLazy::fast_read_lock_unchecked(&self.__private)};
                    if let Some(l) = &l {
                        assert!(<$checker::<G>>::initialized_is_accessible(Phased::phase(l)));
                    }
                    l
                } else {
                    GenericLockedLazy::fast_init_then_read_lock(&self.__private)
                }.map(ReadGuard)
            }
            #[inline(always)]
            /// Get a read lock if the lazy is initialized or an [AccessError]
            pub fn try_read(&'static self) -> Result<ReadGuard<'_,T>,AccessError> {
                if inited::global_inited_hint() {
                    let l = unsafe{GenericLockedLazy::read_lock_unchecked(&self.__private)};
                    let p = Phased::phase(&l);
                    if <$checker::<G>>::initialized_is_accessible(p) {
                        Ok(l)
                    } else {
                        Err(AccessError{phase:p})
                    }
                } else {
                    GenericLockedLazy::try_read_lock(&self.__private)
                }.map(ReadGuard)
            }
            /// if the lazy is not already write locked: get a read lock if the lazy is initialized
            /// or an [AccessError]. Otherwise returns `None`
            #[inline(always)]
            pub fn fast_try_read(&'static self) -> Option<Result<ReadGuard<'_,T>,AccessError>> {
                if inited::global_inited_hint() {
                    let l = unsafe{GenericLockedLazy::fast_read_lock_unchecked(&self.__private)};
                    l.map(|l| {
                        let p = Phased::phase(&l);
                        if <$checker::<G>>::initialized_is_accessible(p) {
                            Ok(l)
                        } else {
                            Err(AccessError{phase:p})
                        }
                    })
                } else {
                    GenericLockedLazy::fast_try_read_lock(&self.__private)
                }.map(|r| r.map(ReadGuard))
            }
            /// Initialize if necessary and returns a write lock
            ///
            /// # Panic
            ///
            /// Panics if initialization panics or if initialization has panicked in a previous
            /// attempt to initialize.
            #[inline(always)]
            pub fn write(&'static self) -> WriteGuard<'_,T> {
                WriteGuard(if inited::global_inited_hint() {
                    let l = unsafe{GenericLockedLazy::write_lock_unchecked(&self.__private)};
                    assert!(<$checker::<G>>::initialized_is_accessible(Phased::phase(&l)));
                    l
                } else {
                    GenericLockedLazy::init_then_write_lock(&self.__private)
                })
            }
            /// Initialize if necessary and returns some write lock if the lazy is not
            /// already write locked. If the lazy is already read or write locked it returns `None`
            ///
            /// # Panic
            ///
            /// If locks succeeds, panics if initialization panics or if initialization has
            /// panicked in a previous attempt to initialize.
            #[inline(always)]
            pub fn fast_write(&'static self) -> Option<WriteGuard<'_,T>> {
                if inited::global_inited_hint() {
                    let l = unsafe{GenericLockedLazy::fast_write_lock_unchecked(&self.__private)};
                    if let Some(l) = &l {
                        assert!(<$checker::<G>>::initialized_is_accessible(Phased::phase(l)));
                    }
                    l
                } else {
                    GenericLockedLazy::fast_init_then_write_lock(&self.__private)
                }.map(WriteGuard)
            }
            /// Get a read lock if the lazy is initialized or an [AccessError]
            #[inline(always)]
            pub fn try_write(&'static self) -> Result<WriteGuard<'_,T>,AccessError> {
                if inited::global_inited_hint() {
                    let l = unsafe{GenericLockedLazy::write_lock_unchecked(&self.__private)};
                    let p = Phased::phase(&l);
                    if <$checker::<G>>::initialized_is_accessible(p) {
                        Ok(l)
                    } else {
                        Err(AccessError{phase:p})
                    }
                } else {
                    GenericLockedLazy::try_write_lock(&self.__private)
                }.map(WriteGuard)
            }
            /// if the lazy is not already read or write locked: get a write lock if the lazy is
            /// initialized or an [AccessError] . Otherwise returns `None`
            #[inline(always)]
            pub fn fast_try_write(&'static self) -> Option<Result<WriteGuard<'_,T>,AccessError>> {
                if inited::global_inited_hint() {
                    let l = unsafe{GenericLockedLazy::fast_write_lock_unchecked(&self.__private)};
                    l.map(|l| {
                        let p = Phased::phase(&l);
                        if <$checker::<G>>::initialized_is_accessible(p) {
                            Ok(l)
                        } else {
                            Err(AccessError{phase:p})
                        }
                    })
                } else {
                    GenericLockedLazy::fast_try_write_lock(&self.__private)
                }.map(|r| r.map(WriteGuard))
            }
            /// Initialize the lazy if no previous attempt to initialized it where performed
            #[inline(always)]
            pub fn init(&'static self) -> Phase {
                let l = GenericLockedLazy::init_then_write_lock(&self.__private);
                Phased::phase(&l)
            }
        }

    };
    (@prime_static $tp:ident,$checker:ident, $data:ty, $gdw: ident, $gd:ident$(,T: $tr: ident)?$(,G: $trg:ident)?) => {
        impl<T, G> $tp<T, G>
        //where $data: 'static + LazyData<Target=T>,
        where G: 'static + Generator<T>,
        T: 'static,
        $(G:$trg, T:Send,)?
        $(T:$tr,)?
        {
            #[inline(always)]
            /// Return a read lock to the initialized value or an error containing a read lock to
            /// the primed or post uninited value
            pub fn primed_read_non_initializing(&'static self) ->
               Result<ReadGuard<'_,T>,ReadGuard<'_,T>> {
               let l = unsafe {GenericLockedLazy::read_lock_unchecked(&self.__private)};
               let p = Phased::phase(&l);
               if <$checker::<G>>::is_accessible(p) {
                   Ok(ReadGuard(l))
               } else {
                   Err(ReadGuard(l))
               }
            }
            #[inline(always)]
            /// Initialize if possible and either return a read lock to the initialized value or an
            /// error containing a read lock to the primed or post uninited value
            pub fn primed_read(&'static self) -> Result<ReadGuard<'_,T>,ReadGuard<'_,T>> {
               let l = unsafe {GenericLockedLazy::init_then_read_lock_unchecked(&self.__private)};
               let p = Phased::phase(&l);
               if <$checker::<G>>::is_accessible(p) {
                   Ok(ReadGuard(l))
               } else {
                   Err(ReadGuard(l))
               }
            }
            #[inline(always)]
            /// Return a write lock that refers to the initialized value or an
            /// error containing a read lock that refers to the primed or post uninited value
            pub fn primed_write_non_initializing(&'static self) -> Result<WriteGuard<'_,T>,ReadGuard<'_,T>> {
               let l = unsafe{GenericLockedLazy::write_lock_unchecked(&self.__private)};
               let p = Phased::phase(&l);
               if <$checker::<G>>::is_accessible(p) {
                   Ok(WriteGuard(l))
               } else {
                   Err(ReadGuard(l.into()))
               }
            }
            #[inline(always)]
            /// Initialize if possible and either return a write lock that refers to the
            /// initialized value or an error containing a read lock that refers to the primed or
            /// post uninited value
            pub fn primed_write(&'static self) -> Result<WriteGuard<'_,T>,ReadGuard<'_,T>> {
               let l = unsafe{GenericLockedLazy::init_then_write_lock_unchecked(&self.__private)};
               let p = Phased::phase(&l);
               if <$checker::<G>>::is_accessible(p) {
                   Ok(WriteGuard(l))
               } else {
                   Err(ReadGuard(l.into()))
               }
            }
        }
    };
    (@prime_global $tp:ident,$checker:ident, $data:ty, $gdw: ident, $gd:ident$(,T: $tr: ident)?$(,G: $trg:ident)?) => {
        impl<T, G> $tp<T, G>
        //where $data: 'static + LazyData<Target=T>,
        where G: 'static + Generator<T>,
        T: 'static,
        $(G:$trg, T:Send,)?
        $(T:$tr,)?
        {
            #[inline(always)]
            /// Return a read lock to the initialized value or an error containing a read lock to
            /// the primed or post uninited value
            pub fn primed_read_non_initializing(&'static self) ->
               Result<ReadGuard<'_,T>,ReadGuard<'_,T>> {
               let l = unsafe {GenericLockedLazy::read_lock_unchecked(&self.__private)};
               let p = Phased::phase(&l);
               if inited::global_inited_hint() {
                  if <$checker::<G>>::initialized_is_accessible(p) {
                      Ok(ReadGuard(l))
                  } else {
                      Err(ReadGuard(l))
                  }
               } else {
                  if <$checker::<G>>::is_accessible(p) {
                      Ok(ReadGuard(l))
                  } else {
                      Err(ReadGuard(l))
                  }
               }
            }
            #[inline(always)]
            /// Initialize if possible and either return a read lock to the initialized value or an
            /// error containing a read lock to the primed or post uninited value
            pub fn primed_read(&'static self) -> Result<ReadGuard<'_,T>,ReadGuard<'_,T>> {
               let l = unsafe {GenericLockedLazy::init_then_read_lock_unchecked(&self.__private)};
               let p = Phased::phase(&l);
               if inited::global_inited_hint() {
                  if <$checker::<G>>::initialized_is_accessible(p) {
                      Ok(ReadGuard(l))
                  } else {
                      Err(ReadGuard(l))
                  }
               } else {
                  if <$checker::<G>>::is_accessible(p) {
                      Ok(ReadGuard(l))
                  } else {
                      Err(ReadGuard(l))
                  }
               }
            }
            #[inline(always)]
            /// Return a write lock that refers to the initialized value or an
            /// error containing a read lock that refers to the primed or post uninited value
            pub fn primed_write_non_initializing(&'static self) -> Result<WriteGuard<'_,T>,ReadGuard<'_,T>> {
               let l = unsafe{GenericLockedLazy::write_lock_unchecked(&self.__private)};
               let p = Phased::phase(&l);
               if inited::global_inited_hint() {
                  if <$checker::<G>>::initialized_is_accessible(p) {
                      Ok(WriteGuard(l))
                  } else {
                      Err(ReadGuard(l.into()))
                  }
               } else {
                  if <$checker::<G>>::is_accessible(p) {
                      Ok(WriteGuard(l))
                  } else {
                      Err(ReadGuard(l.into()))
                  }
               }
            }
            #[inline(always)]
            /// Initialize if possible and either return a write lock that refers to the
            /// initialized value or an error containing a read lock that refers to the primed or
            /// post uninited value
            pub fn primed_write(&'static self) -> Result<WriteGuard<'_,T>,ReadGuard<'_,T>> {
               let l = unsafe{GenericLockedLazy::init_then_write_lock_unchecked(&self.__private)};
               let p = Phased::phase(&l);
               if inited::global_inited_hint() {
                  if <$checker::<G>>::initialized_is_accessible(p) {
                      Ok(WriteGuard(l))
                  } else {
                      Err(ReadGuard(l.into()))
                  }
               } else {
                  if <$checker::<G>>::is_accessible(p) {
                      Ok(WriteGuard(l))
                  } else {
                      Err(ReadGuard(l.into()))
                  }
               }
            }
        }
    };
    (@prime_thread_local $tp:ident,$checker:ident, $data:ty, $gdw: ident, $gd:ident$(,T: $tr: ident)?$(,G: $trg:ident)?) => {
        impl<T, G> $tp<T, G>
        //where $data: 'static + LazyData<Target=T>,
        where G: 'static + Generator<T>,
        T:'static,
        $(G:$trg, T:Send,)?
        $(T:$tr,)?
        {
            #[inline(always)]
            /// Return a read lock to the initialized value or an
            /// error containing a read lock to the primed or post uninited value
            pub fn primed_read_non_initializing(&self) -> Result<ReadGuard<'_,T>,ReadGuard<'_,T>> {
               let l = unsafe{GenericLockedLazy::read_lock_unchecked(as_static(&self.__private))};
               let p = Phased::phase(&l);
               if <$checker::<G>>::is_accessible(p) {
                   Ok(ReadGuard(l))
               } else {
                   Err(ReadGuard(l))
               }
            }
            #[inline(always)]
            /// Initialize if possible and either return a read lock to the initialized value or an
            /// error containing a read lock to the primed or post uninited value
            pub fn primed_read(&self) -> Result<ReadGuard<'_,T>,ReadGuard<'_,T>> {
               let l = unsafe{GenericLockedLazy::init_then_read_lock_unchecked(as_static(&self.__private))};
               let p = Phased::phase(&l);
               if <$checker::<G>>::is_accessible(p) {
                   Ok(ReadGuard(l))
               } else {
                   Err(ReadGuard(l))
               }
            }
            #[inline(always)]
            /// Return a write lock that refers to the initialized value or an
            /// error containing a read lock that refers to the primed or post uninited value
            pub fn primed_write_non_initializing(&self) -> Result<WriteGuard<'_,T>,ReadGuard<'_,T>> {
               let l = unsafe{GenericLockedLazy::write_lock_unchecked(as_static(&self.__private))};
               let p = Phased::phase(&l);
               if <$checker::<G>>::is_accessible(p) {
                   Ok(WriteGuard(l))
               } else {
                   Err(ReadGuard(l.into()))
               }
            }
            #[inline(always)]
            /// Initialize if possible and either return a write lock that refers to the initialized value or an
            /// error containing a read lock that refers to the primed or post uninited value
            pub fn primed_write(&self) -> Result<WriteGuard<'_,T>,ReadGuard<'_,T>> {
               let l = unsafe{GenericLockedLazy::init_then_write_lock_unchecked(as_static(&self.__private))};
               let p = Phased::phase(&l);
               if <$checker::<G>>::is_accessible(p) {
                   Ok(WriteGuard(l))
               } else {
                   Err(ReadGuard(l.into()))
               }
            }
        }
    };
    (@uninited $tp:ident, $man:ident$(<$x:ident>)?, $data:ty, $locker: ty$(,$safe:ident)?) => {
        impl<T, G> $tp<T, G> {
            #[inline(always)]
            /// Build a new static object.
            ///
            /// # Safety
            ///
            /// This function may be unsafe if build this object as anything else than
            /// a static or a thread local static would be the cause of undefined behavior
            pub const $($safe)? fn from_generator(f: G) -> Self {
                #[allow(unused_unsafe)]
                Self {

                    __private: unsafe{GenericLockedLazy::new(f, $man::new(<$locker>::new(Phase::empty())),<$data>::INIT)},
                }
            }
            #[inline(always)]
            /// Build a new static object with debug informations.
            ///
            /// # Safety
            ///
            /// This function may be unsafe if build this object as anything else than
            /// a static or a thread local static would be the cause of undefined behavior
            pub const $($safe)?  fn from_generator_with_info(f: G, info: StaticInfo) -> Self {
                #[allow(unused_unsafe)]
                Self {
                    __private: unsafe{GenericLockedLazy::new_with_info(f, $man::new(<$locker>::new(Phase::empty())), <$data>::INIT,info)},
                }
            }
        }
    };
    (@prime $tp:ident, $man:ident$(<$x:ident>)?, $data:ty, $locker: ty $(,$safe:ident)?) => {
        impl<T, G> $tp<T, G> {
            #[inline(always)]
            /// Build a new static object.
            ///
            /// # Safety
            ///
            /// This function may be unsafe if build this object as anything else than
            /// a static or a thread local static would be the cause of undefined behavior
            pub const $($safe)? fn from_generator(v: T, f: G) -> Self {
                #[allow(unused_unsafe)]
                Self {

                    __private: unsafe{GenericLockedLazy::new(f, $man::new(<$locker>::new(Phase::empty())),<$data>::prime(v))},
                }
            }
            #[inline(always)]
            /// Build a new static object with debug informations.
            ///
            /// # Safety
            ///
            /// This function may be unsafe if build this object as anything else than
            /// a static or a thread local static would be the cause of undefined behavior
            pub const $($safe)?  fn from_generator_with_info(v: T, f: G, info: StaticInfo) -> Self {
                #[allow(unused_unsafe)]
                Self {
                    __private: unsafe{GenericLockedLazy::new_with_info(f, $man::new(<$locker>::new(Phase::empty())), <$data>::prime(v),info)},
                }
            }
        }
    };
    (@proc $tp:ident, $man:ident$(<$x:ident>)?, $checker:ident, $data:ty, $locker: ty, $gdw: ident, $gd:ident $(,T: $tr: ident)?$(,G: $trg:ident)?
    ,$doc:literal $(cfg($attr:meta))? $(,$safe:ident)? $(,$static:lifetime)?) => {
        #[doc=$doc]
        $(#[cfg_attr(docsrs,doc(cfg($attr)))])?
        pub struct $tp<T, G = fn() -> T> {
            __private: GenericLockedLazy<$data, G, $man$(<$x>)?, $checker::<G>>,
        }

        #[must_use="If unused the write lock is immediatly released"]
        #[derive(Debug)]
        pub struct WriteGuard<'a,T>(generic_lazy::WriteGuard<$gdw::<'a,$data>>);

        #[must_use="If unused the write lock is immediatly released"]
        #[derive(Debug)]
        pub struct ReadGuard<'a,T>(generic_lazy::ReadGuard<$gd::<'a,$data>>);

        impl<'a,T> Clone for ReadGuard<'a,T> {
            #[inline(always)]
            fn clone(&self) -> Self {
                Self(self.0.clone())
            }
        }
        impl<'a,T> From<WriteGuard<'a,T>> for ReadGuard<'a,T> {
            #[inline(always)]
            fn from(that:WriteGuard<'a,T>) -> Self {
                Self(that.0.into())
            }
        }

        use core::ops::{Deref,DerefMut};

        impl<'a,T> Deref for WriteGuard<'a,T>
        $(where T: $static)?
        {
            type Target = T;
            #[inline(always)]
            fn deref(&self) -> &T {
                &*self.0
            }
        }
        impl<'a,T> DerefMut for WriteGuard<'a,T>
        $(where T: $static)?
        {
            #[inline(always)]
            fn deref_mut(&mut self) -> &mut T {
                &mut *self.0
            }
        }
        impl<'a,T> Deref for ReadGuard<'a,T>
        $(where T: $static)?
        {
            type Target = T;
            #[inline(always)]
            fn deref(&self) -> &T {
                &*self.0
            }
        }

        impl<'a, T> Phased for ReadGuard<'a,T>
        $(where T: $static)?
        {
            #[inline(always)]
            fn phase(this: &Self) -> Phase {
                Phased::phase(&this.0)
            }
        }

        impl<'a, T> Phased for WriteGuard<'a,T>
        $(where T: $static)?
        {
            #[inline(always)]
            fn phase(this: &Self) -> Phase {
                Phased::phase(&this.0)
            }
        }

        impl<T, G> Phased for $tp<T, G>
        where
        $(T: $static ,)?
        G: $($static +)? Generator<T>
        {
            #[inline(always)]
            fn phase(this: &Self) -> Phase {
                Phased::phase(&this.__private)
            }
        }

        impl<T, G> $tp<T, G>
        where
        $(T: $static ,)?
        G: $($static +)? Generator<T>,
        $(G:$trg, T:Send,)?
        $(T:$tr,)?
        {
            #[inline(always)]
            /// Returns the current phase and synchronize with the end
            /// of the transition to the returned phase.
            pub fn phase(&$($static)? self) -> Phase {
                Phased::phase(&self.__private)
            }
        }
    };
}

impl_mut_lazy! {locked_lazy:extend_locked_lazy, LockedLazy,SyncSequentializer<G>,InitializedChecker,UnInited::<T>, SyncPhaseLocker, SyncPhaseGuard, SyncReadPhaseGuard,
"A mutable locked lazy that initialize its content on the first lock"}

impl_mut_lazy! {global lesser_locked_lazy, LesserLockedLazy,SyncSequentializer<G>,InitializedChecker,UnInited::<T>, SyncPhaseLocker, SyncPhaseGuard, SyncReadPhaseGuard,
"The actual type of mutable statics attributed with [#[dynamic]](macro@crate::dynamic) \
\
The method [from_generator](Self::from_generator) is unsafe because this kind of static \
can only safely be used through this attribute macros."
}

impl_mut_lazy! {primed_static primed_locked_lazy, PrimedLockedLazy,SyncSequentializer<G>,InitializedChecker,Primed::<T>, SyncPhaseLocker, SyncPhaseGuard, SyncReadPhaseGuard,
"The actual type of mutable statics attributed with [#[dynamic(primed)]](macro@crate::dynamic)"}

impl_mut_lazy! {global_primed_static primed_lesser_locked_lazy, PrimedLesserLockedLazy,SyncSequentializer<G>,InitializedChecker,Primed::<T>, SyncPhaseLocker, SyncPhaseGuard, SyncReadPhaseGuard,
"The actual type of mutable statics attributed with [#[dynamic(primed)]](macro@crate::dynamic)"}

impl_mut_lazy! {static locked_lazy_finalize,LockedLazyFinalize,ExitSequentializer<G>,InitializedSoftFinalizedChecker,UnInited::<T>,SyncPhaseLocker, SyncPhaseGuard, SyncReadPhaseGuard, T:Finaly,G:Sync,
"The actual type of mutable statics attributed with [#[dynamic(lazy,finalize)]](macro@crate::dynamic)"
}

impl_mut_lazy! {global lesser_locked_lazy_finalize,LesserLockedLazyFinalize,ExitSequentializer<G>,InitializedSoftFinalizedCheckerLesser,UnInited::<T>,SyncPhaseLocker, SyncPhaseGuard, SyncReadPhaseGuard,T:Finaly, G:Sync,
"The actual type of mutable statics attributed with [#[dynamic(finalize)]](macro@crate::dynamic) \
\
The method [from_generator](Self::from_generator) is unsafe because this kind of static \
can only safely be used through this attribute macros."
}
impl_mut_lazy! {static locked_lazy_droped,LockedLazyDroped,ExitSequentializer<G>,InitializedHardFinalizedChecker,DropedUnInited::<T>, SyncPhaseLocker, SyncPhaseGuard, SyncReadPhaseGuard,G:Sync,
"The actual type of statics attributed with [#[dynamic(lazy,finalize)]](macro@crate::dynamic)"
}

impl_mut_lazy! {global lesser_locked_lazy_droped,LesserLockedLazyDroped,ExitSequentializer<G>,InitializedHardFinalizedCheckerLesser,DropedUnInited::<T>, SyncPhaseLocker, SyncPhaseGuard, SyncReadPhaseGuard,G:Sync,
"The actual type of mutable statics attributed with #[dynamic(drop)] \
\
The method (new)[Self::from_generator] is unsafe because this kind of static \
can only safely be used through this attribute macros."
}

impl_mut_lazy! {primed_static primed_locked_lazy_droped,PrimedLockedLazyDroped,ExitSequentializer<G>,InitializedHardFinalizedChecker,Primed::<T>, SyncPhaseLocker, SyncPhaseGuard, SyncReadPhaseGuard,T:Uninit, G:Sync,
"The actual type of mutable statics attributed with [#[dynamic(primed,drop)]](macro@crate::dynamic)"
}

impl_mut_lazy! {global_primed_static global_primed_locked_lazy_droped,PrimedLesserLockedLazyDroped,ExitSequentializer<G>,InitializedHardFinalizedChecker,Primed::<T>, SyncPhaseLocker, SyncPhaseGuard, SyncReadPhaseGuard,T:Uninit, G:Sync,
"The actual type of mutable statics attributed with [#[dynamic(primed,drop)]](macro@crate::dynamic)"
}

//impl_mut_lazy! {const_static const_locked_lazy_droped, ConstLockedLazyDroped,ExitSequentializer<G>,InitializedSoftFinalizedChecker,Primed::<T>, SyncPhaseLocker, SyncPhaseGuard, SyncReadPhaseGuard,G:Sync,
//"The actual type of statics (non mutable) attributed with [#[dynamic(lazy,drop)]](macro@crate::dynamic)"
//}

impl_mut_lazy! {unsync_locked_lazy:extend_unsync_locked_lazy,UnSyncLockedLazy,UnSyncSequentializer<G>,InitializedChecker,UnInited::<T>,UnSyncPhaseLocker, UnSyncPhaseGuard,UnSyncReadPhaseGuard,
"A RefCell that initializes its content on the first access"
}

#[cfg(feature = "thread_local")]
impl_mut_lazy! {primed_thread_local unsync_primed_locked_lazy,UnSyncPrimedLockedLazy,UnSyncSequentializer<G>,InitializedChecker,Primed::<T>,UnSyncPhaseLocker, UnSyncPhaseGuard,UnSyncReadPhaseGuard,
"The actual type of mutable thread_local statics attributed with [#[dynamic(primed)]](macro@crate::dynamic) \
\
The method [from_generator](Self::from_generator)  is unsafe as the object must be a non mutable thread_local static." cfg(feature="thread_local")
}
#[cfg(feature = "thread_local")]
impl_mut_lazy! {primed_thread_local unsync_primed_locked_lazy_droped,UnSyncPrimedLockedLazyDroped,ThreadExitSequentializer<G>,InitializedHardFinalizedTLChecker,Primed::<T>,UnSyncPhaseLocker, UnSyncPhaseGuard,UnSyncReadPhaseGuard, T:Uninit,
"The actual type of mutable thread_local statics attributed with [#[dynamic(primed,drop)]](macro@crate::dynamic) \
\
The method [from_generator](Self::from_generator) is unsafe as the object must be a non mutable thread_local static." cfg(feature="thread_local")
}

#[cfg(feature = "thread_local")]
impl_mut_lazy! {thread_local unsync_locked_lazy_finalize,UnSyncLockedLazyFinalize,ThreadExitSequentializer<G>,InitializedSoftFinalizedTLChecker,UnInited::<T>,UnSyncPhaseLocker, UnSyncPhaseGuard,UnSyncReadPhaseGuard,T:Finaly,
"The actual type of mutable thread_local statics attributed with [#[dynamic(finalize)]](macro@crate::dynamic) \
\
The method [from_generator](Self::from_generator) is unsafe as the object must be a non mutable thread_local static." cfg(feature="thread_local")
}
#[cfg(feature = "thread_local")]
impl_mut_lazy! {thread_local unsync_locked_lazy_droped,UnSyncLockedLazyDroped,ThreadExitSequentializer<G>,InitializedHardFinalizedTLChecker,DropedUnInited::<T>,UnSyncPhaseLocker, UnSyncPhaseGuard,UnSyncReadPhaseGuard,
"The actual type of thread_local mutable statics attributed with [#[dynamic(drop)]](macro@crate::dynamic) \
\
The method [from_generator](Self::from_generator) is unsafe as the object must be a non mutable thread_local static." cfg(feature="thread_local")
}

#[cfg(all(support_priority, not(feature = "test_no_global_lazy_hint")))]
mod inited {

    use core::sync::atomic::{AtomicBool, Ordering};

    static LAZY_INIT_ENSURED: AtomicBool = AtomicBool::new(false);

    #[static_init_macro::constructor(__lazy_init_finished)]
    extern "C" fn mark_inited() {
        LAZY_INIT_ENSURED.store(true, Ordering::Release);
    }

    #[inline(always)]
    pub(super) fn global_inited_hint() -> bool {
        LAZY_INIT_ENSURED.load(Ordering::Acquire)
    }
}
#[cfg(not(all(support_priority, not(feature = "test_no_global_lazy_hint"))))]
mod inited {
    #[inline(always)]
    pub(super) const fn global_inited_hint() -> bool {
        false
    }
}

#[cfg(test)]
mod test_lazy {
    use super::Lazy;
    static _X: Lazy<u32, fn() -> u32> = Lazy::from_generator(|| 22);

    #[test]
    fn test() {
        assert_eq!(*_X, 22);
    }
}

#[cfg(feature = "test_no_global_lazy_hint")]
#[cfg(test)]
mod test_quasi_lazy {
    use super::LesserLazy;
    static _X: LesserLazy<u32, fn() -> u32> = unsafe { LesserLazy::from_generator(|| 22) };
    #[test]
    fn test() {
        assert_eq!(*_X, 22);
    }
}
#[cfg(all(test, feature = "thread_local"))]
mod test_local_lazy {
    use super::UnSyncLazy;
    #[thread_local]
    static _X: UnSyncLazy<u32, fn() -> u32> = UnSyncLazy::from_generator(|| 22);
    #[test]
    fn test() {
        assert_eq!(*_X, 22);
    }
}
#[cfg(test)]
mod test_lazy_finalize {
    use super::LazyFinalize;
    use crate::Finaly;
    #[derive(Debug)]
    struct A(u32);
    impl Finaly for A {
        fn finaly(&self) {}
    }
    static _X: LazyFinalize<A, fn() -> A> = unsafe { LazyFinalize::from_generator(|| A(22)) };
    #[test]
    fn test() {
        assert_eq!((*_X).0, 22);
    }
}
#[cfg(feature = "test_no_global_lazy_hint")]
#[cfg(test)]
mod test_quasi_lazy_finalize {
    use super::LesserLazyFinalize;
    use crate::Finaly;
    #[derive(Debug)]
    struct A(u32);
    impl Finaly for A {
        fn finaly(&self) {}
    }
    static _X: LesserLazyFinalize<A, fn() -> A> =
        unsafe { LesserLazyFinalize::from_generator(|| A(22)) };
    #[test]
    fn test() {
        assert_eq!((*_X).0, 22);
    }
}
#[cfg(all(test, feature = "thread_local"))]
mod test_local_lazy_finalize {
    use super::UnSyncLazyFinalize;
    use crate::Finaly;
    #[derive(Debug)]
    struct A(u32);
    impl Finaly for A {
        fn finaly(&self) {}
    }
    #[thread_local]
    static _X: UnSyncLazyFinalize<A, fn() -> A> =
        unsafe { UnSyncLazyFinalize::from_generator(|| A(22)) };
    #[test]
    fn test() {
        assert_eq!((*_X).0, 22);
    }
}
#[cfg(all(test, feature = "thread_local"))]
mod test_droped_local_lazy_finalize {
    use super::UnSyncLazyDroped;
    #[derive(Debug)]
    struct A(u32);
    #[thread_local]
    static _X: UnSyncLazyDroped<A> = unsafe { UnSyncLazyDroped::from_generator(|| A(22)) };
    #[test]
    fn test() {
        assert_eq!(_X.0, 22);
    }
}

#[cfg(test)]
mod test_mut_lazy {
    use super::LockedLazy;
    static _X: LockedLazy<u32, fn() -> u32> = LockedLazy::from_generator(|| 22);
    #[test]
    fn test() {
        assert_eq!(*_X.read(), 22);
        *_X.write() = 33;
        assert_eq!(*_X.read(), 33);
    }
}

#[cfg(test)]
mod test_primed_mut_lazy_droped {
    use super::PrimedLockedLazyDroped;
    use crate::Uninit;
    #[derive(Debug)]
    struct A(u32);
    impl Uninit for A {
        fn uninit(&mut self) {
            self.0 = 0
        }
    }
    static _X: PrimedLockedLazyDroped<A> = PrimedLockedLazyDroped::from_generator(A(42), || A(22));
    #[test]
    fn test() {
        assert_eq!(_X.primed_read_non_initializing().unwrap_err().0, 42);
        assert_eq!(_X.read().0, 22);
        _X.write().0 = 33;
        assert_eq!(_X.read().0, 33);
    }
}

#[cfg(test)]
mod test_primed_mut_lazy {
    use super::PrimedLockedLazy;
    static _X: PrimedLockedLazy<u32> = PrimedLockedLazy::from_generator(42, || 22);
    #[test]
    fn test() {
        assert_eq!(*_X.primed_read_non_initializing().unwrap_err(), 42);
        assert_eq!(*_X.read(), 22);
        *_X.write() = 33;
        assert_eq!(*_X.read(), 33);
    }
}

#[cfg(feature = "test_no_global_lazy_hint")]
#[cfg(test)]
mod test_quasi_mut_lazy {
    use super::LesserLockedLazy;
    static _X: LesserLockedLazy<u32, fn() -> u32> =
        unsafe { LesserLockedLazy::from_generator(|| 22) };
    #[test]
    fn test() {
        assert_eq!(*_X.read(), 22);
        *_X.write() = 33;
        assert_eq!(*_X.read(), 33);
    }
}
#[cfg(test)]
mod test_mut_lazy_finalize {
    use super::LockedLazyFinalize;
    use crate::Finaly;
    #[derive(Debug)]
    struct A(u32);
    impl Finaly for A {
        fn finaly(&self) {}
    }
    static _X: LockedLazyFinalize<A, fn() -> A> = LockedLazyFinalize::from_generator(|| A(22));
    #[test]
    fn test() {
        assert!((*_X.read()).0 == 22);
        *_X.write() = A(33);
        assert_eq!((*_X.read()).0, 33);
    }
}
#[cfg(feature = "test_no_global_lazy_hint")]
#[cfg(test)]
mod test_quasi_mut_lazy_finalize {
    use super::LesserLockedLazyFinalize;
    use crate::Finaly;
    #[derive(Debug)]
    struct A(u32);
    impl Finaly for A {
        fn finaly(&self) {}
    }
    static _X: LesserLockedLazyFinalize<A, fn() -> A> =
        unsafe { LesserLockedLazyFinalize::from_generator(|| A(22)) };
    #[test]
    fn test() {
        assert!((*_X.read()).0 == 22);
        *_X.write() = A(33);
        assert_eq!((*_X.read()).0, 33);
    }
}
#[cfg(test)]
mod test_mut_lazy_dropped {
    use super::LockedLazyDroped;
    static _X: LockedLazyDroped<u32, fn() -> u32> = LockedLazyDroped::from_generator(|| 22);
    #[test]
    fn test() {
        assert_eq!(*_X.read(), 22);
        *_X.write() = 33;
        assert_eq!(*_X.read(), 33);
    }
}
#[cfg(feature = "test_no_global_lazy_hint")]
#[cfg(test)]
mod test_quasi_mut_lazy_dropped {
    use super::LesserLockedLazyDroped;
    static _X: LesserLockedLazyDroped<u32, fn() -> u32> =
        unsafe { LesserLockedLazyDroped::from_generator(|| 22) };
    #[test]
    fn test() {
        assert_eq!(*_X.read(), 22);
        *_X.write() = 33;
        assert_eq!(*_X.read(), 33);
    }
}
#[cfg(test)]
#[cfg(feature = "thread_local")]
mod test_unsync_mut_lazy {
    use super::UnSyncLockedLazy;
    #[thread_local]
    static _X: UnSyncLockedLazy<u32, fn() -> u32> = UnSyncLockedLazy::from_generator(|| 22);
    #[test]
    fn test() {
        assert_eq!(*_X.read(), 22);
        *_X.write() = 33;
        assert_eq!(*_X.read(), 33);
    }
}

#[cfg(test)]
#[cfg(feature = "thread_local")]
mod test_unsync_mut_primed_lazy {
    use super::UnSyncPrimedLockedLazy;
    #[thread_local]
    static _X: UnSyncPrimedLockedLazy<u32> =
        unsafe { UnSyncPrimedLockedLazy::from_generator(42, || 22) };
    #[test]
    fn test() {
        assert_eq!(*_X.primed_read_non_initializing().unwrap_err(), 42);
        assert_eq!(*_X.read(), 22);
        *_X.write() = 33;
        assert_eq!(*_X.read(), 33);
    }
}
#[cfg(test)]
#[cfg(feature = "thread_local")]
mod test_unsync_mut_primed_lazy_droped {
    use super::UnSyncPrimedLockedLazyDroped;
    use crate::Uninit;
    #[derive(Debug)]
    struct A(u32);
    impl Uninit for A {
        fn uninit(&mut self) {
            self.0 = 0
        }
    }
    #[thread_local]
    static _X: UnSyncPrimedLockedLazyDroped<A> =
        unsafe { UnSyncPrimedLockedLazyDroped::from_generator(A(42), || A(22)) };
    #[test]
    fn test() {
        assert_eq!(_X.primed_read_non_initializing().unwrap_err().0, 42);
        assert_eq!(_X.read().0, 22);
        _X.write().0 = 33;
        assert_eq!(_X.read().0, 33);
    }
}

#[cfg(test)]
#[cfg(feature = "thread_local")]
mod test_unsync_mut_lazy_finalize {
    use super::UnSyncLockedLazyFinalize;
    use crate::Finaly;
    #[derive(Debug)]
    struct A(u32);
    impl Finaly for A {
        fn finaly(&self) {}
    }
    #[thread_local]
    static _X: UnSyncLockedLazyFinalize<A, fn() -> A> =
        unsafe { UnSyncLockedLazyFinalize::from_generator(|| A(22)) };
    #[test]
    fn test() {
        assert!((*_X.read()).0 == 22);
        *_X.write() = A(33);
        assert_eq!((*_X.read()).0, 33);
    }
}
#[cfg(test)]
#[cfg(feature = "thread_local")]
mod test_unsync_mut_lazy_droped {
    use super::UnSyncLockedLazyDroped;
    #[thread_local]
    static _X: UnSyncLockedLazyDroped<u32, fn() -> u32> =
        unsafe { UnSyncLockedLazyDroped::from_generator(|| 22) };
    #[test]
    fn test() {
        assert_eq!(*_X.read(), 22);
        *_X.write() = 33;
        assert_eq!(*_X.read(), 33);
    }
}

#[inline(always)]
/// # Safety
/// v must refer to a static
unsafe fn as_static<T>(v: &T) -> &'static T {
    &*(v as *const _)
}
