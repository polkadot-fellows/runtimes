#![allow(unused)] //functions that are usefull for extension

use crate::{
    Finaly, Generator, LazySequentializer, LockNature, LockResult, Phase, Phased, Sequential,
    Sequentializer, StaticInfo, Uninit, UniqueLazySequentializer,
};
use core::cell::UnsafeCell;
use core::fmt::{self, Debug, Display, Formatter};
use core::hint::unreachable_unchecked;
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::ops::{Deref, DerefMut};

#[cfg(debug_mode)]
use crate::CyclicPanic;

#[cfg(any(feature = "parking_lot_core", debug_mode))]
use std::panic::RefUnwindSafe;

/// Policy for lazy initialization
pub(crate) trait LazyPolicy {
    /// shall the initialization be performed (tested at each access)
    fn shall_init(_: Phase) -> bool;
    /// Is the object accessible in phase `p`
    fn is_accessible(p: Phase) -> bool;
    /// Is the object accessible in phase `p`
    fn post_init_is_accessible(p: Phase) -> bool;
    /// If the object is known to already be initialized, is the object accessible in phase `p`
    fn initialized_is_accessible(p: Phase) -> bool;
}

/// Generic lazy interior data storage, uninitialized with interior mutability data storage
/// that call T::finaly when finalized
pub(crate) struct UnInited<T>(UnsafeCell<MaybeUninit<T>>);

impl<T: Finaly> Finaly for UnInited<T> {
    #[inline(always)]
    fn finaly(&self) {
        //SAFETY: UnInited is only used as part of GenericLazy, that gives access
        //only if the Sequentializer is a Lazy Sequentializer
        //
        //So the lazy Sequentializer should only execute finaly if the object initialization
        //succeeded
        unsafe { &*self.get() }.finaly();
    }
}

impl<T> UnInited<T> {
    pub const INIT: Self = Self(UnsafeCell::new(MaybeUninit::uninit()));
}

/// Generic lazy interior data storage, initialized with interior mutability data storage
/// that call T::finaly when finalized
pub(crate) struct Primed<T>(UnsafeCell<T>);

impl<T: Uninit> Finaly for Primed<T> {
    #[inline(always)]
    fn finaly(&self) {
        //SAFETY: UnInited is only used as part of GenericLazy, that gives access
        //only if the Sequentializer is a Lazy Sequentializer
        //
        //So the lazy Sequentializer should only execute finaly if the object initialization
        //succeeded
        unsafe { &mut *self.0.get() }.uninit();
    }
}

impl<T> Primed<T> {
    pub const fn prime(v: T) -> Self {
        Self(UnsafeCell::new(v))
    }
}

/// Generic lazy interior data storage, uninitialized with interior mutability data storage
/// that call drop when finalized
pub(crate) struct DropedUnInited<T>(UnsafeCell<MaybeUninit<T>>);

impl<T> Finaly for DropedUnInited<T> {
    #[inline(always)]
    fn finaly(&self) {
        //SAFETY: UnInited is only used as part of GenericLazy, that gives access
        //only if the Sequentializer is a Lazy Sequentializer
        //
        //So the lazy Sequentializer should only execute finaly if the object initialization
        //succeeded
        unsafe { self.get().drop_in_place() };
    }
}

impl<T> DropedUnInited<T> {
    pub const INIT: Self = Self(UnsafeCell::new(MaybeUninit::uninit()));
}

/// Trait implemented by generic lazy inner data.
///
/// Dereferencement of generic lazy will return a reference to
/// the inner data returned by the get method
pub(crate) trait LazyData {
    type Target;
    fn get(&self) -> *mut Self::Target;
    /// # Safety
    ///
    /// The reference to self should be unique
    unsafe fn init(&self, v: Self::Target);
    fn init_mut(&mut self, v: Self::Target);
}

impl<T> LazyData for UnInited<T> {
    type Target = T;
    #[inline(always)]
    fn get(&self) -> *mut T {
        self.0.get() as *mut T
    }
    #[inline(always)]
    unsafe fn init(&self, v: T) {
        self.get().write(v)
    }
    #[inline(always)]
    fn init_mut(&mut self, v: T) {
        *self.0.get_mut() = MaybeUninit::new(v)
    }
}

impl<T> LazyData for DropedUnInited<T> {
    type Target = T;
    #[inline(always)]
    fn get(&self) -> *mut T {
        self.0.get() as *mut T
    }
    #[inline(always)]
    unsafe fn init(&self, v: T) {
        self.get().write(v)
    }
    #[inline(always)]
    fn init_mut(&mut self, v: T) {
        *self.0.get_mut() = MaybeUninit::new(v)
    }
}

impl<T> LazyData for Primed<T> {
    type Target = T;
    #[inline(always)]
    fn get(&self) -> *mut T {
        self.0.get()
    }
    #[inline(always)]
    unsafe fn init(&self, v: T) {
        *self.get() = v
    }
    #[inline(always)]
    fn init_mut(&mut self, v: T) {
        *self.0.get_mut() = v
    }
}

/// Lazy access error
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct AccessError {
    pub phase: Phase,
}

impl Display for AccessError {
    fn fmt(&self, ft: &mut Formatter<'_>) -> fmt::Result {
        write!(ft, "Error: inaccessible lazy in {}", self.phase)
    }
}

