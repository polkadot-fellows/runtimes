use core::cell::UnsafeCell;
use std::sync::atomic::{compiler_fence, AtomicUsize, Ordering};
use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::Barrier;
use std::time::Duration;

use static_init::dynamic;

#[cfg(target_os = "linux")]
use core::mem::MaybeUninit;

#[cfg(target_os = "linux")]
use libc::{getrusage, rusage, RUSAGE_THREAD};

use crossbeam::thread;

use criterion::{black_box, measurement::WallTime, BenchmarkGroup, BenchmarkId};

use crate::tick_counter::TickCounter;

struct MutSynchronized<T>(UnsafeCell<T>);

unsafe impl<T> Sync for MutSynchronized<T> {}

pub struct Config<
    const MICRO_BENCH: bool,
    const NTHREAD: usize,
    const NT_SART: usize,
    const TOLERATE_CONTEXT_SWITCH: bool,
    const AVOID_SCHEDULER_PREMPTION: bool,
    //if AVOID_SCHEDULER_PREMPTION if activated all thread get a new fresh time slice
    //before each iteration. The result may be more artificial than in a
    //real scenario but this may be usefull to bench specifical
    //parts of the locking algorithm
>;

#[cfg(target_os = "linux")]
fn get_involontary_context_switch() -> i64 {
    unsafe {
        let mut usage = MaybeUninit::<rusage>::zeroed().assume_init();
        assert_eq!(getrusage(RUSAGE_THREAD, &mut usage), 0);
        usage.ru_nivcsw
    }
}
#[cfg(not(target_os = "linux"))]
fn get_involontary_context_switch() -> i64 {
    0
}

#[dynamic(0)]
static TK: TickCounter = TickCounter::new();

pub fn synchro_bench_input<
    I,
    T,
    R,
    const MICRO_BENCH: bool,
    const NT: usize,
    const NT_START: usize,
    const TOL_SWITCH: bool,
    const AVOID_SCHEDULER: bool,
>(
    c: &mut BenchmarkGroup<WallTime>,
    id: BenchmarkId,
    input: &I,
    build: impl Fn(&I) -> T,
    access: impl Fn(&T) -> R + Sync,
    _: Config<MICRO_BENCH, NT, NT_START, TOL_SWITCH, AVOID_SCHEDULER>,
) {
    let started: AtomicUsize = AtomicUsize::new(NT_START);

    let vm: MutSynchronized<T> = MutSynchronized(UnsafeCell::new(build(input)));

    let (sender, receiver) = sync_channel(0);

    let barrier = Barrier::new(NT + 1);

    assert!(NT_START <= NT);

    thread::scope(|s| {
        let test_init = {
            |sender: SyncSender<Option<Duration>>| loop {
                let mut expect = 0;
                let deb_prempted_count = if TOL_SWITCH {
                    0
                } else {
                    get_involontary_context_switch()
                };
                loop {
                    match started.compare_exchange_weak(
                        expect,
                        expect + 1,
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                    ) {
                        Err(x) => {
                            if x == NT_START + 1 {
                                break;
                            }
                            if x == NT + 2 {
                                return;
                            }
                            if x < NT_START {
                                expect = x;
                                continue;
                            }
                        }
                        Ok(_) => continue,
                    }
                }
                #[allow(clippy::branches_sharing_code)] //on purpose
                let duration = if MICRO_BENCH {
                    compiler_fence(Ordering::AcqRel);
                    let d = unsafe { TK.time(|| access(&*vm.0.get())) };
                    compiler_fence(Ordering::AcqRel);
                    d
                } else {
                    compiler_fence(Ordering::AcqRel);
                    let s = std::time::Instant::now();
                    compiler_fence(Ordering::AcqRel);
                    black_box(unsafe { access(&*vm.0.get()) });
                    compiler_fence(Ordering::AcqRel);
                    let d = Some(s.elapsed());
                    compiler_fence(Ordering::AcqRel);
                    d
                };
                let end_prempted_count = if TOL_SWITCH {
                    0
                } else {
                    get_involontary_context_switch()
                };
                if end_prempted_count == deb_prempted_count {
                    sender.send(duration).unwrap();
                } else {
                    sender.send(None).unwrap();
                }
                if AVOID_SCHEDULER {
                    barrier.wait();
                } else {
                    expect = 2 * NT + 10;
                    while let Err(x) = started.compare_exchange_weak(
                        expect,
                        expect + 1,
                        Ordering::Acquire,
                        Ordering::Relaxed,
                    ) {
                        if x >= 2 * NT + 10 {
                            expect = x;
                        }
                        for _ in 1..32 {
                            core::hint::spin_loop()
                        }
                    }
                }
            }
        };

        let mut spawned = vec![];

        c.bench_with_input(id, input, |b, input| {
            b.iter_custom(|iter| {
                if iter > 0 && spawned.is_empty() {
                    for _ in 0..NT {
                        let sender = sender.clone();
                        spawned.push(s.spawn(move |_| test_init(sender)));
                    }
                }
                let mut total = Duration::from_nanos(0);
                let mut index = 0;
                let mut iter_failure = 0;
                while index != iter {
                    //VMX.store(0, Ordering::Relaxed);
                    while started
                        .compare_exchange_weak(
                            NT_START,
                            NT_START + 1,
                            Ordering::Relaxed,
                            Ordering::Relaxed,
                        )
                        .is_err()
                    {
                        for _ in 1..8 {
                            core::hint::spin_loop()
                        }
                    }
                    let mut iter_total = Duration::from_secs(0);
                    let mut had_failure = false;
                    for _ in 0..NT {
                        iter_total += match receiver.recv_timeout(Duration::from_secs(10)) {
                            Err(_) => {
                                eprintln!("Timed out");
                                std::process::exit(1);
                            }
                            Ok(Some(v)) => v,
                            Ok(None) => {
                                had_failure = true;
                                Duration::from_secs(0)
                            }
                        }
                    }
                    if !had_failure {
                        index += 1;
                        total += iter_total;
                    } else {
                        iter_failure += 1;
                        if iter_failure > 4 * iter && (index == 0 || iter_failure / index > 10) {
                            eprintln!("To many iteration failure due to context switches");
                            std::process::exit(1);
                        }
                    }
                    unsafe { *vm.0.get() = build(input) };

                    if AVOID_SCHEDULER {
                        started
                            .compare_exchange(NT_START + 1, 0, Ordering::Release, Ordering::Relaxed)
                            .unwrap();
                        barrier.wait();
                    } else {
                        started
                            .compare_exchange(
                                NT_START + 1,
                                2 * NT + 10,
                                Ordering::Release,
                                Ordering::Relaxed,
                            )
                            .unwrap();
                        while started
                            .compare_exchange_weak(
                                3 * NT + 10,
                                0,
                                Ordering::Relaxed,
                                Ordering::Relaxed,
                            )
                            .is_err()
                        {
                            for _ in 1..32 {
                                core::hint::spin_loop()
                            }
                        }
                    }
                }
                total
            })
        });

        started
            .compare_exchange(NT_START, NT + 2, Ordering::AcqRel, Ordering::Relaxed)
            .unwrap();
        spawned.into_iter().for_each(|t| t.join().unwrap());
    })
    .unwrap();
}
