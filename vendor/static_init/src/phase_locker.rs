#[cfg(not(feature = "spin_loop"))]
mod futex;

mod spin_wait;

#[cfg(not(feature = "spin_loop"))]
mod sync;

#[cfg(feature = "spin_loop")]
#[path = "phase_locker/sync_spinning.rs"]
mod sync;

pub(crate) use sync::Mutex;
pub(crate) use sync::{SyncPhaseGuard, SyncPhaseLocker, SyncReadPhaseGuard};

mod unsync;
pub(crate) use unsync::UnSyncPhaseLocker;
pub(crate) use unsync::{UnSyncPhaseGuard, UnSyncReadPhaseGuard};

use super::{Phase, Phased};

/// A phase guard ensure that the target object will
/// performed atomic phase transition
///
/// # Safety
///
/// The trait is unsafe because the implementation must fullfill the
/// following requirement described in the documentation of the functions
pub(crate) unsafe trait PhaseGuard<'a, T: ?Sized + 'a> {
    /// Set the phase at which will be the traget object
    /// when the phase guard will be dropped
    ///
    /// # Safety
    ///
    /// This function is unsafe because not providing a correct phase
    /// may lead to miss interpretation of the state of the target object
    /// which may result in unsound lazy
    fn set_phase(&mut self, p: Phase);
    /// Set the phase of the target object with release semantic if the
    /// PhaseGuard is Sync
    fn commit_phase(&mut self);
    /// Return the phase at which will be the object
    fn phase(&self) -> Phase;
    /// Execute the function f then:
    ///   - if execution of f does not panic change, call Self::set_phase(on_success)
    ///   - if execution of f panics: set the phase of the target object to on_panic and
    ///   release the lock.
    fn transition<R>(
        &mut self,
        f: impl FnOnce(&'a T) -> R,
        on_success: Phase,
        on_panic: Phase,
    ) -> R;
}

pub(crate) trait Mappable<T, V, U> {
    fn map<F: FnOnce(&T) -> &V>(self, f: F) -> U;
}

pub(crate) unsafe trait PhaseLocker<'a, T: 'a> {
    type ReadGuard: Phased;
    type WriteGuard: Phased + PhaseGuard<'a, T>;

    fn lock<FL: Fn(Phase) -> LockNature, FW: Fn(Phase) -> LockNature>(
        &'a self,
        value: &'a T,
        lock_nature: FL,
        on_wake_nature: FW,
        hint: Phase,
    ) -> LockResult<Self::ReadGuard, Self::WriteGuard>;
    fn lock_mut(&'a mut self, value: &'a T) -> Self::WriteGuard;
    fn try_lock<F: Fn(Phase) -> LockNature>(
        &'a self,
        value: &'a T,
        lock_nature: F,
        hint: Phase,
    ) -> Option<LockResult<Self::ReadGuard, Self::WriteGuard>>;
    fn phase(&self) -> Phase;
}

pub(crate) unsafe trait MutPhaseLocker {
    fn get_phase_unique(&mut self) -> Phase;
    fn set_phase(&mut self, p: Phase);

    fn transition<R>(&mut self, f: impl FnOnce() -> R, on_success: Phase, on_panic: Phase) -> R;
}

/// Nature of the lock requested
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum LockNature {
    Read,
    Write,
    None,
}
/// Result of a Phased locking
pub enum LockResult<R, W> {
    Read(R),
    Write(W),
    None(Phase),
}

#[cfg(all(feature = "lock_statistics", not(feature = "spin_loop")))]
pub use sync::LockStatistics;
