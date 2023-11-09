use super::spin_wait::SpinWait;
use super::{LockNature, LockResult, Mappable, MutPhaseLocker, PhaseGuard, PhaseLocker};
use crate::phase::*;
use crate::{Phase, Phased};
use core::cell::UnsafeCell;
use core::mem::forget;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{fence, AtomicU32, Ordering};

/// A synchronised phase locker.
pub(crate) struct SyncPhaseLocker(AtomicU32);

pub(crate) struct Lock<'a> {
    futex: &'a AtomicU32,
    init_phase: Phase,
    on_unlock: Phase,
}

/// A phase guard that allow atomic phase transition that
/// can be turned fastly into a [SyncReadPhaseGuard].
pub(crate) struct SyncPhaseGuard<'a, T: ?Sized>(&'a T, Lock<'a>);

pub(crate) struct ReadLock<'a> {
    futex: &'a AtomicU32,
    init_phase: Phase,
}

/// A kind of read lock.
pub(crate) struct SyncReadPhaseGuard<'a, T: ?Sized>(&'a T, ReadLock<'a>);

pub(crate) struct Mutex<T>(UnsafeCell<T>, SyncPhaseLocker);

pub(crate) struct MutexGuard<'a, T>(&'a mut T, Lock<'a>);

// SyncPhaseGuard
//-------------------
//
impl<'a, T> Deref for SyncPhaseGuard<'a, T> {
    type Target = T;
    #[inline(always)]
    fn deref(&self) -> &T {
        self.0
    }
}

impl<'a, T: ?Sized> SyncPhaseGuard<'a, T> {
    #[inline(always)]
    fn new(r: &'a T, lock: Lock<'a>) -> Self {
        Self(r, lock)
    }

    #[inline(always)]
    pub fn map<S: ?Sized>(self, f: impl FnOnce(&'a T) -> &'a S) -> SyncPhaseGuard<'a, S> {
        SyncPhaseGuard(f(self.0), self.1)
    }
}
impl<'a, T: 'a, U: 'a> Mappable<T, U, SyncPhaseGuard<'a, U>> for SyncPhaseGuard<'a, T> {
    #[inline(always)]
    fn map<F: FnOnce(&'a T) -> &'a U>(self, f: F) -> SyncPhaseGuard<'a, U> {
        Self::map(self, f)
    }
}
unsafe impl<'a, T: ?Sized> PhaseGuard<'a, T> for SyncPhaseGuard<'a, T> {
    #[inline(always)]
    fn set_phase(&mut self, p: Phase) {
        self.1.on_unlock = p;
    }
    #[inline(always)]
    fn commit_phase(&mut self) {
        //Butter fly trick
        let cur = self.1.phase();
        let to_xor = self.1.on_unlock ^ cur;
        self.1.xor_phase(to_xor);
    }
    #[inline(always)]
    fn phase(&self) -> Phase {
        self.1.on_unlock
    }
    #[inline(always)]
    fn transition<R>(
        &mut self,
        f: impl FnOnce(&'a T) -> R,
        on_success: Phase,
        on_panic: Phase,
    ) -> R {
        self.1.on_unlock = on_panic;
        let res = f(self.0);
        self.1.on_unlock = on_success;
        res
    }
}

impl<'a, T> Phased for SyncPhaseGuard<'a, T> {
    #[inline(always)]
    fn phase(this: &Self) -> Phase {
        this.1.on_unlock
    }
}

// SyncReadPhaseGuard
//-------------------
//
impl<'a, T> Deref for SyncReadPhaseGuard<'a, T> {
    type Target = T;
    #[inline(always)]
    fn deref(&self) -> &T {
        self.0
    }
}

impl<'a, T: ?Sized> SyncReadPhaseGuard<'a, T> {
    #[inline(always)]
    fn new(r: &'a T, lock: ReadLock<'a>) -> Self {
        Self(r, lock)
    }

