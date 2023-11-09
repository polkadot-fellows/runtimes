pub use static_impl::{ConstStatic, Static, __set_init_prio};

#[cfg(debug_mode)]
mod static_impl {

    use crate::{FinalyMode, InitMode, StaticInfo};

    use core::cmp::Ordering::*;
    use core::mem::MaybeUninit;
    use core::ops::{Deref, DerefMut};
    use core::sync::atomic::{AtomicI32, Ordering};

    /// The actual type of mutable *dynamic statics*.
    ///
    /// It implements `Deref<Target=T>` and `DerefMut`.
    ///
    /// All associated functions are only usefull for the implementation of
    /// the `dynamic` proc macro attribute
    pub struct Static<T>(MaybeUninit<T>, StaticInfo, AtomicI32);

    /// The actual type of non mutable *dynamic statics*.
    ///
    /// It implements `Deref<Target=T>`.
    ///
    /// All associated functions are only usefull for the implementation of
    /// the `dynamic` proc macro attribute
    pub struct ConstStatic<T>(Static<T>);

    static CUR_INIT_PRIO: AtomicI32 = AtomicI32::new(i32::MIN);

    static CUR_DROP_PRIO: AtomicI32 = AtomicI32::new(i32::MIN);

    #[doc(hidden)]
    #[inline]
    pub fn __set_init_prio(v: i32) {
        CUR_INIT_PRIO.store(v, Ordering::Relaxed);
    }

    impl<T> Static<T> {
        #[inline]
        /// Build an uninitialized Static
        ///
        /// # Safety
        ///
        /// The target object should be a mutable statics to ensure
        /// that all accesses to this object are unsafe.
        pub const unsafe fn uninit(info: StaticInfo) -> Self {
            Self(MaybeUninit::uninit(), info, AtomicI32::new(0))
        }
        #[inline]
        pub const fn from(v: T, info: StaticInfo) -> Self {
            Static(MaybeUninit::new(v), info, AtomicI32::new(1))
        }

        #[inline]
        pub fn set_to(this: &'static mut Self, v: T) {
            this.0 = MaybeUninit::new(v);
            this.2.store(1, Ordering::Relaxed);
        }

        #[inline]
        /// # Safety
        ///
        /// The objet should not be accessed after this call
        pub unsafe fn drop(this: &'static mut Self) {
            if let FinalyMode::ProgramDestructor(prio) = &this.1.drop_mode {
                CUR_DROP_PRIO.store(*prio as i32, Ordering::Relaxed);
                this.0.as_mut_ptr().drop_in_place();
                CUR_DROP_PRIO.store(i32::MIN, Ordering::Relaxed);
            } else {
                this.0.as_mut_ptr().drop_in_place();
            };
            this.2.store(2, Ordering::Relaxed);
        }
    }

    #[inline]
    fn check_access(info: &StaticInfo, status: i32) {
        if status == 0 {
            core::panic!(
                "Attempt to access variable {:#?} before it is initialized during initialization \
                 priority {}. Tip: increase init priority of this static to a value larger than \
                 {prio} (attribute syntax: `#[dynamic(init=<prio>)]`)",
                info,
                prio = CUR_INIT_PRIO.load(Ordering::Relaxed)
            )
        }
        if status == 2 {
            core::panic!(
                "Attempt to access variable {:#?} after it was destroyed during destruction \
                 priority {prio}. Tip increase drop priority of this static to a value larger \
                 than {prio} (attribute syntax: `#[dynamic(drop=<prio>)]`)",
                info,
                prio = CUR_DROP_PRIO.load(Ordering::Relaxed)
            )
        }
        let init_prio = CUR_INIT_PRIO.load(Ordering::Relaxed);
        let drop_prio = CUR_DROP_PRIO.load(Ordering::Relaxed);

        if let FinalyMode::ProgramDestructor(prio) = &info.drop_mode {
            match drop_prio.cmp(&(*prio as i32)) {
                Equal => core::panic!(
                    "This access to variable {:#?} is not sequenced before to its drop. Tip \
                     increase drop priority of this static to a value larger than {prio} \
                     (attribute syntax: `#[dynamic(drop=<prio>)]`)",
                    info,
                    prio = drop_prio
                ),
                Greater => core::panic!(
                    "Unexpected initialization order while accessing {:#?} from drop priority {}. \
                     This is a bug of `static_init` library, please report \"
             the issue inside `static_init` repository.",
                    info,
                    drop_prio
                ),
                Less => (),
            }
        }

        if let InitMode::ProgramConstructor(prio) = &info.init_mode {
            match init_prio.cmp(&(*prio as i32)) {
                Equal => core::panic!(
                    "This access to variable {:#?} is not sequenced after construction of this \
                     static. Tip increase init priority of this static to a value larger than \
                     {prio} (attribute syntax: `#[dynamic(init=<prio>)]`)",
                    info,
                    prio = init_prio
                ),
                Greater => core::panic!(
                    "Unexpected initialization order while accessing {:#?} from init priority {}. \
                     This is a bug of `static_init` library, please report \"
             the issue inside `static_init` repository.",
                    info,
                    init_prio,
                ),
                Less => (),
            }
        }
    }

