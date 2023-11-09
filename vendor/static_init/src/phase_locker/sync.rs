use super::futex::Futex;
use super::spin_wait::SpinWait;
use super::{LockNature, LockResult, Mappable, MutPhaseLocker, PhaseGuard, PhaseLocker};
use crate::phase::*;
use crate::{Phase, Phased};
use core::cell::UnsafeCell;
use core::mem::forget;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{fence, Ordering};

#[cfg(feature = "lock_statistics")]
use core::sync::atomic::AtomicUsize;

#[cfg(feature = "lock_statistics")]
use core::fmt::{self, Display, Formatter};

/// A synchronised phase locker.
pub(crate) struct SyncPhaseLocker(Futex);

pub(crate) struct Lock<'a> {
    futex: &'a Futex,
    init_phase: Phase,
    on_unlock: Phase,
}

/// A phase guard that allow atomic phase transition that
/// can be turned fastly into a [SyncReadPhaseGuard].
pub(crate) struct SyncPhaseGuard<'a, T: ?Sized>(&'a T, Lock<'a>);

pub(crate) struct ReadLock<'a> {
    futex: &'a Futex,
    init_phase: Phase,
}

/// A kind of read lock.
pub(crate) struct SyncReadPhaseGuard<'a, T: ?Sized>(&'a T, ReadLock<'a>);

pub(crate) struct Mutex<T>(UnsafeCell<T>, SyncPhaseLocker);

pub(crate) struct MutexGuard<'a, T>(&'a mut T, Lock<'a>);

#[cfg(feature = "lock_statistics")]
static OPTIMISTIC_FAILURES: AtomicUsize = AtomicUsize::new(0);

#[cfg(feature = "lock_statistics")]
static SECOND_ATTEMPT_FAILURES: AtomicUsize = AtomicUsize::new(0);

#[cfg(feature = "lock_statistics")]
static WRITE_LOCK_WHILE_READER_FAILURES: AtomicUsize = AtomicUsize::new(0);

#[cfg(feature = "lock_statistics")]
static WRITE_WAIT_FAILURES: AtomicUsize = AtomicUsize::new(0);

#[cfg(feature = "lock_statistics")]
static WRITE_WAIT_SUCCESSES: AtomicUsize = AtomicUsize::new(0);

#[cfg(feature = "lock_statistics")]
static READ_WAIT_FAILURES: AtomicUsize = AtomicUsize::new(0);

#[cfg(feature = "lock_statistics")]
static READ_WAIT_SUCCESSES: AtomicUsize = AtomicUsize::new(0);

#[cfg(feature = "lock_statistics")]
static ADDAPTATIVE_WAIT_SUCCESSES: AtomicUsize = AtomicUsize::new(0);

#[cfg(feature = "lock_statistics")]
static LATE_ADDAPTATIONS: AtomicUsize = AtomicUsize::new(0);

#[cfg(feature = "lock_statistics")]
#[derive(Debug)]
pub struct LockStatistics {
    pub optimistic_failures: usize,
    pub second_attempt_failures: usize,
    pub write_lock_while_reader_failures: usize,
    pub write_wait_failures: usize,
    pub write_wait_successes: usize,
    pub read_wait_failures: usize,
    pub read_wait_successes: usize,
    pub addaptative_wait_successes: usize,
    pub late_addaptations: usize,
}