#[cfg(feature = "parking_lot_core")]
impl std::error::Error for AccessError {}

pub(crate) struct GenericLazySeq<T, M> {
    value: T,
    sequentializer: M,
}

pub(crate) struct GenericLockedLazySeq<T, M> {
    value: T,
    sequentializer: M,
}

/// A type that wrap a Sequentializer and a raw data, and that may
/// initialize the data, at each access depending on the LazyPolicy
/// provided as generic argument.
pub(crate) struct GenericLazy<T, F, M, S> {
    seq: GenericLazySeq<T, M>,
    generator: F,
    phantom: PhantomData<S>,
    #[cfg(debug_mode)]
    _info: Option<StaticInfo>,
}

// SAFETY: The synchronization is ensured by the Sequentializer
//  1. GenericLazy fullfill the requirement that its sequentializer is a field
//  of itself as is its target data.
//  2. The sequentializer ensure that the initialization is atomic
unsafe impl<T: LazyData, M: Sync> Sync for GenericLazySeq<T, M> where <T as LazyData>::Target: Sync {}
unsafe impl<T: LazyData, M: Sync> Send for GenericLazySeq<T, M> where <T as LazyData>::Target: Send {}

#[cfg(any(feature = "parking_lot_core", debug_mode))]
impl<T: LazyData, M: RefUnwindSafe> RefUnwindSafe for GenericLazySeq<T, M> where
    <T as LazyData>::Target: RefUnwindSafe
{
}

// SAFETY: The synchronization is ensured by the Sequentializer
//  1. GenericLazy fullfill the requirement that its sequentializer is a field
//  of itself as is its target data.
//  2. The sequentializer ensure that the initialization is atomic
unsafe impl<T: LazyData, M: Sync> Sync for GenericLockedLazySeq<T, M> where
    <T as LazyData>::Target: Send
{
}
unsafe impl<T: LazyData, M: Sync> Send for GenericLockedLazySeq<T, M> where
    <T as LazyData>::Target: Send
{
}

#[cfg(any(feature = "parking_lot_core", debug_mode))]
impl<T: LazyData, M: RefUnwindSafe> RefUnwindSafe for GenericLockedLazySeq<T, M> where
    <T as LazyData>::Target: RefUnwindSafe
{
}

