use crate::phase_locker::{MutPhaseLocker, PhaseGuard};
use crate::phase_locker::{SyncPhaseLocker, UnSyncPhaseLocker};
use crate::{GeneratorTolerance, Phase, Sequential};
use core::marker::PhantomData;

pub(crate) type SyncSequentializer<G> = generic::LazySequentializer<SyncPhaseLocker, G>;

pub(crate) type UnSyncSequentializer<G> = generic::LazySequentializer<UnSyncPhaseLocker, G>;

#[inline]
#[cold]
fn lazy_initialization_only<'a, T: 'a, P: PhaseGuard<'a, T>>(
    mut phase_guard: P,
    init: impl FnOnce(&'a T),
) -> P {
    let cur = Phase::empty();

    let initialized = cur | Phase::INITIALIZED;

    let initialization_panic = cur | Phase::INITIALIZATION_PANICKED | Phase::INITIALIZATION_SKIPED;

    phase_guard.transition(init, initialized, initialization_panic);

    phase_guard
}

#[inline]
#[cold]
fn mut_lazy_initialization_only<P: MutPhaseLocker>(locker: &mut P, init: impl FnOnce()) {
    let cur = Phase::empty();

    let initialized = cur | Phase::INITIALIZED;

    let initialization_panic = cur | Phase::INITIALIZATION_PANICKED | Phase::INITIALIZATION_SKIPED;

    locker.transition(init, initialized, initialization_panic);
}

struct Unit<T>(PhantomData<T>);

impl<T> Unit<T> {
    const fn new() -> Self {
        Self(PhantomData)
    }
}

#[inline]
#[cold]
fn lazy_initialization<'a, P: PhaseGuard<'a, S>, S: Sequential + 'a, Tol: GeneratorTolerance>(
    mut phase_guard: P,
    init: impl FnOnce(&'a <S as Sequential>::Data),
    reg: impl FnOnce(&'a S) -> bool,
    _: Unit<Tol>,
) -> P
where
    <S as Sequential>::Data: 'a,
{
    let cur = phase_guard.phase();

    debug_assert!(!cur.intersects(Phase::FINALIZED | Phase::FINALIZATION_PANICKED));

    debug_assert!(!cur.intersects(Phase::INITIALIZED));

    let registration_finished;

    if !Tol::INIT_FAILURE || cur.is_empty() {
        debug_assert!(cur.is_empty());

        let registration_failed = Phase::REGISTRATION_PANICKED | Phase::INITIALIZATION_SKIPED;

        if phase_guard.transition(reg, cur, registration_failed) {
            registration_finished = Phase::REGISTERED;
        } else {
            registration_finished = Phase::REGISTRATION_REFUSED;
        }
    } else {
        registration_finished = cur;
    }

    if registration_finished.intersects(Phase::REGISTERED) {
        let before_init = if Tol::INIT_FAILURE {
            registration_finished
                & !(Phase::INITIALIZED
                    | Phase::INITIALIZATION_PANICKED
                    | Phase::INITIALIZATION_SKIPED)
        } else {
            registration_finished
        };

        let initialized = before_init | Phase::INITIALIZED;

        let initialization_panic =
            before_init | Phase::INITIALIZATION_PANICKED | Phase::INITIALIZATION_SKIPED;

        phase_guard.transition(
            |s| init(Sequential::data(s)),
            initialized,
            initialization_panic,
        )
    } else if Tol::FINAL_REGISTRATION_FAILURE {
        let before_init = if Tol::INIT_FAILURE {
            registration_finished
                & !(Phase::INITIALIZED
                    | Phase::INITIALIZATION_PANICKED
                    | Phase::INITIALIZATION_SKIPED)
        } else {
            registration_finished
        };

        let initialized = before_init | Phase::INITIALIZED;

        let initialization_panic =
            before_init | Phase::INITIALIZATION_PANICKED | Phase::INITIALIZATION_SKIPED;

        phase_guard.transition(
            |s| init(Sequential::data(s)),
            initialized,
            initialization_panic,
        )
    } else {
        let no_init = registration_finished | Phase::INITIALIZATION_SKIPED;

        phase_guard.set_phase(no_init);
    }
    phase_guard
}

fn lazy_finalization<'a, T: 'a, P: PhaseGuard<'a, T>>(mut phase_guard: P, f: impl FnOnce(&'a T)) {
    let cur = phase_guard.phase();

    let finalizing_success = cur | Phase::FINALIZED;

    let finalizing_failed = cur | Phase::FINALIZATION_PANICKED;

    phase_guard.transition(f, finalizing_success, finalizing_failed);
}