    impl<T> Deref for Static<T> {
        type Target = T;
        #[inline(always)]
        fn deref(&self) -> &T {
            check_access(&self.1, self.2.load(Ordering::Relaxed));
            // SAFETY: The object is either
            //  - built with `uninit`, in which case it is a mutable static
            //    so all access path to it are unsafe
            //  - or built with `from` in which case it is necessarily initialized
            unsafe { &*self.0.as_ptr() }
        }
    }
    impl<T> DerefMut for Static<T> {
        #[inline(always)]
        fn deref_mut(&mut self) -> &mut T {
            check_access(&self.1, self.2.load(Ordering::Relaxed));
            // SAFETY: The object is either
            //  - built with `uninit`, in which case it is a mutable static
            //    so all access path to it are unsafe
            //  - or built with `from` in which case it is necessarily initialized
            unsafe { &mut *self.0.as_mut_ptr() }
        }
    }

    impl<T> ConstStatic<T> {
        #[inline]
        /// Build an uninitialized ConstStatic
        ///
        /// # Safety
        ///
        /// The target object should be a mutable static to
        /// ensure that all accesses to the object are unsafe.
        pub const unsafe fn uninit(info: StaticInfo) -> Self {
            Self(Static::uninit(info))
        }
        #[inline]
        pub const fn from(v: T, info: StaticInfo) -> Self {
            Self(Static::from(v, info))
        }
        #[inline]
        /// # Safety
        ///
        /// The reference to self should be unique.
        pub fn set_to(this: &'static mut Self, v: T) {
            Static::set_to(&mut this.0, v)
        }
        #[inline]
        /// # Safety
        ///
        /// The objet should not be accessed after this call
        pub unsafe fn drop(this: &'static mut Self) {
            Static::drop(&mut this.0);
        }
    }

    impl<T> Deref for ConstStatic<T> {
        type Target = T;
        #[inline(always)]
        fn deref(&self) -> &T {
            // SAFETY: The object is either
            //  - built with `uninit`, in which case it is a mutable static
            //    so all access path to it are unsafe
            //  - or built with `from` in which case it is necessarily initialized
            &*self.0
        }
    }
}

#[cfg(not(debug_mode))]
mod static_impl {
    use core::mem::MaybeUninit;
    use core::ops::{Deref, DerefMut};

    /// The actual type of mutable *dynamic statics*.
    ///
    /// It implements `Deref<Target=T>` and `DerefMut`.
    ///
    /// All associated functions are only usefull for the implementation of
    /// the `dynamic` proc macro attribute
    pub struct Static<T>(MaybeUninit<T>);

    /// The actual type of non mutable *dynamic statics*.
    ///
    /// It implements `Deref<Target=T>`.
    ///
    /// All associated functions are only usefull for the implementation of
    /// the `dynamic` proc macro attribute
    pub struct ConstStatic<T>(Static<T>);

    #[doc(hidden)]
    #[inline(always)]
    pub fn __set_init_prio(_: i32) {}

    //As a trait in order to avoid noise;
    impl<T> Static<T> {
        #[inline]
        /// Build a new static.
        ///
        /// # Safety
        ///
        /// The target object must be a mutable static
        pub const unsafe fn uninit() -> Self {
            Self(MaybeUninit::uninit())
        }
        #[inline]
        pub const fn from(v: T) -> Self {
            Self(MaybeUninit::new(v))
        }

        #[inline]
        pub fn set_to(this: &'static mut Self, v: T) {
            this.0 = MaybeUninit::new(v);
        }

        #[inline]
        /// Drop the inner object
        ///
        /// # Safety
        ///
        /// The object should have been previously initialized
        pub unsafe fn drop(this: &'static mut Self) {
            this.0.as_mut_ptr().drop_in_place();
        }
    }

    impl<T> Deref for Static<T> {
        type Target = T;
        #[inline(always)]
        fn deref(&self) -> &T {
            // SAFETY: The object is either
            //  - built with `uninit`, in which case it is a mutable static
            //    so all access path to it are unsafe
            //  - or built with `from` in which case it is necessarily initialized
            unsafe { &*self.0.as_ptr() }
        }
    }
    impl<T> DerefMut for Static<T> {
        #[inline(always)]
        fn deref_mut(&mut self) -> &mut T {
            // SAFETY: The object is either
            //  - built with `uninit`, in which case it is a mutable static
            //    so all access path to it are unsafe
            //  - or built with `from` in which case it is necessarily initialized
            unsafe { &mut *self.0.as_mut_ptr() }
        }
    }

    impl<T> ConstStatic<T> {
        #[inline]
        /// Build a new ConstStatic
        ///
        /// # Safety
        ///
        /// The target object must be a mutable static
        pub const unsafe fn uninit() -> Self {
            Self(Static::uninit())
        }
        #[inline]
        pub const fn from(v: T) -> Self {
            Self(Static::from(v))
        }
        #[inline]
        pub fn set_to(this: &'static mut Self, v: T) {
            Static::set_to(&mut this.0, v)
        }
        #[inline]
        /// # Safety
        ///
        /// The object should have been previously initialized
        pub unsafe fn drop(this: &'static mut Self) {
            Static::drop(&mut this.0);
        }
    }

    impl<T> Deref for ConstStatic<T> {
        type Target = T;
        #[inline(always)]
        fn deref(&self) -> &T {
            // SAFETY: The object is either
            //  - built with `uninit`, in which case it is a mutable static
            //    so all access path to it are unsafe
            //  - or built with `from` in which case it is necessarily initialized
            &*self.0
        }
    }
}
