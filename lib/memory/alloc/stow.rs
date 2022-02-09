use core::mem;
use core::ptr;
use tg_core::f16;
use tg_core::murmur3::Murmur3;
use crate::alloc::{Hold, HoldError};

/// Conversion from a value to `Self`, allocating in a `Hold` as needed.
pub trait StowFrom<'b, T>: Sized {
    /// Converts a `value` into `Self`, allocating in `hold` as needed.
    fn try_stow_from(value: T, hold: &dyn Hold<'b>) -> Result<Self, (T, HoldError)>;

    #[inline]
    fn stow_from(value: T, hold: &dyn Hold<'b>) -> Self {
        match StowFrom::try_stow_from(value, hold) {
            Ok(value) => value,
            Err((_, error)) => panic!("{:?}", error),
        }
    }
}

/// Conversion from `Self` into a value, allocating in a `Hold` as needed.
pub trait StowInto<'b, T>: Sized {
    /// Converts `Self` into a value, allocating in `hold` as needed.
    fn try_stow_into(self, hold: &dyn Hold<'b>) -> Result<T, (Self, HoldError)>;

    #[inline]
    fn stow_into(self, hold: &dyn Hold<'b>) -> T {
        match self.try_stow_into(hold) {
            Ok(value) => value,
            Err((_, error)) => panic!("{:?}", error),
        }
    }
}

/// A type that can be recursively moved into a `Hold`.
pub trait Stow<'b, T: ?Sized = Self> {
    /// Moves the value at the `src` pointer to the `dst` pointer, recursively
    /// moving all child values into the `hold`. If a child `stow` operation
    /// fails, then any already completed child `stow` operations get reverted
    /// by calling `unstow` on the child.
    unsafe fn stow(src: *mut Self, dst: *mut T, hold: &dyn Hold<'b>) -> Result<(), HoldError>;

    /// Reverts the most recent `stow` operation by moving the value at the
    /// `dst` pointer back to the `src` pointer from whence it came. The
    /// memory at the `src` address will be in the same state the `stow`
    /// operation left it in.
    ///
    /// # Safety
    ///
    /// The `src` and `dst` pointers must not be aliased outside the current
    /// call stack.
    unsafe fn unstow(src: *mut Self, dst: *mut T);
}

impl<'b, S: Stow<'b, T>, T> StowFrom<'b, S> for T {
    default fn try_stow_from(mut src: S, hold: &dyn Hold<'b>) -> Result<T, (S, HoldError)> {
        unsafe {
            let mut dst = mem::uninitialized::<T>();
            if let Err(error) = S::stow(&mut src, &mut dst, hold) {
                mem::forget(dst);
                return Err((src, error));
            }
            return Ok(dst);
        }
    }
}

impl<'b, S, T: StowFrom<'b, S>> StowInto<'b, T> for S {
    #[inline]
    fn try_stow_into(self, hold: &dyn Hold<'b>) -> Result<T, (S, HoldError)> {
        T::try_stow_from(self, hold)
    }
}

macro_rules! stow_from_value {
    ($type:ty) => (
        impl<'b> StowFrom<'b, $type> for $type {
            #[inline]
            fn try_stow_from(value: $type, _hold: &dyn Hold<'b>) -> Result<$type, ($type, HoldError)> {
                Ok(value)
            }
        }
    );
}

stow_from_value!(());
stow_from_value!(u8);
stow_from_value!(u16);
stow_from_value!(u32);
stow_from_value!(u64);
stow_from_value!(usize);
stow_from_value!(i8);
stow_from_value!(i16);
stow_from_value!(i32);
stow_from_value!(i64);
stow_from_value!(isize);
stow_from_value!(f16);
stow_from_value!(f32);
stow_from_value!(f64);
stow_from_value!(char);
stow_from_value!(bool);
stow_from_value!(Murmur3);

macro_rules! stow_value {
    ($type:ty) => (
        impl<'b> Stow<'b> for $type {
            #[inline]
            unsafe fn stow(src: *mut $type, dst: *mut $type, _hold: &dyn Hold<'b>) -> Result<(), HoldError> {
                ptr::copy_nonoverlapping(src, dst, 1);
                Ok(())
            }

            #[inline]
            unsafe fn unstow(_src: *mut $type, _dts: *mut $type) {
                // nop
            }
        }
    );
}

stow_value!(());
stow_value!(u8);
stow_value!(u16);
stow_value!(u32);
stow_value!(u64);
stow_value!(usize);
stow_value!(i8);
stow_value!(i16);
stow_value!(i32);
stow_value!(i64);
stow_value!(isize);
stow_value!(f16);
stow_value!(f32);
stow_value!(f64);
stow_value!(char);
stow_value!(bool);
stow_value!(Murmur3);

impl<'b, T: Stow<'b>> Stow<'b> for [T] {
    default unsafe fn stow(src: *mut [T], dst: *mut [T], hold: &dyn Hold<'b>) -> Result<(), HoldError> {
        let n = (*src).len();
        assert_eq!((*dst).len(), n);
        let mut src = src as *mut T;
        let mut dst = dst as *mut T;
        let mut i = 0;
        while i < n {
            if let err @ Err(_) = T::stow(src, dst, hold) {
                while i > 0 {
                    i = i.wrapping_sub(1);
                    dst = dst.wrapping_sub(1);
                    src = src.wrapping_sub(1);
                    T::unstow(src, dst);
                }
                return err;
            }
            src = src.wrapping_add(1);
            dst = dst.wrapping_add(1);
            i = i.wrapping_add(1);
        }
        Ok(())
    }

    default unsafe fn unstow(src: *mut [T], dst: *mut [T]) {
        let mut i = (*src).len();
        assert_eq!((*dst).len(), i);
        let mut src = (src as *mut T).wrapping_add(i);
        let mut dst = (dst as *mut T).wrapping_add(i);
        while i > 0 {
            i = i.wrapping_sub(1);
            dst = dst.wrapping_sub(1);
            src = src.wrapping_sub(1);
            T::unstow(src, dst);
        }
    }
}