mod generic {
    use super::{
        lazy_finalization, lazy_initialization, lazy_initialization_only,
        mut_lazy_initialization_only, Unit,
    };
    use crate::phase_locker::{
        LockNature, LockResult, Mappable, MutPhaseLocker, PhaseGuard, PhaseLocker,
    };
    use crate::{
        FinalizableLazySequentializer, GeneratorTolerance,
        LazySequentializer as LazySequentializerTrait, Phase, Phased, Sequential, Sequentializer,
        UniqueLazySequentializer,
    };

    #[cfg(debug_mode)]
    use crate::CyclicPanic;
    use core::hint::unreachable_unchecked;
    use core::marker::PhantomData;
    #[cfg(debug_mode)]
    use core::sync::atomic::{AtomicUsize, Ordering};

    /// Ensure sequentialization.
    ///
    /// The SplitedSequentializer::init method can be called concurently on this
    /// object, only one thread will perform the initialization.
    ///
    /// More over the SplitedSequentializer::finalize method can be called by
    /// one thread while other threads call init. The finalize call will wait
    /// until the init function finished or skiped the initialization process.
    ///
    /// The finalization function will proceed only if the Sequential is in
    /// initialized phase. Concurent call to finalize may lead to concurent
    /// calls the finalize argument functor.
    ///
    /// # Initialization phases:
    ///
    /// The init function will firt check if `shall_proceed` functor is true.
    /// If it is the following phase transition of the object will happen
    ///
    ///  1. Initial state
    ///
    ///  2. registration
    ///
    ///  3. Either:   
    ///
    ///     a. registration_panicked and initialization_skiped (final)
    ///
    ///     b. registrated and initializing
    ///
    ///     c. registration_refused and initializing (if init_on_reg_failure is true)
    ///
    ///     d. registrated and initiazation_skiped (final if init_on_ref_failure is false)
    ///
    /// Then, if 3) is b:
    ///
    /// 4. Either:
    ///
    ///     - registrated and initialization_panicked
    ///
    ///     - registrated and initialized
    ///
    /// Or, if 3) is c):
    ///
    /// 4. Either:
    ///
    ///     - initialization_panicked
    ///
    ///     - initialized
    ///
    /// # Finalization phase:
    ///
    /// The finalization phase will be executed only if the previous phase is initialized
    ///
    /// The phase will conserve its qualificatif (registrated, initialized) and the following attriute
    /// transition will happend:
    ///
    /// 1. Finalization
    ///
    /// 2. Either:
    ///
    ///     a. Finalized
    ///
    ///     b. Finalization panicked
    ///
    pub struct LazySequentializer<Locker, G>(
        Locker,
        PhantomData<G>,
        #[cfg(debug_mode)] AtomicUsize,
    );

    impl<L, G> Phased for LazySequentializer<L, G>
    where
        L: Phased,
    {
        #[inline(always)]
        fn phase(this: &Self) -> Phase {
            Phased::phase(&this.0)
        }
    }
    impl<L, G> LazySequentializer<L, G> {
        #[inline(always)]
        pub const fn new(locker: L) -> Self {
            Self(
                locker,
                PhantomData,
                #[cfg(debug_mode)]
                AtomicUsize::new(0),
            )
        }
    }

    // SAFETY: it is safe because it does implement synchronized locks
    unsafe impl<'a, T: Sequential + 'a, L: 'a, G: 'a + GeneratorTolerance> Sequentializer<'a, T>
        for LazySequentializer<L, G>
    where
        T::Sequentializer: AsRef<LazySequentializer<L, G>>,
        T::Sequentializer: AsMut<LazySequentializer<L, G>>,
        L: PhaseLocker<'a, T::Data>,
        L: Phased,
    {
        type ReadGuard = L::ReadGuard;
        type WriteGuard = L::WriteGuard;

        #[inline(always)]
        fn lock(
            s: &'a T,
            lock_nature: impl Fn(Phase) -> LockNature,
            hint: Phase,
        ) -> LockResult<Self::ReadGuard, Self::WriteGuard> {
            let this = Sequential::sequentializer(s).as_ref();

            let data = Sequential::data(s);

            this.0.lock(data, &lock_nature, &lock_nature, hint)
        }

        #[inline(always)]
        fn try_lock(
            s: &'a T,
            lock_nature: impl Fn(Phase) -> LockNature,
            hint: Phase,
        ) -> Option<LockResult<Self::ReadGuard, Self::WriteGuard>> {
            let this = Sequential::sequentializer(s).as_ref();

            let data = Sequential::data(s);

            this.0.try_lock(data, &lock_nature, hint)
        }

        #[inline(always)]
        fn lock_mut(s: &'a mut T) -> Self::WriteGuard {
            let (that, data) = Sequential::sequentializer_data_mut(s);

            that.as_mut().0.lock_mut(data)
        }
    }