#[cfg(feature = "lock_statistics")]
impl LockStatistics {
    pub fn get_and_reset() -> Self {
        Self {
            optimistic_failures: OPTIMISTIC_FAILURES.swap(0, Ordering::Relaxed),
            second_attempt_failures: SECOND_ATTEMPT_FAILURES.swap(0, Ordering::Relaxed),

            write_lock_while_reader_failures: WRITE_LOCK_WHILE_READER_FAILURES
                .swap(0, Ordering::Relaxed),

            write_wait_failures: WRITE_WAIT_FAILURES.swap(0, Ordering::Relaxed),

            write_wait_successes: WRITE_WAIT_SUCCESSES.swap(0, Ordering::Relaxed),

            read_wait_failures: READ_WAIT_FAILURES.swap(0, Ordering::Relaxed),

            read_wait_successes: READ_WAIT_SUCCESSES.swap(0, Ordering::Relaxed),

            addaptative_wait_successes: ADDAPTATIVE_WAIT_SUCCESSES.swap(0, Ordering::Relaxed),

            late_addaptations: LATE_ADDAPTATIONS.swap(0, Ordering::Relaxed),
        }
    }
}
#[cfg(feature = "lock_statistics")]
impl Display for LockStatistics {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:#?}", self)
    }
}

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
    fn clone(&self) -> Self {
        SyncReadPhaseGuard(self.0, self.1.clone())
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
    fn new(futex: &'a Futex, current: u32) -> Self {
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
        let prev = self.futex.fetch_xor(xor, Ordering::Release);

        let r = if prev & READ_WAITER_BIT != 0 {
            wake_readers(&self.futex, 0, true)
        } else {
            ReadLock::new(self.futex, self.on_unlock.bits())
        };

        forget(self);

        r
    }
}

#[cold]
#[inline]
fn transfer_lock(futex: &Futex, mut cur: u32) {
    // try to reaquire the lock
    //state: phase | 0:READ_WAITER_BIT<|>0:WRITE_WAITER_BIT
    assert_eq!(cur & (LOCKED_BIT | READER_BITS | READER_OVERF), 0);
    assert_ne!(cur & (READ_WAITER_BIT | WRITE_WAITER_BIT), 0);
    if futex.prefer_wake_one_writer() {
        loop {
            let mut un_activate_lock = 0;
            if cur & WRITE_WAITER_BIT != 0 {
                //state: phase | <READ_WAITER_BIT> | WRITE_WAITER_BIT
                let prev = futex.fetch_xor(WRITE_WAITER_BIT | LOCKED_BIT, Ordering::Relaxed);
                assert_ne!(prev & WRITE_WAITER_BIT, 0);
                assert_eq!(prev & (LOCKED_BIT | READER_BITS | READER_OVERF), 0);
                if futex.wake_one_writer() {
                    return;
                };
                cur ^= WRITE_WAITER_BIT | LOCKED_BIT;
                // turn the write lock into a read lock if
                // there are reader waiting
                un_activate_lock = LOCKED_BIT;
                //phase: phase | LOCKED_BIT | <READ_WAITER_BIT>
                //
                //so here we own a write lock
            }

            if cur & READ_WAITER_BIT != 0 {
                //phase: phase | <LOCKED_BIT> | READ_WAITER_BIT
                wake_readers(futex, un_activate_lock, false);
                //drop the acquired read lock
                return;
            }

            //cur: phase | LOCKED_BIT
            cur = futex.fetch_and(!LOCKED_BIT, Ordering::Relaxed);
            assert_ne!(cur & LOCKED_BIT, 0);
            if has_no_waiters(cur) {
                break;
            } //else new threads are waiting
            cur &= !LOCKED_BIT; //unused
            core::hint::spin_loop();
        }
    } else {
        loop {
            if cur & READ_WAITER_BIT != 0 {
                //phase: phase | <WRITE_WAITER_BIT> | READ_WAITER_BIT
                wake_readers(futex, 0, false);
                return;
            }

            assert_ne!(cur & WRITE_WAITER_BIT, 0);

            //state: phase | <READ_WAITER_BIT> | WRITE_WAITER_BIT
            let prev = futex.fetch_xor(WRITE_WAITER_BIT | LOCKED_BIT, Ordering::Relaxed);
            assert_ne!(prev & WRITE_WAITER_BIT, 0);
            assert_eq!(prev & (LOCKED_BIT | READER_BITS | READER_OVERF), 0);
            if futex.wake_one_writer() {
                return;
            };
            //phase: phase | LOCKED_BIT | <READ_WAITER_BIT>

            //cur: phase | LOCKED_BIT
            cur = futex.fetch_and(!LOCKED_BIT, Ordering::Relaxed);

            assert_ne!(cur & LOCKED_BIT, 0);

            if has_no_waiters(cur) {
                break;
            } //else new threads are waiting
            cur &= !LOCKED_BIT; //unused
            core::hint::spin_loop();
        }
    }
}