impl<T, F, M, S> GenericLazy<T, F, M, S> {
    #[inline(always)]
    /// const initialize the lazy, the inner data may be in an uninitialized state
    ///
    /// # Safety
    ///
    /// ## Constraint on T
    ///
    /// Should initialize the object when init is called and the method get should
    /// return a pointer without UB if the object is not initialized
    ///
    /// ## Constraint on P
    ///
    /// The parameter M should be a lazy sequentializer that ensure that:
    ///  1. When finalize is called, no other shared reference to the inner data exist
    ///  2. The finalization is run only if the object was previously initialized
    ///
    /// ## Constraint on F
    ///
    /// The parameter F should be a Generator that ensured that the object
    /// is accessble after a call to generate succeeds
    ///
    /// ## Constraint on S
    ///
    /// S should be a lazy policy that report correctly when the object
    /// is accessbile, this in adequation with M and F.
    pub const unsafe fn new(generator: F, sequentializer: M, value: T) -> Self {
        Self {
            seq: GenericLazySeq {
                value,
                sequentializer,
            },
            generator,
            phantom: PhantomData,
            #[cfg(debug_mode)]
            _info: None,
        }
    }
    #[inline(always)]
    /// const initialize the lazy, the inner data may be in an uninitialized state
    ///
    /// # Safety
    ///
    /// ## Constraint on T
    ///
    /// Should initialize the object when init is called and the method get should
    /// return a pointer without UB if the object is not initialized
    ///
    /// ## Constraint on P
    ///
    /// The parameter M should be a lazy sequentializer that ensure that:
    ///  1. When finalize is called, no other shared reference to the inner data exist
    ///  2. The finalization is run only if the object was previously initialized
    ///
    /// ## Constraint on F
    ///
    /// The parameter F should be a Generator that ensured that the object
    /// is accessble after a call to generate succeeds
    ///
    /// ## Constraint on S
    ///
    /// S should be a lazy policy that report correctly when the object
    /// is accessbile, this in adequation with M and F.
    pub const unsafe fn new_with_info(
        generator: F,
        sequentializer: M,
        value: T,
        _info: StaticInfo,
    ) -> Self {
        Self {
            seq: GenericLazySeq {
                value,
                sequentializer,
            },
            generator,
            phantom: PhantomData,
            #[cfg(debug_mode)]
            _info: Some(_info),
        }
    }
    #[inline(always)]
    ///get access to the sequentializer
    pub fn sequentializer(this: &Self) -> &M {
        &this.seq.sequentializer
    }
    #[inline(always)]
    ///get a pointer to the raw data
    pub fn get_raw_data(this: &Self) -> &T {
        &this.seq.value
    }
}
impl<'a, T, F, M, S> GenericLazy<T, F, M, S>
where
    T: 'a + LazyData,
    M: 'a,
    M: LazySequentializer<'a, GenericLazySeq<T, M>>,
    F: 'a + Generator<T::Target>,
    S: 'a + LazyPolicy,
{
    /// Get a reference to the target
    ///
    /// # Safety
    ///
    /// Undefined behaviour if the referenced value has not been initialized
    #[inline(always)]
    pub unsafe fn get_unchecked(&'a self) -> &'a T::Target {
        &*self.seq.value.get()
    }

    /// Get a reference to the target, returning an error if the
    /// target is not in the correct phase.
    #[inline(always)]
    pub fn try_get(&'a self) -> Result<&'a T::Target, AccessError> {
        check_access::<*mut T::Target, S>(
            self.seq.value.get(),
            Phased::phase(&self.seq.sequentializer),
        )
        .map(|ptr| unsafe { &*ptr })
    }

    /// Get a reference to the target
    ///
    /// # Panics
    ///
    /// Panic if the target is not in the correct phase
    #[inline(always)]
    pub fn get(&'a self) -> &'a T::Target {
        self.try_get().unwrap()
    }

    /// Get a mutable reference to the target
    ///
    /// # Safety
    ///
    /// Undefined behaviour if the referenced value has not been initialized
    #[inline(always)]
    pub unsafe fn get_mut_unchecked(&'a mut self) -> &'a mut T::Target {
        &mut *self.seq.value.get()
    }

    /// Get a mutable reference to the target, returning an error if the
    /// target is not in the correct phase.
    #[inline(always)]
    pub fn try_get_mut(&'a mut self) -> Result<&'a mut T::Target, AccessError> {
        check_access::<*mut T::Target, S>(
            self.seq.value.get(),
            Phased::phase(&self.seq.sequentializer),
        )
        .map(|ptr| unsafe { &mut *ptr })
    }

    /// Get a reference to the target
    ///
    /// # Panics
    ///
    /// Panic if the target is not in the correct phase
    #[inline(always)]
    pub fn get_mut(&'a mut self) -> &'a mut T::Target {
        self.try_get_mut().unwrap()
    }

    /// Attempt initialization then get a reference to the target
    ///
    /// # Safety
    ///
    /// Undefined behaviour if the referenced value has not been initialized
    #[inline(always)]
    pub unsafe fn init_then_get_unchecked(&'a self) -> &'a T::Target {
        self.init();
        self.get_unchecked()
    }
    /// Attempt initialization then get a reference to the target, returning an error if the
    /// target is not in the correct phase.
    #[inline(always)]
    pub fn init_then_try_get(&'a self) -> Result<&'a T::Target, AccessError> {
        let phase = self.init();
        post_init_check_access::<*mut T::Target, S>(self.seq.value.get(), phase)
            .map(|ptr| unsafe { &*ptr })
    }
    /// Attempt initialization then get a reference to the target, returning an error if the
    /// target is not in the correct phase.
    #[inline(always)]
    pub fn init_then_get(&'a self) -> &'a T::Target {
        Self::init_then_try_get(self).unwrap()
    }
    #[inline(always)]
    /// Potentialy initialize the inner data, returning the
    /// phase reached at the end of the initialization attempt
    pub fn init(&'a self) -> Phase {
        may_debug(
            || {
                <M as LazySequentializer<'a, GenericLazySeq<T, M>>>::init(
                    &self.seq,
                    S::shall_init,
                    |data: &T| {
                        // SAFETY
                        // This function is called only once within the init function
                        // Only one thread can ever get this mutable access
                        let d = Generator::generate(&self.generator);
                        unsafe { data.init(d) };
                    },
                )
            },
            #[cfg(debug_mode)]
            &self._info,
        )
    }
}

impl<T, F, M, S> GenericLazy<T, F, M, S>
where
    M: UniqueLazySequentializer<GenericLazySeq<T, M>>,
    T: LazyData,
    S: LazyPolicy,
    F: Generator<T::Target>,
{
    #[inline(always)]
    /// Attempt initialization then get a mutable reference to the target
    ///
    /// # Safety
    ///
    /// Undefined behaviour if the referenced value has not been initialized
    pub unsafe fn only_init_then_get_mut_unchecked(&mut self) -> &mut T::Target {
        self.only_init_unique();
        &mut *self.seq.value.get()
    }

    #[inline(always)]
    /// Attempt initialization then get a mutable reference to the target, returning an error if the
    /// target is not in the correct phase.
    pub fn only_init_then_try_get_mut(&mut self) -> Result<&mut T::Target, AccessError> {
        let phase = self.only_init_unique();
        post_init_check_access::<*mut T::Target, S>(self.seq.value.get(), phase)
            .map(|ptr| unsafe { &mut *ptr })
    }

    #[inline(always)]
    /// Attempt initialization then get a mutable reference to the target, returning an error if the
    /// target is not in the correct phase.
    pub fn only_init_then_get_mut(&mut self) -> &mut T::Target {
        Self::only_init_then_try_get_mut(self).unwrap()
    }

    #[inline(always)]
    /// Potentialy initialize the inner data, returning the
    /// phase reached at the end of the initialization attempt
    pub fn only_init_unique(&mut self) -> Phase {
        let generator = &self.generator;
        let seq = &mut self.seq;
        <M as UniqueLazySequentializer<GenericLazySeq<T, M>>>::init_unique(
            seq,
            S::shall_init,
            |data: &mut T| {
                // SAFETY
                // This function is called only once within the init function
                // Only one thread can ever get this mutable access
                let d = Generator::generate(generator);
                unsafe { data.init_mut(d) };
            },
        )
    }
}

//SAFETY: data and sequentialize are two fields of Self.
unsafe impl<T: LazyData, M> Sequential for GenericLazySeq<T, M> {
    type Data = T;
    type Sequentializer = M;
    #[inline(always)]
    fn sequentializer(this: &Self) -> &Self::Sequentializer {
        &this.sequentializer
    }
    #[inline(always)]
    fn sequentializer_data_mut(this: &mut Self) -> (&mut Self::Sequentializer, &mut Self::Data) {
        (&mut this.sequentializer, &mut this.value)
    }
    #[inline(always)]
    fn data(this: &Self) -> &Self::Data {
        &this.value
    }
}

#[must_use = "If unused the write lock is immediatly released"]
pub(crate) struct WriteGuard<T>(T);

impl<T> Deref for WriteGuard<T>
where
    T: Deref,
    <T as Deref>::Target: LazyData,
{
    type Target = <<T as Deref>::Target as LazyData>::Target;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe { &*(*self.0).get() }
    }
}
impl<T> DerefMut for WriteGuard<T>
where
    T: Deref,
    <T as Deref>::Target: LazyData,
{
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(*self.0).get() }
    }
}

impl<T> Phased for WriteGuard<T>
where
    T: Phased,
{
    #[inline(always)]
    fn phase(this: &Self) -> Phase {
        Phased::phase(&this.0)
    }
}

impl<T> Debug for WriteGuard<T>
where
    T: Deref,
    <T as Deref>::Target: LazyData,
    <<T as Deref>::Target as LazyData>::Target: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("WriteGuard").field(&*self).finish()
    }
}

#[must_use = "If unused the read lock is immediatly released"]
#[derive(Clone)]
pub(crate) struct ReadGuard<T>(T);

impl<T> Deref for ReadGuard<T>
where
    T: Deref,
    <T as Deref>::Target: LazyData,
{
    type Target = <<T as Deref>::Target as LazyData>::Target;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe { &*(*self.0).get() }
    }
}

impl<T> Debug for ReadGuard<T>
where
    T: Deref,
    <T as Deref>::Target: LazyData,
    <<T as Deref>::Target as LazyData>::Target: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ReadGuard").field(&*self).finish()
    }
}

