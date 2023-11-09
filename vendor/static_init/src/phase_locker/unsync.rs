use super::{LockNature, LockResult, Mappable, MutPhaseLocker, PhaseGuard, PhaseLocker};
use crate::phase::*;
use crate::{Phase, Phased};
use core::cell::Cell;
use core::mem::forget;
use core::ops::Deref;

#[cfg(any(feature = "parking_lot_core", debug_mode))]
use std::panic::RefUnwindSafe;

/// A kind of RefCell that is also phase locker.
pub(crate) struct UnSyncPhaseLocker(Cell<u32>);

/// Equivalent to std::cell::Ref.
pub(crate) struct UnSyncPhaseGuard<'a, T: ?Sized>(&'a T, &'a Cell<u32>, Phase);

/// Equivalent to std::cell::RefMut that implements PhaseLocker.
pub(crate) struct UnSyncReadPhaseGuard<'a, T: ?Sized>(&'a T, &'a Cell<u32>);

impl<'a, T> Deref for UnSyncPhaseGuard<'a, T> {
    type Target = T;
    #[inline(always)]
    fn deref(&self) -> &T {
        self.0
    }
}

impl<'a, T> Phased for UnSyncPhaseGuard<'a, T> {
    fn phase(this: &Self) -> Phase {
        this.2
    }
}

impl<'a, T: ?Sized> UnSyncPhaseGuard<'a, T> {
    #[inline(always)]
    pub(crate) fn new(r: &'a T, p: &'a Cell<u32>) -> Self {
        Self(r, p, Phase::from_bits_truncate(p.get()))
    }

    #[inline(always)]
    pub fn map<S: ?Sized>(self, f: impl FnOnce(&'a T) -> &'a S) -> UnSyncPhaseGuard<'a, S> {
        let p = UnSyncPhaseGuard(f(self.0), self.1, self.2);
        forget(self);
        p
    }
}
impl<'a, T: 'a, U: 'a> Mappable<T, U, UnSyncPhaseGuard<'a, U>> for UnSyncPhaseGuard<'a, T> {
    #[inline(always)]
    fn map<F: FnOnce(&'a T) -> &'a U>(self, f: F) -> UnSyncPhaseGuard<'a, U> {
        Self::map(self, f)
    }
}

unsafe impl<'a, T: ?Sized> PhaseGuard<'a, T> for UnSyncPhaseGuard<'a, T> {
    #[inline(always)]
    fn set_phase(&mut self, p: Phase) {
        self.2 = p;
    }
    #[inline(always)]
    fn commit_phase(&mut self) {
        self.1.set(self.2.bits() | LOCKED_BIT);
    }
    #[inline(always)]
    fn phase(&self) -> Phase {
        self.2
    }
    #[inline(always)]
    fn transition<R>(
        &mut self,
        f: impl FnOnce(&'a T) -> R,
        on_success: Phase,
        on_panic: Phase,
    ) -> R {
        self.2 = on_panic;
        let res = f(self.0);
        self.2 = on_success;
        res
    }
}

#[cfg(any(feature = "parking_lot_core", debug_mode))]
impl<'a, T: RefUnwindSafe> RefUnwindSafe for UnSyncPhaseGuard<'a, T> {}

impl<'a, T: ?Sized> From<UnSyncPhaseGuard<'a, T>> for UnSyncReadPhaseGuard<'a, T> {
    #[inline(always)]
    fn from(l: UnSyncPhaseGuard<'a, T>) -> UnSyncReadPhaseGuard<'a, T> {
        l.1.set(l.2.bits() | READER_UNITY);
        let r = UnSyncReadPhaseGuard(l.0, l.1);
        forget(l);
        r
    }
}

impl<'a, T: ?Sized> Drop for UnSyncPhaseGuard<'a, T> {
    #[inline(always)]
    fn drop(&mut self) {
        self.1.set(self.2.bits());
    }
}

