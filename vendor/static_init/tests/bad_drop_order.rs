// Copyright 2021 Olivier Kannengieser
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

#[cfg(debug_mode)]
mod test {

    struct A(bool);

    use static_init::{destructor, dynamic};

    #[dynamic(drop_only = 10)]
    static V0: A = A(false);

    #[dynamic(drop_only = 20)]
    static V1: A = A(true);

    impl Drop for A {
        fn drop(&mut self) {
            if self.0 {
                unsafe { &*V0 };
            }
        }
    }

    fn panic_hook(p: &core::panic::PanicInfo<'_>) -> () {
        println!("Panic caught {}", p);
        std::process::exit(0)
    }

    #[destructor(0)]
    extern "C" fn set_hook() {
        std::panic::set_hook(Box::new(panic_hook));
    }

    #[destructor(30)]
    extern "C" fn bad_exit() {
        println!("No panic happened :(");
        unsafe { libc::_exit(1) }
    }
}

#[test]
fn bad_drop_order() {}