impl<T, U> From<WriteGuard<T>> for ReadGuard<U>
where
    U: From<T>,
{
    #[inline(always)]
    fn from(v: WriteGuard<T>) -> Self {
        Self(v.0.into())
    }
}

impl<T> Phased for ReadGuard<T>
where
    T: Phased,
{
    #[inline(always)]
    fn phase(this: &Self) -> Phase {
        Phased::phase(&this.0)
    }
}

#[cfg(any(feature = "parking_lot_core", debug_mode))]
impl<T: LazyData> RefUnwindSafe for ReadGuard<T> where <T as LazyData>::Target: RefUnwindSafe {}

#[cfg(any(feature = "parking_lot_core", debug_mode))]
impl<T: LazyData> RefUnwindSafe for WriteGuard<T> where <T as LazyData>::Target: RefUnwindSafe {}

/// A type that wrap a Sequentializer and a raw data, and that may
/// initialize the data, at each access depending on the LazyPolicy
/// provided as generic argument.
pub(crate) struct GenericLockedLazy<T, F, M, S> {
    seq: GenericLockedLazySeq<T, M>,
    generator: F,
    phantom: PhantomData<S>,
    #[cfg(debug_mode)]
    _info: Option<StaticInfo>,
}

impl<T, F, M, S> GenericLockedLazy<T, F, M, S> {
    #[inline(always)]
    /// const initialize the lazy, the inner data may be in an uninitialized state
    ///
    /// # Safety
    ///
    /// ## Constraint on T
    ///
    /// Should initialize the object when init is called and the method get should
    /// return a pointer without UB if the object is not initialized
    ///
    /// ## Constraint on P
    ///
    /// The parameter M should be a lazy sequentializer that ensure that:
    ///  1. When finalize is called, no other shared reference to the inner data exist
    ///  2. The finalization is run only if the object was previously initialized
    ///
    /// ## Constraint on F
    ///
    /// The parameter F should be a Generator that ensured that the object
    /// is accessble after a call to generate succeeds
    ///
    /// ## Constraint on S
    ///
    /// S should be a lazy policy that report correctly when the object
    /// is accessbile, this in adequation with M and F.
    pub const unsafe fn new(generator: F, sequentializer: M, value: T) -> Self {
        Self {
            seq: GenericLockedLazySeq {
                value,
                sequentializer,
            },
            generator,
            phantom: PhantomData,
            #[cfg(debug_mode)]
            _info: None,
        }
    }
    #[inline(always)]
    /// const initialize the lazy, the inner data may be in an uninitialized state
    ///
    /// # Safety
    ///
    /// ## Constraint on T
    ///
    /// Should initialize the object when init is called and the method get should
    /// return a pointer without UB if the object is not initialized
    ///
    /// ## Constraint on P
    ///
    /// The parameter M should be a lazy sequentializer that ensure that:
    ///  1. When finalize is called, no other shared reference to the inner data exist
    ///  2. The finalization is run only if the object was previously initialized
    ///
    /// ## Constraint on F
    ///
    /// The parameter F should be a Generator that ensured that the object
    /// is accessble after a call to generate succeeds
    ///
    /// ## Constraint on S
    ///
    /// S should be a lazy policy that report correctly when the object
    /// is accessbile, this in adequation with M and F.
    pub const unsafe fn new_with_info(
        generator: F,
        sequentializer: M,
        value: T,
        _info: StaticInfo,
    ) -> Self {
        Self {
            seq: GenericLockedLazySeq {
                value,
                sequentializer,
            },
            generator,
            phantom: PhantomData,
            #[cfg(debug_mode)]
            _info: Some(_info),
        }
    }
    #[inline(always)]
    ///get access to the sequentializer
    pub fn sequentializer(this: &Self) -> &M {
        &this.seq.sequentializer
    }
}
impl<'a, T, F, M, S> GenericLockedLazy<T, F, M, S>
where
    T: 'a + LazyData,
    M: 'a,
    M: LazySequentializer<'a, GenericLockedLazySeq<T, M>>,
    F: 'a + Generator<T::Target>,
    S: 'a + LazyPolicy,
    M::ReadGuard: Phased,
    M::WriteGuard: Phased,
{
    /// Get a mutable reference to the target
    ///
    /// # Safety
    ///
    /// Undefined behaviour if the referenced value has not been initialized
    #[inline(always)]
    pub unsafe fn get_mut_unchecked(&'a mut self) -> &'a mut T::Target {
        &mut *self.seq.value.get()
    }

    /// Get a mutable reference to the target, returning an error if the
    /// target is not in the correct phase.
    #[inline(always)]
    pub fn try_get_mut(&'a mut self) -> Result<&'a mut T::Target, AccessError> {
        check_access::<*mut T::Target, S>(
            self.seq.value.get(),
            Phased::phase(&self.seq.sequentializer),
        )
        .map(|ptr| unsafe { &mut *ptr })
    }

    /// Get a reference to the target
    ///
    /// # Panics
    ///
    /// Panic if the target is not in the correct phase
    #[inline(always)]
    pub fn get_mut(&'a mut self) -> &'a mut T::Target {
        self.try_get_mut().unwrap()
    }
    /// Attempt to get a read lock the LazyData object (not the target), returning None
    /// if a unique lock is already held or in high contention cases.
    ///
    /// # Safety
    ///
    /// The obtained [ReadGuard] may reference an uninitialized target.
    #[inline(always)]
    pub unsafe fn fast_read_lock_unchecked(this: &'a Self) -> Option<ReadGuard<M::ReadGuard>> {
        <M as Sequentializer<'a, GenericLockedLazySeq<T, M>>>::try_lock(
            &this.seq,
            |_| LockNature::Read,
            M::INITIALIZED_HINT,
        )
        .map(|l| {
            if let LockResult::Read(l) = l {
                ReadGuard(l)
            } else {
                unreachable_unchecked()
            }
        })
    }
    /// Attempt to get a read lock the LazyData object (not the target), returning None
    /// if a unique lock is already held or in high contention cases.
    ///
    /// If the lock succeeds and the object is not in an accessible phase, some error is returned
    #[inline(always)]
    pub fn fast_try_read_lock(
        this: &'a Self,
    ) -> Option<Result<ReadGuard<M::ReadGuard>, AccessError>> {
        unsafe { Self::fast_read_lock_unchecked(this) }
            .map(checked_access::<ReadGuard<M::ReadGuard>, S>)
    }

    /// Attempt to get a read lock the LazyData object (not the target), returning None
    /// if a unique lock is already held or in high contention cases.
    ///
    /// # Panics
    ///
    /// Panics if the lock succeeds and the object is not in an accessible phase.
    #[inline(always)]
    pub fn fast_read_lock(this: &'a Self) -> Option<ReadGuard<M::ReadGuard>> {
        Self::fast_try_read_lock(this).map(|r| r.unwrap())
    }

    /// Get a read lock the LazyData object (not the target)
    ///
    /// # Safety
    ///
    /// The obtained [ReadGuard] may reference an uninitialized target.
    #[inline(always)]
    pub unsafe fn read_lock_unchecked(this: &'a Self) -> ReadGuard<M::ReadGuard> {
        if let LockResult::Read(l) = <M as Sequentializer<'a, GenericLockedLazySeq<T, M>>>::lock(
            &this.seq,
            |_| LockNature::Read,
            M::INITIALIZED_HINT,
        ) {
            ReadGuard(l)
        } else {
            unreachable_unchecked()
        }
    }

    /// Get a read lock the LazyData object (not the target)
    ///
    /// If the object is not in an accessible phase, some error is returned
    #[inline(always)]
    pub fn try_read_lock(this: &'a Self) -> Result<ReadGuard<M::ReadGuard>, AccessError> {
        checked_access::<ReadGuard<M::ReadGuard>, S>(unsafe { Self::read_lock_unchecked(this) })
    }

    /// Get a read lock the LazyData object (not the target).
    ///
    /// # Panics
    ///
    /// Panics if the lock succeeds and the object is not in an accessible phase.
    #[inline(always)]
    pub fn read_lock(this: &'a Self) -> ReadGuard<M::ReadGuard> {
        Self::try_read_lock(this).unwrap()
    }

    /// Attempt to get a write lock the LazyData object (not the target), returning None
    /// if a lock is already held or in high contention cases.
    ///
    /// # Safety
    ///
    /// The obtained [ReadGuard] may reference an uninitialized target.
    #[inline(always)]
    pub unsafe fn fast_write_lock_unchecked(this: &'a Self) -> Option<WriteGuard<M::WriteGuard>> {
        <M as Sequentializer<'a, GenericLockedLazySeq<T, M>>>::try_lock(
            &this.seq,
            |_| LockNature::Write,
            M::INITIALIZED_HINT,
        )
        .map(|l| {
            if let LockResult::Write(l) = l {
                WriteGuard(l)
            } else {
                unreachable_unchecked()
            }
        })
    }

    /// Attempt to get a write lock the LazyData object (not the target), returning None
    /// if a lock is already held or in high contention cases.
    ///
    /// If the lock succeeds and the object is not in an accessible phase, some error is returned
    #[inline(always)]
    pub fn fast_try_write_lock(
        this: &'a Self,
    ) -> Option<Result<WriteGuard<M::WriteGuard>, AccessError>> {
        unsafe { Self::fast_write_lock_unchecked(this) }
            .map(checked_access::<WriteGuard<M::WriteGuard>, S>)
    }

    /// Attempt to get a write lock the LazyData object (not the target), returning None
    /// if a lock is already held or in high contention cases.
    ///
    /// # Panics
    ///
    /// Panics if the lock succeeds and the object is not in an accessible phase.
    #[inline(always)]
    pub fn fast_write_lock(this: &'a Self) -> Option<WriteGuard<M::WriteGuard>> {
        Self::fast_try_write_lock(this).map(|r| r.unwrap())
    }

    /// Get a write lock the LazyData object (not the target)
    ///
    /// # Safety
    ///
    /// The obtained [ReadGuard] may reference an uninitialized target.
    #[inline(always)]
    pub unsafe fn write_lock_unchecked(this: &'a Self) -> WriteGuard<M::WriteGuard> {
        if let LockResult::Write(l) = <M as Sequentializer<'a, GenericLockedLazySeq<T, M>>>::lock(
            &this.seq,
            |_| LockNature::Write,
            M::INITIALIZED_HINT,
        ) {
            WriteGuard(l)
        } else {
            unreachable_unchecked()
        }
    }

    /// Get a read lock the LazyData object (not the target)
    ///
    /// If the object is not in an accessible phase, an error is returned
    #[inline(always)]
    pub fn try_write_lock(this: &'a Self) -> Result<WriteGuard<M::WriteGuard>, AccessError> {
        checked_access::<WriteGuard<M::WriteGuard>, S>(unsafe { Self::write_lock_unchecked(this) })
    }

    /// Get a write lock the LazyData object (not the target).
    ///
    /// # Panics
    ///
    /// Panics if the lock succeeds and the object is not in an accessible phase.
    #[inline(always)]
    pub fn write_lock(this: &'a Self) -> WriteGuard<M::WriteGuard> {
        Self::try_write_lock(this).unwrap()
    }

    #[inline(always)]
    /// Initialize if necessary then return a read lock
    ///
    /// # Safety
    ///
    /// Undefined behaviour if after initialization the return object is not in an accessible
    /// state.
    pub unsafe fn init_then_read_lock_unchecked(this: &'a Self) -> ReadGuard<M::ReadGuard> {
        let r = may_debug(
            || {
                <M as LazySequentializer<'a, GenericLockedLazySeq<T, M>>>::init_then_read_guard(
                    &this.seq,
                    S::shall_init,
                    |data: &T| {
                        // SAFETY
                        // This function is called only once within the init function
                        // Only one thread can ever get this mutable access
                        let d = Generator::generate(&this.generator);
                        #[allow(unused_unsafe)]
                        unsafe {
                            data.init(d)
                        };
                    },
                )
            },
            #[cfg(debug_mode)]
            &this._info,
        );
        ReadGuard(r)
    }

    /// Initialize if necessary then return a read lock
    ///
    /// Returns an error if after initialization the return object is not in an accessible
    /// state.
    #[inline(always)]
    pub fn init_then_try_read_lock(this: &'a Self) -> Result<ReadGuard<M::ReadGuard>, AccessError> {
        post_init_checked_access::<ReadGuard<M::ReadGuard>, S>(unsafe {
            Self::init_then_read_lock_unchecked(this)
        })
    }

    /// Initialize if necessary then return a read lock
    ///
    /// # Panics
    ///
    /// Panics if after initialization the return object is not in an accessible
    /// state.
    #[inline(always)]
    pub fn init_then_read_lock(this: &'a Self) -> ReadGuard<M::ReadGuard> {
        Self::init_then_try_read_lock(this).unwrap()
    }

    /// If necessary attempt to get a write_lock initilialize the object then turn the write
    /// lock into a read lock, otherwise attempt to get directly a read_lock. Attempt to take
    /// a lock may fail because other locks are held or because of contention.
    ///
    /// # Safety
    ///
    /// If the target is not accessible this may cause undefined behaviour.
    #[inline(always)]
    pub unsafe fn fast_init_then_read_lock_unchecked(
        this: &'a Self,
    ) -> Option<ReadGuard<M::ReadGuard>> {
        may_debug(
            || {
                <M as LazySequentializer<'a, GenericLockedLazySeq<T, M>>>::try_init_then_read_guard(
                    &this.seq,
                    S::shall_init,
                    |data: &T| {
                        // SAFETY
                        // This function is called only once within the init function
                        // Only one thread can ever get this mutable access
                        let d = Generator::generate(&this.generator);
                        #[allow(unused_unsafe)]
                        unsafe {
                            data.init(d)
                        };
                    },
                )
            },
            #[cfg(debug_mode)]
            &this._info,
        )
        .map(ReadGuard)
    }

    #[inline(always)]
    /// If necessary attempt to get a write_lock initilialize the object then turn the write
    /// lock into a read lock, otherwise attempt to get directly a read_lock. Attempt to take
    /// a lock may fail because other locks are held or because of contention.
    ///
    /// If the target is not accessible some error is returned.
    pub fn fast_init_then_try_read_lock(
        this: &'a Self,
    ) -> Option<Result<ReadGuard<M::ReadGuard>, AccessError>> {
        unsafe { Self::fast_init_then_read_lock_unchecked(this) }
            .map(post_init_checked_access::<ReadGuard<M::ReadGuard>, S>)
    }

    #[inline(always)]
    /// If necessary attempt to get a write_lock initilialize the object then turn the write
    /// lock into a read lock, otherwise attempt to get directly a read_lock. Attempt to take
    /// a lock may fail because other locks are held or because of contention.
    ///
    /// # Panics
    ///
    /// If the target is not accessible some error is returned.
    pub fn fast_init_then_read_lock(this: &'a Self) -> Option<ReadGuard<M::ReadGuard>> {
        Self::fast_init_then_try_read_lock(this).map(|r| r.unwrap())
    }

    #[inline(always)]
    /// Get a write locks, initialize the target if necessary then returns a readlock.
    ///
    /// # Safety
    ///
    /// If the target object is not accessible, this will cause undefined behaviour
    pub unsafe fn init_then_write_lock_unchecked(this: &'a Self) -> WriteGuard<M::WriteGuard> {
        let r = may_debug(
            || {
                <M as LazySequentializer<'a, GenericLockedLazySeq<T, M>>>::init_then_write_guard(
                    &this.seq,
                    S::shall_init,
                    |data: &T| {
                        // SAFETY
                        // This function is called only once within the init function
                        // Only one thread can ever get this mutable access
                        let d = Generator::generate(&this.generator);
                        #[allow(unused_unsafe)]
                        unsafe {
                            data.init(d)
                        };
                    },
                )
            },
            #[cfg(debug_mode)]
            &this._info,
        );
        WriteGuard(r)
    }

    #[inline(always)]
    /// Get a write locks, initialize the target if necessary then returns the write lock.
    ///
    /// If the target object is not accessible an error is returned.
    pub fn init_then_try_write_lock(
        this: &'a Self,
    ) -> Result<WriteGuard<M::WriteGuard>, AccessError> {
        post_init_checked_access::<WriteGuard<M::WriteGuard>, S>(unsafe {
            Self::init_then_write_lock_unchecked(this)
        })
    }
    #[inline(always)]
    /// Get a write locks, initialize the target if necessary then returns a write lock.
    ///
    /// Panics if the target object is not accessible.
    #[inline(always)]
    pub fn init_then_write_lock(this: &'a Self) -> WriteGuard<M::WriteGuard> {
        Self::init_then_try_write_lock(this).unwrap()
    }

    #[inline(always)]
    /// Attempt to get a write locks then initialize the target if necessary and returns the
    /// writelock.
    ///
    /// # Safety
    ///
    /// Undefined behavior if the target object is not accessible.
    pub unsafe fn fast_init_then_write_lock_unchecked(
        this: &'a Self,
    ) -> Option<WriteGuard<M::WriteGuard>> {
        may_debug(
            || {
                <M as LazySequentializer<'a, GenericLockedLazySeq<T, M>>>::try_init_then_write_guard(
                    &this.seq,
                    S::shall_init,
                    |data: &T| {
                        // SAFETY
                        // This function is called only once within the init function
                        // Only one thread can ever get this mutable access
                        let d = Generator::generate(&this.generator);
                        #[allow(unused_unsafe)]
                        unsafe { data.init(d) };
                    },
                )
            },
            #[cfg(debug_mode)]
            &this._info,
        )
        .map(WriteGuard)
    }
    /// Attempt to get a write locks then initialize the target if necessary and returns the
    /// writelock.
    ///
    /// Returns an error if the target object is not accessible.
    #[inline(always)]
    pub fn fast_init_then_try_write_lock(
        this: &'a Self,
    ) -> Option<Result<WriteGuard<M::WriteGuard>, AccessError>> {
        unsafe { Self::fast_init_then_write_lock_unchecked(this) }
            .map(post_init_checked_access::<WriteGuard<M::WriteGuard>, S>)
    }
    /// Attempt to get a write locks then initialize the target if necessary and returns the
    /// writelock.
    ///
    /// # Panics
    ///
    /// Panics if the target object is not accessible.
    #[inline(always)]
    pub fn fast_init_then_write_lock(this: &'a Self) -> Option<WriteGuard<M::WriteGuard>> {
        Self::fast_init_then_try_write_lock(this).map(|r| r.unwrap())
    }
}