impl<'a, T> Deref for UnSyncReadPhaseGuard<'a, T> {
    type Target = T;
    #[inline(always)]
    fn deref(&self) -> &T {
        self.0
    }
}

impl<'a, T> Phased for UnSyncReadPhaseGuard<'a, T> {
    fn phase(this: &Self) -> Phase {
        this.phase()
    }
}
impl<'a, T> Clone for UnSyncReadPhaseGuard<'a, T> {
    fn clone(&self) -> Self {
        self.1.set(self.1.get().checked_add(READER_UNITY).unwrap());
        Self(self.0, self.1)
    }
}

impl<'a, T: ?Sized> UnSyncReadPhaseGuard<'a, T> {
    #[inline(always)]
    pub(crate) fn new(r: &'a T, p: &'a Cell<u32>) -> Self {
        Self(r, p)
    }
    #[inline(always)]
    pub fn phase(&self) -> Phase {
        Phase::from_bits_truncate(self.1.get())
    }
    #[inline(always)]
    pub fn map<S: ?Sized>(self, f: impl FnOnce(&'a T) -> &'a S) -> UnSyncReadPhaseGuard<'a, S> {
        let p = UnSyncReadPhaseGuard(f(self.0), self.1);
        forget(self);
        p
    }
}
impl<'a, T: 'a, U: 'a> Mappable<T, U, UnSyncReadPhaseGuard<'a, U>> for UnSyncReadPhaseGuard<'a, T> {
    #[inline(always)]
    fn map<F: FnOnce(&'a T) -> &'a U>(self, f: F) -> UnSyncReadPhaseGuard<'a, U> {
        Self::map(self, f)
    }
}

#[cfg(any(feature = "parking_lot_core", debug_mode))]
impl<'a, T> RefUnwindSafe for UnSyncReadPhaseGuard<'a, T> {}

impl<'a, T: ?Sized> Drop for UnSyncReadPhaseGuard<'a, T> {
    #[inline(always)]
    fn drop(&mut self) {
        self.1.set(self.1.get() - READER_UNITY);
    }
}

// UnSyncPhasedLocker
// ---------------
//
//
unsafe impl MutPhaseLocker for UnSyncPhaseLocker {
    #[inline(always)]
    fn get_phase_unique(&mut self) -> Phase {
        Phase::from_bits(self.0.get()).unwrap()
    }

    #[inline(always)]
    fn set_phase(&mut self, p: Phase) {
        *self.0.get_mut() = p.bits();
    }

    #[inline(always)]
    fn transition<R>(&mut self, f: impl FnOnce() -> R, on_success: Phase, on_panic: Phase) -> R {
        self.0.set(on_panic.bits());
        let r = f();
        self.0.set(on_success.bits());
        r
    }
}
unsafe impl<'a, T: 'a> PhaseLocker<'a, T> for UnSyncPhaseLocker {
    type ReadGuard = UnSyncReadPhaseGuard<'a, T>;
    type WriteGuard = UnSyncPhaseGuard<'a, T>;

    #[inline(always)]
    fn lock<FL: Fn(Phase) -> LockNature, FW: Fn(Phase) -> LockNature>(
        &'a self,
        value: &'a T,
        lock_nature: FL,
        _on_wake_nature: FW,
        _hint: Phase,
    ) -> LockResult<Self::ReadGuard, Self::WriteGuard> {
        Self::lock(self, value, lock_nature)
    }
    #[inline(always)]
    fn lock_mut(&'a mut self, value: &'a T) -> Self::WriteGuard {
        Self::lock_mut(self, value)
    }
    #[inline(always)]
    fn try_lock<F: Fn(Phase) -> LockNature>(
        &'a self,
        value: &'a T,
        lock_nature: F,
        _hint: Phase,
    ) -> Option<LockResult<Self::ReadGuard, Self::WriteGuard>> {
        Self::try_lock(self, value, lock_nature)
    }
    #[inline(always)]
    fn phase(&self) -> Phase {
        Self::phase(self)
    }
}