    #[inline(always)]
    pub fn map<S: ?Sized>(self, f: impl FnOnce(&'a T) -> &'a S) -> SyncReadPhaseGuard<'a, S> {
        SyncReadPhaseGuard(f(self.0), self.1)
    }
}
impl<'a, T: 'a, U: 'a> Mappable<T, U, SyncReadPhaseGuard<'a, U>> for SyncReadPhaseGuard<'a, T> {
    #[inline(always)]
    fn map<F: FnOnce(&'a T) -> &'a U>(self, f: F) -> SyncReadPhaseGuard<'a, U> {
        Self::map(self, f)
    }
}
impl<'a, T> From<SyncPhaseGuard<'a, T>> for SyncReadPhaseGuard<'a, T> {
    #[inline(always)]
    fn from(this: SyncPhaseGuard<'a, T>) -> SyncReadPhaseGuard<'a, T> {
        SyncReadPhaseGuard(this.0, this.1.into())
    }
}

impl<'a, T> Phased for SyncReadPhaseGuard<'a, T> {
    #[inline(always)]
    fn phase(this: &Self) -> Phase {
        this.1.init_phase
    }
}

impl<'a, T> Clone for SyncReadPhaseGuard<'a, T> {
    #[inline(always)]
    fn clone(&self) -> Self {
        Self(self.0, self.1.clone())
    }
}

// Mutex
//-------------------
//
unsafe impl<T: Send> Sync for Mutex<T> {}

unsafe impl<T: Send> Send for Mutex<T> {}

impl<T> Mutex<T> {
    #[inline(always)]
    pub(crate) const fn new(value: T) -> Self {
        Self(UnsafeCell::new(value), SyncPhaseLocker::new(Phase::empty()))
    }
    #[inline(always)]
    pub(crate) fn lock(&self) -> MutexGuard<'_, T> {
        let lk = if let LockResult::Write(l) = {
            self.1.raw_lock(
                |_p| LockNature::Write,
                |_p| LockNature::Write,
                Phase::empty(),
            )
        } {
            l
        } else {
            unreachable!()
        };
        MutexGuard(unsafe { &mut *self.0.get() }, lk)
    }
}

impl<'a, T> Deref for MutexGuard<'a, T> {
    type Target = T;
    #[inline(always)]
    fn deref(&self) -> &T {
        self.0
    }
}
impl<'a, T> DerefMut for MutexGuard<'a, T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut T {
        self.0
    }
}

// Lock
// ----

// STATES:
// LOCKED_BIT | <READ_WAITER_BIT|WRITE_WAITER_BIT> => Write lock held
// any READER_BIT | <READ_WAITER_BIT|WRITE_WAITER_BIT> => Read lock held
// LOCKED_BIT|any READER_BIT | <READ_WAITER_BIT|WRITE_WAITER_BIT>
//       => wlock or rlock is being transfered to rlock
//       => rlock are taken right now
// any READ_WAITER_BIT,WRITE_WAITER_BIT => a lock is being released
// during transfer to a write lock, the WRITE_WAITER_BIT is 0
// but if the transfer succeed, it means that there where one or
// more waiter for the write lock and WRITE_WAITER_BIT must be reset to 1
// if a waiter is awaken.

impl<'a> Lock<'a> {
    #[inline(always)]
    fn new(futex: &'a AtomicU32, current: u32) -> Self {
        let p = Phase::from_bits_truncate(current);
        Self {
            futex,
            init_phase: p,
            on_unlock: p,
        }
    }
    #[inline(always)]
    pub fn phase(&self) -> Phase {
        let v = self.futex.load(Ordering::Relaxed);
        Phase::from_bits_truncate(v)
    }
    #[inline(always)]
    pub fn xor_phase(&self, xor: Phase) -> Phase {
        let v = self.futex.fetch_xor(xor.bits(), Ordering::Release);
        Phase::from_bits_truncate(v) ^ xor
    }
}

impl<'a> Lock<'a> {
    #[inline(always)]
    fn into_read_lock(self, cur: Phase) -> ReadLock<'a> {
        //state: old_phase | LOCKED_BIT | <0:READ_WAITER_BIT|0:WRITE_WAITER_BIT>
        let xor = (cur ^ self.on_unlock).bits() | LOCKED_BIT | READER_UNITY;
        //state: phase | READER_UNITY | <0:READ_WAITER_BIT|0:WRITE_WAITER_BIT>
        self.futex.fetch_xor(xor, Ordering::Release);

        let r = ReadLock::new(self.futex, self.on_unlock.bits());

        forget(self);

        r
    }
}