impl<T, F, M, S> GenericLockedLazy<T, F, M, S>
where
    M: UniqueLazySequentializer<GenericLockedLazySeq<T, M>>,
    T: LazyData,
    S: LazyPolicy,
    F: Generator<T::Target>,
{
    #[inline(always)]
    /// Attempt initialization then get a mutable reference to the target
    ///
    /// # Safety
    ///
    /// Undefined behaviour if the referenced value has not been initialized
    pub unsafe fn only_init_then_get_mut_unchecked(&mut self) -> &mut T::Target {
        self.only_init_unique();
        &mut *self.seq.value.get()
    }
    #[inline(always)]
    /// Attempt initialization then get a mutable reference to the target, returning an error if the
    /// target is not in the correct phase.
    pub fn only_init_then_try_get_mut(&mut self) -> Result<&mut T::Target, AccessError> {
        let phase = self.only_init_unique();
        check_access::<*mut T::Target, S>(self.seq.value.get(), phase)
            .map(|ptr| unsafe { &mut *ptr })
    }
    #[inline(always)]
    /// Attempt initialization then get a mutable reference to the target, returning an error if the
    /// target is not in the correct phase.
    pub fn only_init_then_get_mut(&mut self) -> &mut T::Target {
        Self::only_init_then_try_get_mut(self).unwrap()
    }
    #[inline(always)]
    /// Potentialy initialize the inner data, returning the
    /// phase reached at the end of the initialization attempt
    pub fn only_init_unique(&mut self) -> Phase {
        let generator = &self.generator;
        let seq = &mut self.seq;
        <M as UniqueLazySequentializer<GenericLockedLazySeq<T, M>>>::init_unique(
            seq,
            S::shall_init,
            |data: &mut T| {
                // SAFETY
                // This function is called only once within the init function
                // Only one thread can ever get this mutable access
                let d = Generator::generate(generator);
                unsafe { data.init_mut(d) };
            },
        )
    }
}

