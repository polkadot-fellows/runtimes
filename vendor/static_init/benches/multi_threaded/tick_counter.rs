pub use inner::TickCounter;

#[cfg(target_arch = "x86_64")]
mod inner {

    // The instruction cpuid / rdtsc / rdtscp are used to benchmark
    // because the execution time of those instruction is very short
    // so that we get more chance to have thread trying to get the
    // lock in the same time.
    //
    // Cpuid is used to serialize instructions see:
    //https://www.intel.com/content/dam/www/public/us/en/documents/white-papers/ia-32-ia-64-benchmark-code-execution-paper.pdf
    use core::arch::x86_64::{__cpuid, __rdtscp, _rdtsc};
    use core::sync::atomic::{compiler_fence, Ordering};
    use criterion::black_box;
    use std::time::{Duration, Instant};

    #[derive(Copy, Clone)]
    pub struct TickCounter(u64, f64);

    impl TickCounter {
        pub fn new() -> TickCounter {
            #![allow(clippy::many_single_char_names)]
            let n = 10000;
            let mut arr = vec![];
            arr.reserve(n);

            for _ in 1..1000 {
                let s = Self::raw_start();
                let e = Self::raw_end();
                black_box(e - s);
            }

            std::thread::yield_now();

            for _ in 1..10 {
                let s = Self::raw_start();
                let e = Self::raw_end();
                black_box(e - s);
            }

            for _ in 0..n {
                let s = Self::raw_start();
                let e = Self::raw_end();
                arr.push(e - s);
            }

            arr.sort_unstable();
            for k in 0..n / 10 {
                arr[k] = arr[n / 10];
            }
            for k in n - n / 10..n {
                arr[k] = arr[n - n / 10 - 1];
            }
            let s = arr.iter().fold(0, |cur, v| cur + *v);
            let zero = s / 10000;

            // Now estimate the time/tick
            let n = 200;
            let mut arr = vec![];
            arr.reserve(n);

            //heat up
            for _ in 1..100 {
                Instant::now().elapsed();
            }

            std::thread::yield_now();

            for _ in 1..10 {
                Instant::now().elapsed();
            }

            let mut i = 0;
            while i < n {
                let e = Instant::now();
                let e0 = black_box(Self::raw_start());
                for _ in 0..i + 1 {
                    black_box(Self::raw_start());
                    black_box(Self::raw_end());
                }
                let e1 = black_box(Self::raw_start());
                let y = e.elapsed();
                if e1 < e0 {
                    continue;
                } else {
                    i += 1;
                }
                let dx = e1 - e0;
                let x = if dx > zero { dx - zero } else { 0 };
                arr.push((x as u32, y));
            }

            //Regularize
            let mut arr_1 = vec![];
            for v in arr.into_iter() {
                let v0 = v.0 as f64;
                arr_1.push((v0, v.1.as_nanos() as f64 / v0));
            }

            //Windsorize
            arr_1.sort_unstable_by(|a, b| PartialOrd::partial_cmp(&a.1, &b.1).unwrap());

            for k in 0..n / 10 {
                arr_1[k].1 = arr_1[n / 10].1;
            }
            for k in n - n / 10..n {
                arr_1[k].1 = arr_1[n - n / 10 - 1].1;
            }

            //the linear function that minimize quadratic error sum goes
            //through the middle point yeah!!
            let xm = arr_1.iter().fold(0f64, |v, x| v + x.0);
            let ym = arr_1.iter().fold(0f64, |v, x| v + (x.0 * x.1));

            let ns_per_tick = ym / xm;
            println!(
                "Estimated processor frequency: {}",
                (100f64 / ns_per_tick).round() / 100f64
            );
            TickCounter(zero, ns_per_tick)
        }
        #[inline(always)]
        pub fn time<R, F: FnOnce() -> R>(&self, f: F) -> Option<Duration> {
            let s = Self::raw_start();
            black_box(f());
            let e = Self::raw_end();
            if e < s {
                return None;
            }
            let v = (e - s) as f64;
            let v = (v - self.0 as f64) * self.1;
            let v = v.round();
            if v >= 0f64 {
                Some(Duration::from_nanos(v as u64))
            } else {
                Some(Duration::from_nanos(0))
            }
        }
        #[inline(always)]
        fn raw_start() -> u64 {
            compiler_fence(Ordering::AcqRel);
            let r = unsafe {
                //__cpuid(0);
                _rdtsc()
            };
            compiler_fence(Ordering::AcqRel);
            r
            //let cpuid_ask: u64 = 0;
            //let high: u64;
            //let low: u64;
            //unsafe {
            //    asm!(
            //         "cpuid",
            //         "rdtsc",
            //         out("rdx") high,
            //         inout("rax") cpuid_ask => low,
            //         out("rbx") _,
            //         out("rcx") _,
            //         options(nostack,preserves_flags)
            //    )
            //};
            //(high << 32) | low
        }
        #[inline(always)]
        fn raw_end() -> u64 {
            let mut v = 0;
            compiler_fence(Ordering::AcqRel);
            let c = unsafe {
                let c = __rdtscp(&mut v);
                __cpuid(0);
                c
            };
            compiler_fence(Ordering::AcqRel);
            c
            //let high: u64;
            //let low: u64;
            //unsafe {
            //    asm!(
            //         "rdtscp",
            //         "mov {high}, rdx",
            //         "mov {low}, rax",
            //         "mov rax, 0",
            //         "cpuid",
            //         high = out(reg) high,
            //         low = out(reg) low,
            //         out("rax")  _,
            //         out("rbx")  _,
            //         out("rcx")  _,
            //         out("rdx")  _,
            //         options(nostack,preserves_flags)
            //    )
            //};
            //(high << 32) | low
        }
    }
}

#[cfg(not(target_arch = "x86_64"))]
mod inner {
    use criterion::black_box;
    use std::time::{Duration, Instant};

    #[derive(Copy, Clone)]
    pub struct TickCounter(Duration);

    impl TickCounter {
        pub fn new() -> TickCounter {
            let mut arr = [Duration::from_secs(0); 10000];
            for _ in 1..1000 {
                let s = Self::raw_start();
                let e = Self::raw_end();
                black_box(e - s);
            }
            for v in arr.iter_mut() {
                let s = Self::raw_start();
                let e = Self::raw_end();
                *v = e - s;
            }
            arr.sort_unstable();
            for k in 0..1000 {
                arr[k] = arr[1000];
            }
            for k in 9000..10000 {
                arr[k] = arr[8999];
            }
            let s = arr.iter().fold(Duration::from_secs(0), |cur, v| cur + *v);
            let zero = s / 10000;
            TickCounter(zero)
        }
        #[inline(always)]
        pub fn time<R, F: FnOnce() -> R>(&self, f: F) -> Option<Duration> {
            let s = Self::raw_start();
            black_box(f());
            let e = Self::raw_end();
            if e < s {
                return None;
            }
            let v = e - s;
            if v >= self.0 {
                Some(v - self.0)
            } else {
                Some(Duration::from_nanos(0))
            }
        }
        #[inline(always)]
        fn raw_start() -> Instant {
            Instant::now()
        }
        #[inline(always)]
        fn raw_end() -> Instant {
            Instant::now()
        }
    }
}