impl<'a> Drop for Lock<'a> {
    #[inline(always)]
    fn drop(&mut self) {
        //state: old_phase | LOCKED_BIT
        let p = self.init_phase.bits();

        match self.futex.compare_exchange(
            p | LOCKED_BIT,
            self.on_unlock.bits(),
            Ordering::Release,
            Ordering::Relaxed,
        ) {
            Ok(_) => (),
            Err(_) => unreachable!(),
        };
    }
}

impl<'a> From<Lock<'a>> for ReadLock<'a> {
    #[inline(always)]
    fn from(this: Lock<'a>) -> ReadLock<'a> {
        let p = this.init_phase;
        this.into_read_lock(p)
    }
}

// ReadLock
// --------
impl<'a> ReadLock<'a> {
    #[inline(always)]
    fn new(futex: &'a AtomicU32, current: u32) -> Self {
        let p = Phase::from_bits_truncate(current);
        Self {
            futex,
            init_phase: p,
        }
    }
}
impl<'a> Clone for ReadLock<'a> {
    fn clone(&self) -> Self {
        let mut spin_wait = SpinWait::new();
        let mut cur = self.futex.load(Ordering::Relaxed);
        loop {
            if !has_readers_max(cur)
                && self
                    .futex
                    .compare_exchange_weak(
                        cur,
                        cur + READER_UNITY,
                        Ordering::Acquire,
                        Ordering::Relaxed,
                    )
                    .is_ok()
            {
                return ReadLock {
                    futex: &self.futex,
                    init_phase: self.init_phase,
                };
            }
            spin_wait.spin_no_yield();
            cur = self.futex.load(Ordering::Relaxed);
        }
    }
}

impl<'a> Drop for ReadLock<'a> {
    #[inline(always)]
    fn drop(&mut self) {
        self.futex.fetch_sub(READER_UNITY, Ordering::Release);
    }
}

#[inline(always)]
fn has_no_readers(v: u32) -> bool {
    v & (READER_OVERF | READER_BITS) == 0
}

#[inline(always)]
fn has_readers_max(v: u32) -> bool {
    //can actualy happen in two condition:
    //  - READER_BITS
    //  - READER_BITS | READER_OVERF
    v & (READER_OVERF | READER_BITS) == (READER_OVERF | READER_BITS)
}

#[inline(always)]
fn is_not_write_locked(v: u32) -> bool {
    v & LOCKED_BIT == 0
}

#[inline(always)]
fn is_write_lockable(v: u32) -> bool {
    is_not_write_locked(v) && has_no_readers(v)
}
#[inline(always)]
fn is_read_lockable(v: u32) -> bool {
    is_not_write_locked(v) && !has_readers_max(v)
}

// SyncPhaseLocker
// ---------------
//
struct MutGuard<'a>(&'a mut AtomicU32, Phase);
impl<'a> Drop for MutGuard<'a> {
    fn drop(&mut self) {
        *self.0.get_mut() = self.1.bits();
    }
}

// SyncPhaseLocker
// ---------------
//
unsafe impl MutPhaseLocker for SyncPhaseLocker {
    #[inline(always)]
    fn get_phase_unique(&mut self) -> Phase {
        Phase::from_bits(*self.0.get_mut()).unwrap()
    }

    #[inline(always)]
    fn set_phase(&mut self, p: Phase) {
        *self.0.get_mut() = p.bits();
    }

    #[inline(always)]
    fn transition<R>(&mut self, f: impl FnOnce() -> R, on_success: Phase, on_panic: Phase) -> R {
        let m = MutGuard(&mut self.0, on_panic);
        let r = f();
        forget(m);
        Self::set_phase(self, on_success);
        r
    }
}
unsafe impl<'a, T: 'a> PhaseLocker<'a, T> for SyncPhaseLocker {
    type ReadGuard = SyncReadPhaseGuard<'a, T>;
    type WriteGuard = SyncPhaseGuard<'a, T>;

    #[inline(always)]
    fn lock<FL: Fn(Phase) -> LockNature, FW: Fn(Phase) -> LockNature>(
        &'a self,
        value: &'a T,
        lock_nature: FL,
        on_wake_nature: FW,
        hint: Phase,
    ) -> LockResult<Self::ReadGuard, Self::WriteGuard> {
        Self::lock(self, value, lock_nature, on_wake_nature, hint)
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
        hint: Phase,
    ) -> Option<LockResult<Self::ReadGuard, Self::WriteGuard>> {
        Self::try_lock(self, value, lock_nature, hint)
    }
    #[inline(always)]
    fn phase(&self) -> Phase {
        Self::phase(self)
    }
}
impl Phased for SyncPhaseLocker {
    #[inline(always)]
    fn phase(this: &Self) -> Phase {
        this.phase()
    }
}