//SAFETY: data and sequentialize are two fields of Self.
unsafe impl<T: LazyData, M> Sequential for GenericLockedLazySeq<T, M> {
    type Data = T;
    type Sequentializer = M;
    #[inline(always)]
    fn sequentializer(this: &Self) -> &Self::Sequentializer {
        &this.sequentializer
    }
    #[inline(always)]
    fn sequentializer_data_mut(this: &mut Self) -> (&mut Self::Sequentializer, &mut Self::Data) {
        (&mut this.sequentializer, &mut this.value)
    }
    #[inline(always)]
    fn data(this: &Self) -> &Self::Data {
        &this.value
    }
}
impl<F, T, M, S> Deref for GenericLockedLazy<T, F, M, S> {
    type Target = T;
    #[inline(always)]
    ///get a pointer to the raw data
    fn deref(&self) -> &T {
        &self.seq.value
    }
}
impl<T, M> Deref for GenericLockedLazySeq<T, M> {
    type Target = T;
    #[inline(always)]
    ///get a pointer to the raw data
    fn deref(&self) -> &T {
        &self.value
    }
}

impl<F, T, M: Phased, S> Phased for GenericLockedLazy<T, F, M, S> {
    #[inline(always)]
    fn phase(this: &Self) -> Phase {
        Phased::phase(&this.seq.sequentializer)
    }
}
impl<F, T, M: Phased, S> Phased for GenericLazy<T, F, M, S> {
    #[inline(always)]
    fn phase(this: &Self) -> Phase {
        Phased::phase(&this.seq.sequentializer)
    }
}