    #[inline(always)]
    fn whole_lock<'a, T: Sequential + 'a, L: 'a, G: 'a>(
        s: &'a T,
        lock_nature: impl Fn(Phase) -> LockNature,
        hint: Phase,
    ) -> LockResult<L::ReadGuard, L::WriteGuard>
    where
        T::Sequentializer: AsRef<LazySequentializer<L, G>>,
        T::Sequentializer: AsMut<LazySequentializer<L, G>>,
        L: PhaseLocker<'a, T>,
    {
        let this = Sequential::sequentializer(s).as_ref();

        this.0.lock(s, &lock_nature, &lock_nature, hint)
    }

    #[inline(always)]
    fn try_whole_lock<'a, T: Sequential + 'a, L: 'a, G: 'a>(
        s: &'a T,
        lock_nature: impl Fn(Phase) -> LockNature,
        hint: Phase,
    ) -> Option<LockResult<L::ReadGuard, L::WriteGuard>>
    where
        T::Sequentializer: AsRef<LazySequentializer<L, G>>,
        T::Sequentializer: AsMut<LazySequentializer<L, G>>,
        L: PhaseLocker<'a, T>,
    {
        let this = Sequential::sequentializer(s).as_ref();

        this.0.try_lock(s, &lock_nature, hint)
    }

    #[cfg(not(debug_mode))]
    fn debug_save_thread<T>(_: &T) {}

    #[cfg(debug_mode)]
    fn debug_save_thread<T: Sequential, L, G>(s: &T) -> DebugGuard<'_, T, L, G>
    where
        T::Sequentializer: AsRef<LazySequentializer<L, G>>,
    {
        let this = Sequential::sequentializer(s).as_ref();
        use parking_lot::lock_api::GetThreadId;
        this.2.store(
            parking_lot::RawThreadId.nonzero_thread_id().into(),
            Ordering::Relaxed,
        );
        DebugGuard(s, PhantomData)
    }