impl<'b, T: Copy + Stow<'b>> Stow<'b> for [T] {
    unsafe fn stow(src: *mut [T], dst: *mut [T], _hold: &dyn Hold<'b>) -> Result<(), HoldError> {
        let n = (*src).len();
        assert_eq!((*dst).len(), n);
        ptr::copy_nonoverlapping(src as *mut T, dst as *mut T, n);
        Ok(())
    }

    unsafe fn unstow(_src: *mut [T], _dst: *mut [T]) {
        // nop
    }
}

impl<'b> Stow<'b> for str {
    unsafe fn stow(src: *mut str, dst: *mut str, _hold: &dyn Hold<'b>) -> Result<(), HoldError> {
        let n = (*src).len();
        assert_eq!((*dst).len(), n);
        ptr::copy_nonoverlapping(src as *mut u8, dst as *mut u8, n);
        Ok(())
    }

    unsafe fn unstow(_src: *mut str, _dst: *mut str) {
        // nop
    }
}

impl<'b, T0: Stow<'b>, T1: Stow<'b>> Stow<'b> for (T0, T1) {
    unsafe fn stow(src: *mut (T0, T1), dst: *mut (T0, T1), hold: &dyn Hold<'b>) -> Result<(), HoldError> {
        if let err @ Err(_) = T0::stow(&mut (*src).0, &mut (*dst).0, hold) {
            return err;
        }
        if let err @ Err(_) = T1::stow(&mut (*src).1, &mut (*dst).1, hold) {
            T0::unstow(&mut (*src).0, &mut (*dst).0);
            return err;
        }
        Ok(())
    }

    unsafe fn unstow(src: *mut (T0, T1), dst: *mut (T0, T1)) {
        T1::unstow(&mut (*src).1, &mut (*dst).1);
        T0::unstow(&mut (*src).0, &mut (*dst).0);
    }
}

impl<'b, T0: Stow<'b>, T1: Stow<'b>, T2: Stow<'b>> Stow<'b> for (T0, T1, T2) {
    unsafe fn stow(src: *mut (T0, T1, T2), dst: *mut (T0, T1, T2), hold: &dyn Hold<'b>) -> Result<(), HoldError> {
        if let err @ Err(_) = T0::stow(&mut (*src).0, &mut (*dst).0, hold) {
            return err;
        }
        if let err @ Err(_) = T1::stow(&mut (*src).1, &mut (*dst).1, hold) {
            T0::unstow(&mut (*src).0, &mut (*dst).0);
            return err;
        }
        if let err @ Err(_) = T2::stow(&mut (*src).2, &mut (*dst).2, hold) {
            T1::unstow(&mut (*src).1, &mut (*dst).1);
            T0::unstow(&mut (*src).0, &mut (*dst).0);
            return err;
        }
        Ok(())
    }

    unsafe fn unstow(src: *mut (T0, T1, T2), dst: *mut (T0, T1, T2)) {
        T2::unstow(&mut (*src).2, &mut (*dst).2);
        T1::unstow(&mut (*src).1, &mut (*dst).1);
        T0::unstow(&mut (*src).0, &mut (*dst).0);
    }
}

impl<'b, T0: Stow<'b>, T1: Stow<'b>, T2: Stow<'b>, T3: Stow<'b>> Stow<'b> for (T0, T1, T2, T3) {
    unsafe fn stow(src: *mut (T0, T1, T2, T3), dst: *mut (T0, T1, T2, T3), hold: &dyn Hold<'b>) -> Result<(), HoldError> {
        if let err @ Err(_) = T0::stow(&mut (*src).0, &mut (*dst).0, hold) {
            return err;
        }
        if let err @ Err(_) = T1::stow(&mut (*src).1, &mut (*dst).1, hold) {
            T0::unstow(&mut (*src).0, &mut (*dst).0);
            return err;
        }
        if let err @ Err(_) = T2::stow(&mut (*src).2, &mut (*dst).2, hold) {
            T1::unstow(&mut (*src).1, &mut (*dst).1);
            T0::unstow(&mut (*src).0, &mut (*dst).0);
            return err;
        }
        if let err @ Err(_) = T3::stow(&mut (*src).3, &mut (*dst).3, hold) {
            T2::unstow(&mut (*src).2, &mut (*dst).2);
            T1::unstow(&mut (*src).1, &mut (*dst).1);
            T0::unstow(&mut (*src).0, &mut (*dst).0);
            return err;
        }
        Ok(())
    }

    unsafe fn unstow(src: *mut (T0, T1, T2, T3), dst: *mut (T0, T1, T2, T3)) {
        T3::unstow(&mut (*src).3, &mut (*dst).3);
        T2::unstow(&mut (*src).2, &mut (*dst).2);
        T1::unstow(&mut (*src).1, &mut (*dst).1);
        T0::unstow(&mut (*src).0, &mut (*dst).0);
    }
}