impl Phased for UnSyncPhaseLocker {
    fn phase(this: &Self) -> Phase {
        this.phase()
    }
}

impl UnSyncPhaseLocker {
    #[inline(always)]
    pub const fn new(p: Phase) -> Self {
        Self(Cell::new(p.bits()))
    }
    #[inline(always)]
    /// Return the current (phase)[crate::Phase].
    pub fn phase(&self) -> Phase {
        Phase::from_bits_truncate(self.0.get())
    }
    #[inline(always)]
    /// Return a lock whose nature depends on 'lock_nature'
    ///
    /// # Panic
    ///
    /// Panic if an attempt to get a read or write lock is made
    /// while a write_lock is already held or if an attempt is made
    /// to get a write_lock if any read or write lock is held.
    pub fn try_lock<'a, T: ?Sized>(
        &'a self,
        v: &'a T,
        lock_nature: impl Fn(Phase) -> LockNature,
    ) -> Option<LockResult<UnSyncReadPhaseGuard<'_, T>, UnSyncPhaseGuard<'_, T>>> {
        match lock_nature(self.phase()) {
            LockNature::Write => {
                if self.0.get() & (LOCKED_BIT | READER_BITS) != 0 {
                    None
                } else {
                    self.0.set(self.0.get() | LOCKED_BIT);
                    Some(LockResult::Write(UnSyncPhaseGuard::new(v, &self.0)))
                }
            }
            LockNature::Read => {
                if self.0.get() & LOCKED_BIT != 0 || self.0.get() & READER_BITS == READER_BITS {
                    None
                } else {
                    self.0.set(self.0.get().checked_add(READER_UNITY).unwrap());
                    Some(LockResult::Read(UnSyncReadPhaseGuard::new(v, &self.0)))
                }
            }
            LockNature::None => Some(LockResult::None(self.phase())),
        }
    }
    #[inline(always)]
    /// Return a mutable phase lock
    pub fn lock_mut<'a, T: ?Sized>(&'a mut self, v: &'a T) -> UnSyncPhaseGuard<'_, T> {
        self.0.set(self.0.get() | LOCKED_BIT);
        UnSyncPhaseGuard::new(v, &self.0)
    }
    #[inline(always)]
    /// Return a lock whose nature depends on 'lock_nature'
    ///
    /// # Panic
    ///
    /// Panic if an attempt to get a read or write lock is made
    /// while a write_lock is already held or if an attempt is made
    /// to get a write_lock if any read or write lock is held.
    pub fn lock<'a, T: ?Sized>(
        &'a self,
        v: &'a T,
        lock_nature: impl Fn(Phase) -> LockNature,
    ) -> LockResult<UnSyncReadPhaseGuard<'_, T>, UnSyncPhaseGuard<'_, T>> {
        match lock_nature(self.phase()) {
            LockNature::Write => {
                assert_eq!(
                    self.0.get() & (LOCKED_BIT | READER_BITS),
                    0,
                    "Cannot get a mutable reference if it is already mutably borrowed"
                );
                self.0.set(self.0.get() | LOCKED_BIT);
                LockResult::Write(UnSyncPhaseGuard::new(v, &self.0))
            }
            LockNature::Read => {
                assert_eq!(
                    self.0.get() & LOCKED_BIT,
                    0,
                    "Cannot get a shared reference if it is alread mutably borrowed"
                );
                assert_ne!(
                    self.0.get() & (READER_BITS),
                    READER_BITS,
                    "Maximal number of shared borrow reached."
                );
                self.0.set(self.0.get().checked_add(READER_UNITY).unwrap());
                LockResult::Read(UnSyncReadPhaseGuard::new(v, &self.0))
            }
            LockNature::None => LockResult::None(self.phase()),
        }
    }
}

#[cfg(any(feature = "parking_lot_core", debug_mode))]
impl RefUnwindSafe for UnSyncPhaseLocker {}