    #[must_use]
    #[cfg(debug_mode)]
    struct DebugGuard<'a, T: Sequential, L, G>(&'a T, PhantomData<(L, G)>)
    where
        T::Sequentializer: AsRef<LazySequentializer<L, G>>;

    #[cfg(debug_mode)]
    impl<'a, T: Sequential, L, G> Drop for DebugGuard<'a, T, L, G>
    where
        T::Sequentializer: AsRef<LazySequentializer<L, G>>,
    {
        fn drop(&mut self) {
            let this = Sequential::sequentializer(self.0).as_ref();
            this.2.store(0, Ordering::Relaxed);
        }
    }

    #[inline(always)]
    fn debug_test<T: Sequential, L, G>(_s: &T)
    where
        T::Sequentializer: AsRef<LazySequentializer<L, G>>,
    {
        #[cfg(debug_mode)]
        {
            let this = Sequential::sequentializer(_s).as_ref();
            let id = this.2.load(Ordering::Relaxed);
            if id != 0 {
                use parking_lot::lock_api::GetThreadId;
                if id == parking_lot::RawThreadId.nonzero_thread_id().into() {
                    std::panic::panic_any(CyclicPanic);
                }
            }
        }
    }
    // SAFETY: it is safe because it does implement synchronized locks
    unsafe impl<'a, T: Sequential + 'a, L: 'a, G: 'a> FinalizableLazySequentializer<'a, T>
        for LazySequentializer<L, G>
    where
        T::Sequentializer: AsRef<LazySequentializer<L, G>>,
        T::Sequentializer: AsMut<LazySequentializer<L, G>>,
        L: PhaseLocker<'a, T>,
        L: PhaseLocker<'a, T::Data>,
        L: Phased,
        <L as PhaseLocker<'a, T>>::ReadGuard:
            Mappable<T, T::Data, <L as PhaseLocker<'a, T::Data>>::ReadGuard>,
        <L as PhaseLocker<'a, T>>::WriteGuard:
            Mappable<T, T::Data, <L as PhaseLocker<'a, T::Data>>::WriteGuard>,
        <L as PhaseLocker<'a, T::Data>>::ReadGuard:
            From<<L as PhaseLocker<'a, T::Data>>::WriteGuard>,
        G: GeneratorTolerance,
    {
        #[inline(always)]
        fn init(
            s: &'a T,
            shall_init: impl Fn(Phase) -> bool,
            init: impl FnOnce(&'a <T as Sequential>::Data),
            reg: impl FnOnce(&'a T) -> bool,
        ) -> Phase {
            let this = Sequential::sequentializer(s).as_ref();

            let phase_guard = match this.0.lock(
                s,
                |p| {
                    if shall_init(p) {
                        debug_test(s);
                        LockNature::Write
                    } else {
                        LockNature::None
                    }
                },
                |_| LockNature::Read,
                Phase::INITIALIZED | Phase::REGISTERED,
            ) {
                LockResult::None(p) => return p,
                LockResult::Write(l) => l,
                LockResult::Read(l) => return Phased::phase(&l),
            };

            let _g = debug_save_thread(s);
            let ph = lazy_initialization(phase_guard, init, reg, Unit::<G>::new());
            ph.phase()
        }

        #[inline(always)]
        fn init_then_read_guard(
            s: &'a T,
            shall_init: impl Fn(Phase) -> bool,
            init: impl FnOnce(&'a <T as Sequential>::Data),
            reg: impl FnOnce(&'a T) -> bool,
        ) -> Self::ReadGuard {
            let this = Sequential::sequentializer(s).as_ref();

            match this.0.lock(
                s,
                |p| {
                    if shall_init(p) {
                        debug_test(s);
                        LockNature::Write
                    } else {
                        LockNature::Read
                    }
                },
                |_| LockNature::Read,
                Phase::INITIALIZED | Phase::REGISTERED,
            ) {
                LockResult::Read(l) => l.map(|s| Sequential::data(s)),
                LockResult::Write(l) => {
                    let _g = debug_save_thread(s);
                    let l = lazy_initialization(l, init, reg, Unit::<G>::new());
                    l.map(|s| Sequential::data(s)).into()
                }
                LockResult::None(_) => unsafe { unreachable_unchecked() },
            }
        }
        #[inline(always)]
        fn init_then_write_guard(
            s: &'a T,
            shall_init: impl Fn(Phase) -> bool,
            init: impl FnOnce(&'a <T as Sequential>::Data),
            reg: impl FnOnce(&'a T) -> bool,
        ) -> Self::WriteGuard {
            match whole_lock(s, |_| LockNature::Write, Phase::INITIALIZED) {
                LockResult::Write(l) => {
                    if shall_init(l.phase()) {
                        debug_test(s);
                        let _g = debug_save_thread(s);
                        let l = lazy_initialization(l, init, reg, Unit::<G>::new());
                        l.map(|s| Sequential::data(s))
                    } else {
                        l.map(|s| Sequential::data(s))
                    }
                }
                LockResult::Read(_) => unsafe { unreachable_unchecked() },
                LockResult::None(_) => unsafe { unreachable_unchecked() },
            }
        }

        #[inline(always)]
        fn try_init_then_read_guard(
            s: &'a T,
            shall_init: impl Fn(Phase) -> bool,
            init: impl FnOnce(&'a <T as Sequential>::Data),
            reg: impl FnOnce(&'a T) -> bool,
        ) -> Option<Self::ReadGuard> {
            let this = Sequential::sequentializer(s).as_ref();

            this.0
                .try_lock(
                    s,
                    |p| {
                        if shall_init(p) {
                            debug_test(s);
                            LockNature::Write
                        } else {
                            LockNature::Read
                        }
                    },
                    Phase::INITIALIZED | Phase::REGISTERED,
                )
                .map(|l| match l {
                    LockResult::Read(l) => l.map(|s| Sequential::data(s)),
                    LockResult::Write(l) => {
                        let _g = debug_save_thread(s);
                        let l = lazy_initialization(l, init, reg, Unit::<G>::new());
                        l.map(|s| Sequential::data(s)).into()
                    }
                    LockResult::None(_) => unsafe { unreachable_unchecked() },
                })
        }
        #[inline(always)]
        fn try_init_then_write_guard(
            s: &'a T,
            shall_init: impl Fn(Phase) -> bool,
            init: impl FnOnce(&'a <T as Sequential>::Data),
            reg: impl FnOnce(&'a T) -> bool,
        ) -> Option<Self::WriteGuard> {
            try_whole_lock(
                s,
                |_| LockNature::Write,
                Phase::INITIALIZED | Phase::REGISTERED,
            )
            .map(|l| match l {
                LockResult::Write(l) => {
                    if shall_init(l.phase()) {
                        debug_test(s);
                        let _g = debug_save_thread(s);
                        let l = lazy_initialization(l, init, reg, Unit::<G>::new());
                        l.map(|s| Sequential::data(s))
                    } else {
                        l.map(|s| Sequential::data(s))
                    }
                }
                LockResult::Read(_) => unsafe { unreachable_unchecked() },
                LockResult::None(_) => unsafe { unreachable_unchecked() },
            })
        }
        #[inline(always)]

        fn finalize_callback(s: &'a T, f: impl FnOnce(&'a T::Data)) {
            let this = Sequential::sequentializer(s).as_ref();

            let how = |p: Phase| {
                if p.intersects(Phase::INITIALIZED) {
                    LockNature::Write
                } else {
                    LockNature::None
                }
            };

            let phase_guard = match this.0.lock(
                Sequential::data(s),
                how,
                how,
                Phase::INITIALIZED | Phase::REGISTERED,
            ) {
                LockResult::None(_) => return,
                LockResult::Write(l) => l,
                LockResult::Read(_) => unsafe { unreachable_unchecked() },
            };

            debug_assert!((phase_guard.phase()
                & (Phase::FINALIZED | Phase::FINALIZATION_PANICKED))
                .is_empty());

            lazy_finalization(phase_guard, f);
        }
    }

    impl<L, G> AsRef<LazySequentializer<L, G>> for LazySequentializer<L, G> {
        #[inline(always)]
        fn as_ref(&self) -> &Self {
            self
        }
    }
    impl<L, G> AsMut<LazySequentializer<L, G>> for LazySequentializer<L, G> {
        #[inline(always)]
        fn as_mut(&mut self) -> &mut Self {
            self
        }
    }

    // SAFETY: it is safe because it does implement synchronized locks
    unsafe impl<'a, T: Sequential + 'a, L: 'a, G: 'a> LazySequentializerTrait<'a, T>
        for LazySequentializer<L, G>
    where
        T::Sequentializer: AsRef<LazySequentializer<L, G>>,
        T::Sequentializer: AsMut<LazySequentializer<L, G>>,
        L: PhaseLocker<'a, T>,
        L: PhaseLocker<'a, T::Data>,
        L: Phased,
        <L as PhaseLocker<'a, T>>::ReadGuard:
            Mappable<T, T::Data, <L as PhaseLocker<'a, T::Data>>::ReadGuard>,
        <L as PhaseLocker<'a, T>>::WriteGuard:
            Mappable<T, T::Data, <L as PhaseLocker<'a, T::Data>>::WriteGuard>,
        <L as PhaseLocker<'a, T::Data>>::ReadGuard:
            From<<L as PhaseLocker<'a, T::Data>>::WriteGuard>,
        G: GeneratorTolerance,
    {
        const INITIALIZED_HINT: Phase = Phase::INITIALIZED;
        #[inline(always)]
        fn init(
            s: &'a T,
            shall_init: impl Fn(Phase) -> bool,
            init: impl FnOnce(&'a <T as Sequential>::Data),
        ) -> Phase {
            let this = Sequential::sequentializer(s).as_ref();

            let phase_guard = match this.0.lock(
                Sequential::data(s),
                |p| {
                    if shall_init(p) {
                        debug_test(s);
                        LockNature::Write
                    } else {
                        LockNature::None
                    }
                },
                |_| LockNature::Read,
                Phase::INITIALIZED,
            ) {
                LockResult::None(p) => return p,
                LockResult::Write(l) => l,
                LockResult::Read(l) => return Phased::phase(&l),
            };

            let _g = debug_save_thread(s);
            let ph = lazy_initialization_only(phase_guard, init);
            ph.phase()
        }

        #[inline(always)]
        fn init_then_read_guard(
            s: &'a T,
            shall_init: impl Fn(Phase) -> bool,
            init: impl FnOnce(&'a <T as Sequential>::Data),
        ) -> Self::ReadGuard {
            let this = Sequential::sequentializer(s).as_ref();

            match this.0.lock(
                Sequential::data(s),
                |p| {
                    if shall_init(p) {
                        debug_test(s);
                        LockNature::Write
                    } else {
                        LockNature::Read
                    }
                },
                |_| LockNature::Read,
                Phase::INITIALIZED,
            ) {
                LockResult::Read(l) => l,
                LockResult::Write(l) => {
                    let _g = debug_save_thread(s);
                    let l = lazy_initialization_only(l, init);
                    l.into()
                }
                LockResult::None(_) => unsafe { unreachable_unchecked() },
            }
        }
        #[inline(always)]
        fn init_then_write_guard(
            s: &'a T,
            shall_init: impl Fn(Phase) -> bool,
            init: impl FnOnce(&'a <T as Sequential>::Data),
        ) -> Self::WriteGuard {
            match <Self as Sequentializer<'a, T>>::lock(
                s,
                |_| LockNature::Write,
                Phase::INITIALIZED,
            ) {
                LockResult::Write(l) => {
                    if shall_init(l.phase()) {
                        debug_test(s);
                        let _g = debug_save_thread(s);
                        lazy_initialization_only(l, init)
                    } else {
                        l
                    }
                }
                LockResult::Read(_) => unsafe { unreachable_unchecked() },
                LockResult::None(_) => unsafe { unreachable_unchecked() },
            }
        }

        #[inline(always)]
        fn try_init_then_read_guard(
            s: &'a T,
            shall_init: impl Fn(Phase) -> bool,
            init: impl FnOnce(&'a <T as Sequential>::Data),
        ) -> Option<Self::ReadGuard> {
            let this = Sequential::sequentializer(s).as_ref();

            this.0
                .try_lock(
                    Sequential::data(s),
                    |p| {
                        if shall_init(p) {
                            debug_test(s);
                            LockNature::Write
                        } else {
                            LockNature::Read
                        }
                    },
                    Phase::INITIALIZED,
                )
                .map(|l| match l {
                    LockResult::Read(l) => l,
                    LockResult::Write(l) => {
                        let _g = debug_save_thread(s);
                        let l = lazy_initialization_only(l, init);
                        l.into()
                    }
                    LockResult::None(_) => unsafe { unreachable_unchecked() },
                })
        }
        #[inline(always)]
        fn try_init_then_write_guard(
            s: &'a T,
            shall_init: impl Fn(Phase) -> bool,
            init: impl FnOnce(&'a <T as Sequential>::Data),
        ) -> Option<Self::WriteGuard> {
            <Self as Sequentializer<'a, T>>::try_lock(s, |_| LockNature::Write, Phase::INITIALIZED)
                .map(|l| match l {
                    LockResult::Write(l) => {
                        if shall_init(l.phase()) {
                            debug_test(s);
                            let _g = debug_save_thread(s);
                            lazy_initialization_only(l, init)
                        } else {
                            l
                        }
                    }
                    LockResult::Read(_) => unsafe { unreachable_unchecked() },
                    LockResult::None(_) => unsafe { unreachable_unchecked() },
                })
        }
    }

    impl<L, T: Sequential<Sequentializer = Self>, G: GeneratorTolerance> UniqueLazySequentializer<T>
        for LazySequentializer<L, G>
    where
        L: MutPhaseLocker,
    {
        fn init_unique(
            target: &mut T,
            shall_init: impl Fn(Phase) -> bool,
            init: impl FnOnce(&mut <T as Sequential>::Data),
        ) -> Phase {
            let (that, data) = Sequential::sequentializer_data_mut(target);

            //let phase_guard = that.0.lock_mut(data);

            if shall_init(that.0.get_phase_unique()) {
                mut_lazy_initialization_only(&mut that.0, || init(data));
            }
            that.0.get_phase_unique()
        }
    }
}