impl SyncPhaseLocker {
    #[inline(always)]
    pub const fn new(p: Phase) -> Self {
        SyncPhaseLocker(AtomicU32::new(p.bits()))
    }
    #[inline(always)]
    /// Return the current phase and synchronize with the end of the
    /// phase transition that leads to this phase.
    pub fn phase(&self) -> Phase {
        Phase::from_bits_truncate(self.0.load(Ordering::Acquire))
    }
    #[inline(always)]
    /// Returns a mutable phase locker
    pub fn lock_mut<'a, T: ?Sized>(&'a mut self, v: &'a T) -> SyncPhaseGuard<'_, T> {
        let cur = self.0.fetch_or(LOCKED_BIT, Ordering::Acquire);
        SyncPhaseGuard::new(v, Lock::new(&self.0, cur))
    }
    #[inline(always)]
    /// lock the phase.
    ///
    /// If the returned value is a LockResult::Read, then other threads
    /// may also hold a such a lock. This lock call synchronize with the
    /// phase transition that leads to the current phase and the phase will
    /// not change while this lock is held
    ///
    /// If the returned value is a LockResult::Write, then only this thread
    /// hold the lock and the phase can be atomically transitionned using the
    /// returned lock.
    ///
    /// If the returned value is LockResult::None, then the call to lock synchronize
    /// whit the end of the phase transition that led to the current phase.
    pub fn lock<'a, T: ?Sized>(
        &'a self,
        v: &'a T,
        how: impl Fn(Phase) -> LockNature,
        on_waiting_how: impl Fn(Phase) -> LockNature,
        hint: Phase,
    ) -> LockResult<SyncReadPhaseGuard<'_, T>, SyncPhaseGuard<'_, T>> {
        match self.raw_lock(how, on_waiting_how, hint) {
            LockResult::Write(l) => LockResult::Write(SyncPhaseGuard::new(v, l)),
            LockResult::Read(l) => LockResult::Read(SyncReadPhaseGuard::new(v, l)),
            LockResult::None(p) => LockResult::None(p),
        }
    }
    #[inline(always)]
    /// try to lock the phase.
    ///
    /// If the returned value is a Some(LockResult::Read), then other threads
    /// may also hold a such a lock. This lock call synchronize with the
    /// phase transition that leads to the current phase and the phase will
    /// not change while this lock is held
    ///
    /// If the returned value is a Some(LockResult::Write), then only this thread
    /// hold the lock and the phase can be atomically transitionned using the
    /// returned lock.
    ///
    /// If the returned value is Some(LockResult::None), then the call to lock synchronize
    /// whit the end of the phase transition that led to the current phase.
    ///
    /// If the returned value is None, the the lock is held by other threads and could
    /// not be obtain.
    pub fn try_lock<'a, T: ?Sized>(
        &'a self,
        v: &'a T,
        how: impl Fn(Phase) -> LockNature,
        hint: Phase,
    ) -> Option<LockResult<SyncReadPhaseGuard<'_, T>, SyncPhaseGuard<'_, T>>> {
        self.try_raw_lock(how, hint).map(|l| match l {
            LockResult::Write(l) => LockResult::Write(SyncPhaseGuard::new(v, l)),
            LockResult::Read(l) => LockResult::Read(SyncReadPhaseGuard::new(v, l)),
            LockResult::None(p) => LockResult::None(p),
        })
    }
    #[inline(always)]
    fn try_raw_lock(
        &self,
        how: impl Fn(Phase) -> LockNature,
        hint: Phase,
    ) -> Option<LockResult<ReadLock<'_>, Lock<'_>>> {
        let mut cur = hint.bits();
        match how(hint) {
            LockNature::None => {
                cur = self.0.load(Ordering::Acquire);
                let p = Phase::from_bits_truncate(cur);
                if let LockNature::None = how(p) {
                    return Some(LockResult::None(p));
                }
            }
            LockNature::Write => {
                match self.0.compare_exchange_weak(
                    cur,
                    cur | LOCKED_BIT,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => return Some(LockResult::Write(Lock::new(&self.0, cur))),
                    Err(x) => {
                        cur = x;
                    }
                }
            }
            LockNature::Read => {
                match self.0.compare_exchange_weak(
                    cur,
                    cur + READER_UNITY,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => {
                        return Some(LockResult::Read(ReadLock::new(&self.0, cur)));
                    }
                    Err(x) => {
                        cur = x;
                    }
                }
            }
        }

        let p = Phase::from_bits_truncate(cur);

        match how(p) {
            LockNature::Write => {
                if is_write_lockable(cur)
                    && self
                        .0
                        .compare_exchange(
                            cur,
                            cur | LOCKED_BIT,
                            Ordering::Acquire,
                            Ordering::Relaxed,
                        )
                        .is_ok()
                {
                    return Some(LockResult::Write(Lock::new(&self.0, cur)));
                }
            }
            LockNature::Read => loop {
                if !is_read_lockable(cur) {
                    break;
                }
                match self.0.compare_exchange_weak(
                    cur,
                    cur + READER_UNITY,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => return Some(LockResult::Read(ReadLock::new(&self.0, cur))),
                    Err(x) => {
                        cur = x;
                        if !(how(Phase::from_bits_truncate(cur)) == LockNature::Read) {
                            break;
                        }
                    }
                }
            },
            LockNature::None => {
                fence(Ordering::Acquire);
                return Some(LockResult::None(p));
            }
        }

        None
    }

    #[inline(always)]
    fn raw_lock(
        &self,
        how: impl Fn(Phase) -> LockNature,
        _on_waiting_how: impl Fn(Phase) -> LockNature,
        hint: Phase,
    ) -> LockResult<ReadLock<'_>, Lock<'_>> {
        let mut cur = hint.bits();
        match how(hint) {
            LockNature::None => {
                cur = self.0.load(Ordering::Acquire);
                let p = Phase::from_bits_truncate(cur);
                if let LockNature::None = how(p) {
                    return LockResult::None(p);
                }
            }
            LockNature::Write => {
                match self.0.compare_exchange_weak(
                    cur,
                    cur | LOCKED_BIT,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => return LockResult::Write(Lock::new(&self.0, cur)),
                    Err(x) => {
                        cur = x;
                    }
                }
            }
            LockNature::Read => {
                match self.0.compare_exchange_weak(
                    cur,
                    cur + READER_UNITY,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => {
                        return LockResult::Read(ReadLock::new(&self.0, cur));
                    }
                    Err(x) => {
                        cur = x;
                    }
                }
            }
        }
        self.raw_lock_slow(how, cur)
    }

    #[cold]
    fn raw_lock_slow(
        &self,
        how: impl Fn(Phase) -> LockNature,
        mut cur: u32,
    ) -> LockResult<ReadLock<'_>, Lock<'_>> {
        let mut spin_wait = SpinWait::new();
        loop {
            let p = Phase::from_bits_truncate(cur);
            match how(p) {
                LockNature::Write => {
                    if is_write_lockable(cur) {
                        match self.0.compare_exchange_weak(
                            cur,
                            cur | LOCKED_BIT,
                            Ordering::Acquire,
                            Ordering::Relaxed,
                        ) {
                            Ok(_) => return LockResult::Write(Lock::new(&self.0, cur)),
                            Err(x) => {
                                cur = x;
                                continue;
                            }
                        }
                    }
                }
                LockNature::Read => {
                    let mut spin_wait = SpinWait::new();
                    loop {
                        if !is_read_lockable(cur) {
                            break;
                        }
                        match self.0.compare_exchange_weak(
                            cur,
                            cur + READER_UNITY,
                            Ordering::Acquire,
                            Ordering::Relaxed,
                        ) {
                            Ok(_) => {
                                return LockResult::Read(ReadLock::new(&self.0, cur));
                            }
                            Err(_) => {
                                if !spin_wait.spin_no_yield() {
                                    break;
                                }
                                cur = self.0.load(Ordering::Relaxed);
                                if !(how(Phase::from_bits_truncate(cur)) == LockNature::Read) {
                                    break;
                                }
                            }
                        }
                    }
                }
                LockNature::None => {
                    fence(Ordering::Acquire);
                    return LockResult::None(p);
                }
            }
            spin_wait.spin_no_yield();
            cur = self.0.load(Ordering::Relaxed);
        }
    }
}