impl<'a> Drop for Lock<'a> {
    #[inline(always)]
    fn drop(&mut self) {
        //state: old_phase | LOCKED_BIT | <0:READ_WAITER_BIT|0:WRITE_WAITER_BIT>
        let p = self.init_phase.bits();

        match self.futex.compare_exchange(
            p | LOCKED_BIT,
            self.on_unlock.bits(),
            Ordering::Release,
            Ordering::Relaxed,
        ) {
            Ok(_) => return,
            Err(x) => x,
        };

        //while let Err(x) = self.futex.compare_exchange_weak(
        //    cur, cur & (READ_WAITER_BIT|WRITE_WAITER_BIT|READER_BITS|READER_OVERF) | p,Ordering::Release,Ordering::Relaxed) {
        //    cur = x;
        //}
        let xor = (self.init_phase ^ self.on_unlock).bits() | LOCKED_BIT;
        let prev = self.futex.fetch_xor(xor, Ordering::Release);
        //state: phase | <1:READ_WAITER_BIT|1:WRITE_WAITER_BIT>
        if has_waiters(prev) {
            //let cur = cur & (READ_WAITER_BIT|WRITE_WAITER_BIT|READER_BITS|READER_OVERF) | p;
            //state: phase | 1:READ_WAITER_BIT<|>1:WRITE_WAITER_BIT
            transfer_lock(&self.futex, prev ^ xor);
        }
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
    fn new(futex: &'a Futex, current: u32) -> Self {
        let p = Phase::from_bits_truncate(current);
        Self {
            futex,
            init_phase: p,
        }
    }

    //#[inline(always)]
    //pub fn fast_clone(&self) -> Option<Self> {
    //    let mut cur = self.futex.load(Ordering::Relaxed);

    //    if has_readers_max(cur) {
    //        return None;
    //    }

    //    match self.futex.compare_exchange_weak(cur, cur + READER_UNITY,Ordering::Acquire, Ordering::Relaxed) {
    //        Ok(_) => return Some(ReadLock{futex:&self.futex,init_phase: self.init_phase}),
    //        Err(c) => cur = c,
    //    }

    //    if has_readers_max(cur) {
    //        return None;
    //    }

    //    match self.futex.compare_exchange(cur, cur + READER_UNITY,Ordering::Acquire, Ordering::Relaxed) {
    //        Ok(_) => Some(ReadLock{futex:&self.futex,init_phase: self.init_phase}),
    //        Err(_) => None,
    //    }

    //}
}

impl<'a> Drop for ReadLock<'a> {
    #[inline(always)]
    fn drop(&mut self) {
        //state: phase | <LOCKED_BIT> | READER_UNITY*n | <0:READ_WAITER_BIT> |<0:WRITE_WAITER_BIT>
        let prev = self.futex.fetch_sub(READER_UNITY, Ordering::Release);
        //state: phase | <LOCKED_BIT> | READER_UNITY*(n-1) | <1:READ_WAITER_BIT> |<1:WRITE_WAITER_BIT>
        if has_one_reader(prev) && is_not_write_locked(prev) && has_waiters(prev) {
            //state: phase | READ_WAITER_BIT <|> WRITE_WAITER_BIT
            let cur = prev - READER_UNITY;
            transfer_lock(&self.futex, cur);
        }
    }
}

impl<'a> Clone for ReadLock<'a> {
    fn clone(&self) -> Self {
        let mut spin_wait = SpinWait::new();
        let mut cur = self.futex.load(Ordering::Relaxed);
        loop {
            if !has_readers_max(cur) {
                cur = match read_lock(&self.futex, |cur| !has_readers_max(cur), cur) {
                    Ok(rl) => return rl,
                    Err(cur) => cur,
                }
            }

            if cur & READ_WAITER_BIT == 0 && spin_wait.spin() {
                cur = self.futex.load(Ordering::Relaxed);
                continue;
            }

            if cur & READ_WAITER_BIT == 0 {
                match self.futex.compare_exchange_weak(
                    cur,
                    cur | READ_WAITER_BIT,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Err(x) => {
                        cur = x;
                        continue;
                    }
                    Ok(_) => cur |= READ_WAITER_BIT,
                }
            }

            if self.futex.compare_and_wait_as_reader(cur) {
                let cur = self.futex.load(Ordering::Relaxed);

                assert_ne!(cur & (READER_BITS | READER_OVERF), 0);

                return ReadLock::new(&self.futex, cur);
            }

            spin_wait.reset();
            cur = self.futex.load(Ordering::Relaxed);
        }
    }
}

#[inline(always)]
fn has_no_readers(v: u32) -> bool {
    v & (READER_OVERF | READER_BITS) == 0
}

#[inline(always)]
fn has_readers(v: u32) -> bool {
    v & (READER_OVERF | READER_BITS) != 0
}

#[inline(always)]
fn has_one_reader(v: u32) -> bool {
    v & (READER_OVERF | READER_BITS) == READER_UNITY
}

#[inline(always)]
fn has_readers_max(v: u32) -> bool {
    //can actualy happen in two condition:
    //  - READER_BITS
    //  - READER_BITS | READER_OVERF
    v & READER_BITS == READER_BITS
}

#[inline(always)]
fn is_not_write_locked(v: u32) -> bool {
    v & LOCKED_BIT == 0
}
//#[inline(always)]
//fn is_write_locked(v:u32) -> bool {
//    v & LOCKED_BIT != 0
//}
#[inline(always)]
fn has_waiters(v: u32) -> bool {
    v & (READ_WAITER_BIT | WRITE_WAITER_BIT) != 0
}
#[inline(always)]
fn has_no_waiters(v: u32) -> bool {
    v & (READ_WAITER_BIT | WRITE_WAITER_BIT) == 0
}

#[inline(always)]
fn is_write_lockable(v: u32) -> bool {
    is_not_write_locked(v) && (has_readers(v) || has_no_waiters(v))
}
#[inline(always)]
fn is_read_lockable(v: u32) -> bool {
    (has_readers(v) || (has_no_waiters(v) && is_not_write_locked(v))) && !has_readers_max(v)
}

#[inline(always)]
fn wake_readers(futex: &Futex, to_unactivate: u32, converting: bool) -> ReadLock {
    // at least one reader must have been marked + READER_OVERF
    let rb = if converting { 0 } else { READER_UNITY };
    let v = futex.fetch_xor(
        READ_WAITER_BIT | to_unactivate | READER_OVERF | rb,
        Ordering::Relaxed,
    );
    assert_eq!(v & to_unactivate, to_unactivate);
    if !converting {
        //otherwise threads may be already taking read lock
        assert_ne!(v & READER_UNITY, rb); //BUG: fired
    }
    assert_eq!((v ^ to_unactivate) & LOCKED_BIT, 0);

    let c = futex.wake_readers();

    let cur = futex.fetch_sub(READER_OVERF - READER_UNITY * (c as u32), Ordering::Relaxed);
    ReadLock::new(futex, cur)
}

struct MutGuard<'a>(&'a mut Futex, Phase);
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
        SyncPhaseLocker(Futex::new(p.bits()))
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
        let mut cur = match self.optimistic_lock(&how, hint) {
            Ok(x) => return Some(x),
            Err(cur) => cur,
        };

        #[cfg(feature = "lock_statistics")]
        {
            OPTIMISTIC_FAILURES.fetch_add(1, Ordering::Relaxed);
        }

        let p = Phase::from_bits_truncate(cur);

        match how(p) {
            LockNature::Write => {
                if is_write_lockable(cur)
                    && has_no_readers(cur)
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

        #[cfg(feature = "lock_statistics")]
        {
            SECOND_ATTEMPT_FAILURES.fetch_add(1, Ordering::Relaxed);
        }

        None
    }

    #[inline(always)]
    fn raw_lock(
        &self,
        how: impl Fn(Phase) -> LockNature,
        on_waiting_how: impl Fn(Phase) -> LockNature,
        hint: Phase,
    ) -> LockResult<ReadLock<'_>, Lock<'_>> {
        let cur = match self.optimistic_lock(&how, hint) {
            Ok(x) => return x,
            Err(cur) => cur,
        };

        #[cfg(feature = "lock_statistics")]
        {
            OPTIMISTIC_FAILURES.fetch_add(1, Ordering::Relaxed);
        }

        let p = Phase::from_bits_truncate(cur);

        match how(p) {
            LockNature::Write => {
                if is_write_lockable(cur)
                    && has_no_readers(cur)
                    && self
                        .0
                        .compare_exchange_weak(
                            cur,
                            cur | LOCKED_BIT,
                            Ordering::Acquire,
                            Ordering::Relaxed,
                        )
                        .is_ok()
                {
                    return LockResult::Write(Lock::new(&self.0, cur));
                }
            }
            LockNature::Read => {
                if is_read_lockable(cur) {
                    if let Ok(r) = read_lock(
                        &self.0,
                        |cur| {
                            how(Phase::from_bits_truncate(cur)) == LockNature::Read
                                && is_read_lockable(cur)
                        },
                        cur,
                    ) {
                        return LockResult::Read(r);
                    }
                }
            }
            LockNature::None => {
                fence(Ordering::Acquire);
                return LockResult::None(p);
            }
        }
        #[cfg(feature = "lock_statistics")]
        {
            SECOND_ATTEMPT_FAILURES.fetch_add(1, Ordering::Relaxed);
        }

        self.raw_lock_slow(how, on_waiting_how)
    }
    #[cold]
    fn raw_lock_slow(
        &self,
        how: impl Fn(Phase) -> LockNature,
        on_waiting_how: impl Fn(Phase) -> LockNature,
    ) -> LockResult<ReadLock<'_>, Lock<'_>> {
        let mut spin_wait = SpinWait::new();

        let mut cur = self.0.load(Ordering::Relaxed);

        loop {
            match how(Phase::from_bits_truncate(cur)) {
                LockNature::None => {
                    fence(Ordering::Acquire);
                    return LockResult::None(Phase::from_bits_truncate(cur));
                }
                LockNature::Write => {
                    if is_write_lockable(cur) {
                        if has_no_readers(cur) {
                            match self.0.compare_exchange_weak(
                                cur,
                                cur | LOCKED_BIT,
                                Ordering::Acquire,
                                Ordering::Relaxed,
                            ) {
                                Ok(_) => {
                                    return LockResult::Write(Lock::new(&self.0, cur));
                                }
                                Err(x) => {
                                    cur = x;
                                    continue;
                                }
                            }
                        } else {
                            //lock while readers
                            match self.0.compare_exchange_weak(
                                cur,
                                cur | LOCKED_BIT,
                                Ordering::Acquire,
                                Ordering::Relaxed,
                            ) {
                                Ok(x) => cur = x | LOCKED_BIT,
                                Err(x) => {
                                    cur = x;
                                    continue;
                                }
                            }

                            cur = match wait_for_readers(&self.0, cur) {
                                Ok(l) => return LockResult::Write(l),
                                Err(cur) => cur,
                            };
                            #[cfg(feature = "lock_statistics")]
                            {
                                WRITE_LOCK_WHILE_READER_FAILURES.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }
                    if cur & WRITE_WAITER_BIT == 0 && spin_wait.spin() {
                        cur = self.0.load(Ordering::Relaxed);
                        continue;
                    }
                }
                LockNature::Read => {
                    if is_read_lockable(cur) {
                        cur = match read_lock(
                            &self.0,
                            |cur| {
                                how(Phase::from_bits_truncate(cur)) == LockNature::Read
                                    && is_read_lockable(cur)
                            },
                            cur,
                        ) {
                            Ok(r) => return LockResult::Read(r),
                            Err(cur) => cur,
                        };
                    }

                    if has_no_waiters(cur) && spin_wait.spin() {
                        cur = self.0.load(Ordering::Relaxed);
                        continue;
                    }
                }
            }

            match on_waiting_how(Phase::from_bits_truncate(cur)) {
                LockNature::None => {
                    fence(Ordering::Acquire);
                    return LockResult::None(Phase::from_bits_truncate(cur));
                }

                LockNature::Write => {
                    if cur & WRITE_WAITER_BIT == 0 {
                        match self.0.compare_exchange_weak(
                            cur,
                            cur | WRITE_WAITER_BIT,
                            Ordering::Relaxed,
                            Ordering::Relaxed,
                        ) {
                            Err(x) => {
                                cur = x;
                                continue;
                            }
                            Ok(_) => cur |= WRITE_WAITER_BIT,
                        }
                    }

                    if let Some(lock) = wait_as_writer_then_wake_with_lock(&self.0, cur, &how) {
                        #[cfg(feature = "lock_statistics")]
                        {
                            WRITE_WAIT_SUCCESSES.fetch_add(1, Ordering::Relaxed);
                            if how(Phase::from_bits_truncate(cur)) != LockNature::Write {
                                ADDAPTATIVE_WAIT_SUCCESSES.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                        return lock;
                    } else {
                        #[cfg(feature = "lock_statistics")]
                        {
                            WRITE_WAIT_FAILURES.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
                LockNature::Read => {
                    if cur & READ_WAITER_BIT == 0 {
                        match self.0.compare_exchange_weak(
                            cur,
                            cur | READ_WAITER_BIT,
                            Ordering::Relaxed,
                            Ordering::Relaxed,
                        ) {
                            Err(x) => {
                                cur = x;
                                continue;
                            }
                            Ok(_) => cur |= READ_WAITER_BIT,
                        }
                    }

                    if let Some(lock) = wait_as_reader_then_wake_with_lock(&self.0, cur, &how) {
                        #[cfg(feature = "lock_statistics")]
                        {
                            READ_WAIT_SUCCESSES.fetch_add(1, Ordering::Relaxed);
                            if how(Phase::from_bits_truncate(cur)) != LockNature::Read {
                                ADDAPTATIVE_WAIT_SUCCESSES.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                        return lock;
                    } else {
                        #[cfg(feature = "lock_statistics")]
                        {
                            READ_WAIT_FAILURES.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
            }
            spin_wait.reset();
            cur = self.0.load(Ordering::Relaxed);
        }
    }

    #[inline(always)]
    fn optimistic_lock(
        &self,
        how: impl Fn(Phase) -> LockNature,
        hint: Phase,
    ) -> Result<LockResult<ReadLock<'_>, Lock<'_>>, u32> {
        let mut cur = hint.bits();
        match how(hint) {
            LockNature::None => {
                cur = self.0.load(Ordering::Acquire);
                let p = Phase::from_bits_truncate(cur);
                if let LockNature::None = how(p) {
                    return Ok(LockResult::None(p));
                }
            }
            LockNature::Write => {
                match self.0.compare_exchange_weak(
                    cur,
                    cur | LOCKED_BIT,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => return Ok(LockResult::Write(Lock::new(&self.0, cur))),
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
                        return Ok(LockResult::Read(ReadLock::new(&self.0, cur)));
                    }
                    Err(x) => {
                        cur = x;
                    }
                }
            }
        }
        Err(cur)
    }
}

#[inline(always)]
fn read_lock(
    futex: &Futex,
    shall_continue: impl Fn(u32) -> bool,
    mut cur: u32,
) -> Result<ReadLock<'_>, u32> {
    let mut inner_spin_wait = SpinWait::new();

    loop {
        match futex.compare_exchange_weak(
            cur,
            cur + READER_UNITY,
            Ordering::Acquire,
            Ordering::Relaxed,
        ) {
            Ok(_) => {
                return Ok(ReadLock::new(&futex, cur));
            }
            Err(_) => {
                inner_spin_wait.spin_no_yield();
                cur = futex.load(Ordering::Relaxed);
                if !shall_continue(cur) {
                    break;
                }
            }
        }
    }
    Err(cur)
}

#[cold]
fn wait_as_writer_then_wake_with_lock(
    futex: &Futex,
    cur: u32,
    how: impl Fn(Phase) -> LockNature,
) -> Option<LockResult<ReadLock<'_>, Lock<'_>>> {
    debug_assert_ne!(cur & WRITE_WAITER_BIT, 0);

    if futex.compare_and_wait_as_writer(cur) {
        let cur = futex.load(Ordering::Relaxed);

        assert_ne!(cur & LOCKED_BIT, 0);

        let lock = Lock::new(&futex, cur);

        match how(Phase::from_bits_truncate(cur)) {
            LockNature::Write => return Some(LockResult::Write(lock)),

            LockNature::Read => {
                #[cfg(feature = "lock_statistics")]
                {
                    LATE_ADDAPTATIONS.fetch_add(1, Ordering::Relaxed);
                }
                return Some(LockResult::Read(
                    lock.into_read_lock(Phase::from_bits_truncate(cur)),
                ));
            }
            LockNature::None => {
                #[cfg(feature = "lock_statistics")]
                {
                    LATE_ADDAPTATIONS.fetch_add(1, Ordering::Relaxed);
                }
                return Some(LockResult::None(Phase::from_bits_truncate(cur)));
            }
        }
    }
    None
}

#[cold]
fn wait_as_reader_then_wake_with_lock(
    futex: &Futex,
    cur: u32,
    how: impl Fn(Phase) -> LockNature,
) -> Option<LockResult<ReadLock<'_>, Lock<'_>>> {
    debug_assert_ne!(cur & READ_WAITER_BIT, 0);

    if futex.compare_and_wait_as_reader(cur) {
        let cur = futex.load(Ordering::Relaxed);

        assert_ne!(cur & (READER_BITS | READER_OVERF), 0);

        let lock = ReadLock::new(&futex, cur);

        match how(Phase::from_bits_truncate(cur)) {
            LockNature::Read => return Some(LockResult::Read(lock)),
            LockNature::None => {
                #[cfg(feature = "lock_statistics")]
                {
                    LATE_ADDAPTATIONS.fetch_add(1, Ordering::Relaxed);
                }
                return Some(LockResult::None(Phase::from_bits_truncate(cur)));
            }
            LockNature::Write => (),
        }
    }
    None
}

#[inline(always)]
fn wait_for_readers(futex: &Futex, mut cur: u32) -> Result<Lock<'_>, u32> {
    // wait for reader releasing the lock
    let mut spinwait = SpinWait::new();
    while spinwait.spin() {
        cur = futex.load(Ordering::Acquire);
        if has_no_readers(cur) {
            return Ok(Lock::new(&futex, cur));
        }
    }

    loop {
        match futex.compare_exchange_weak(
            cur,
            (cur | WRITE_WAITER_BIT) & !LOCKED_BIT,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Err(x) => {
                cur = x;
                if has_no_readers(cur) {
                    fence(Ordering::Acquire);
                    return Ok(Lock::new(&futex, cur));
                }
            }
            Ok(_) => {
                cur = (cur | WRITE_WAITER_BIT) & !LOCKED_BIT;
                break;
            }
        }
    }
    Err(cur)
}
