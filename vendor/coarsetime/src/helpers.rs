#[inline]
pub fn _sec_to_u64(sec: u64) -> u64 {
    sec << 32
}

#[inline]
pub fn _millis_to_u64(millis: u64) -> u64 {
    (millis * 1_099_511_628) >> 8
}

#[inline]
pub fn _nsecs_to_u64(nsecs: u64) -> u64 {
    let secs = nsecs / 1_000_000_000;
    _timespec_to_u64(secs, (nsecs - secs * 1_000_000_000) as u32)
}

#[inline]
pub fn _timespec_to_u64(tp_sec: u64, tp_nsec: u32) -> u64 {
    (tp_sec << 32) | ((tp_nsec as u64 * 9_223_372_037) >> 31)
}

#[inline]
pub fn _timeval_to_u64(tv_sec: u64, tv_usec: u32) -> u64 {
    (tv_sec << 32) | ((tv_usec as u64 * 9_223_372_036_855) >> 31)
}