#[inline(always)]
fn may_debug<R, F: FnOnce() -> R>(f: F, #[cfg(debug_mode)] info: &Option<StaticInfo>) -> R {
    #[cfg(not(debug_mode))]
    {
        f()
    }
    #[cfg(debug_mode)]
    {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f())) {
            Ok(r) => r,
            Err(x) => {
                if x.is::<CyclicPanic>() {
                    match info {
                        Some(info) => panic!("Circular initialization of {:#?}", info),
                        None => panic!("Circular lazy initialization detected"),
                    }
                } else {
                    std::panic::resume_unwind(x)
                }
            }
        }
    }
}

#[inline(always)]
fn check_access<T, S: LazyPolicy>(l: T, phase: Phase) -> Result<T, AccessError> {
    if S::is_accessible(phase) {
        Ok(l)
    } else {
        Err(AccessError { phase })
    }
}

#[inline(always)]
fn checked_access<T: Phased, S: LazyPolicy>(l: T) -> Result<T, AccessError> {
    let phase = Phased::phase(&l);
    check_access::<T, S>(l, phase)
}
#[inline(always)]
fn post_init_check_access<T, S: LazyPolicy>(l: T, phase: Phase) -> Result<T, AccessError> {
    if S::post_init_is_accessible(phase) {
        Ok(l)
    } else {
        Err(AccessError { phase })
    }
}

#[inline(always)]
fn post_init_checked_access<T: Phased, S: LazyPolicy>(l: T) -> Result<T, AccessError> {
    let phase = Phased::phase(&l);
    post_init_check_access::<T, S>(l, phase)
}
