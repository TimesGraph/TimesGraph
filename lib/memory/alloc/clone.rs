use tg_core::f16;
use crate::alloc::{Hold, HoldError};

/// Failable clone.
pub trait TryClone: Sized {
    /// Returns a clone of `self`; returns an error if the clone fails.
    fn try_clone(&self) -> Result<Self, HoldError>;
}

/// Clone into a specific allocation `Hold`.
pub trait CloneIntoHold<'a, T = Self>: Sized {
    /// Returns a clone of `self` allocated in `hold`; returns an error if the
    /// clone fails.
    fn try_clone_into_hold(&self, hold: &Hold<'a>) -> Result<T, HoldError>;

    /// Returns a clone of `self` allocated in `hold`.
    ///
    /// # Panics
    ///
    /// Panics if the clone fails.
    #[inline]
    fn clone_into_hold(&self, hold: &Hold<'a>) -> T {
        self.try_clone_into_hold(hold).unwrap()
    }
}

macro_rules! try_clone_value {
    ($type:ty) => (
        impl TryClone for $type {
            #[inline]
            fn try_clone(&self) -> Result<$type, HoldError> {
                Ok(*self)
            }
        }
    );
}

try_clone_value!(());
try_clone_value!(u8);
try_clone_value!(u16);
try_clone_value!(u32);
try_clone_value!(u64);
try_clone_value!(usize);
try_clone_value!(i8);
try_clone_value!(i16);
try_clone_value!(i32);
try_clone_value!(i64);
try_clone_value!(isize);
try_clone_value!(f16);
try_clone_value!(f32);
try_clone_value!(f64);
try_clone_value!(char);
try_clone_value!(bool);

impl<T: TryClone> TryClone for Option<T> {
    #[inline]
    fn try_clone(&self) -> Result<Option<T>, HoldError> {
        Ok(match self {
            Some(value) => Some(value.try_clone()?),
            None => None,
        })
    }
}

impl<T0, T1> TryClone for (T0, T1)
    where T0: TryClone,
          T1: TryClone,
{
    fn try_clone(&self) -> Result<(T0, T1), HoldError> {
        let v0 = self.0.try_clone()?;
        let v1 = self.1.try_clone()?;
        Ok((v0, v1))
    }
}

impl<T0, T1, T2> TryClone for (T0, T1, T2)
    where T0: TryClone,
          T1: TryClone,
          T2: TryClone,
{
    fn try_clone(&self) -> Result<(T0, T1, T2), HoldError> {
        let v0 = self.0.try_clone()?;
        let v1 = self.1.try_clone()?;
        let v2 = self.2.try_clone()?;
        Ok((v0, v1, v2))
    }
}

impl<T0, T1, T2, T3> TryClone for (T0, T1, T2, T3)
    where T0: TryClone,
          T1: TryClone,
          T2: TryClone,
          T3: TryClone,
{
    fn try_clone(&self) -> Result<(T0, T1, T2, T3), HoldError> {
        let v0 = self.0.try_clone()?;
        let v1 = self.1.try_clone()?;
        let v2 = self.2.try_clone()?;
        let v3 = self.3.try_clone()?;
        Ok((v0, v1, v2, v3))
    }
}

macro_rules! clone_value_into_hold {
    ($type:ty) => (
        impl<'a> CloneIntoHold<'a, $type> for $type {
            #[inline]
            fn try_clone_into_hold(&self, _hold: &Hold<'a>) -> Result<$type, HoldError> {
                Ok(*self)
            }

            #[inline]
            fn clone_into_hold(&self, _hold: &Hold<'a>) -> $type {
                *self
            }
        }
    );
}

clone_value_into_hold!(());
clone_value_into_hold!(u8);
clone_value_into_hold!(u16);
clone_value_into_hold!(u32);
clone_value_into_hold!(u64);
clone_value_into_hold!(usize);
clone_value_into_hold!(i8);
clone_value_into_hold!(i16);
clone_value_into_hold!(i32);
clone_value_into_hold!(i64);
clone_value_into_hold!(isize);
clone_value_into_hold!(f16);
clone_value_into_hold!(f32);
clone_value_into_hold!(f64);
clone_value_into_hold!(char);
clone_value_into_hold!(bool);

impl<'a, T, U: CloneIntoHold<'a, T>> CloneIntoHold<'a, Option<T>> for Option<U> {
    #[inline]
    fn try_clone_into_hold(&self, hold: &Hold<'a>) -> Result<Option<T>, HoldError> {
        Ok(match self {
            Some(value) => Some(value.try_clone_into_hold(hold)?),
            None => None,
        })
    }
}

impl<'a, T0, T1, U0, U1> CloneIntoHold<'a, (T0, T1)> for (U0, U1)
    where U0: CloneIntoHold<'a, T0>,
          U1: CloneIntoHold<'a, T1>,
{
    fn try_clone_into_hold(&self, hold: &Hold<'a>) -> Result<(T0, T1), HoldError> {
        let v0 = self.0.try_clone_into_hold(hold)?;
        let v1 = self.1.try_clone_into_hold(hold)?;
        Ok((v0, v1))
    }
}

impl<'a, T0, T1, T2, U0, U1, U2> CloneIntoHold<'a, (T0, T1, T2)> for (U0, U1, U2)
    where U0: CloneIntoHold<'a, T0>,
          U1: CloneIntoHold<'a, T1>,
          U2: CloneIntoHold<'a, T2>,
{
    fn try_clone_into_hold(&self, hold: &Hold<'a>) -> Result<(T0, T1, T2), HoldError> {
        let v0 = self.0.try_clone_into_hold(hold)?;
        let v1 = self.1.try_clone_into_hold(hold)?;
        let v2 = self.2.try_clone_into_hold(hold)?;
        Ok((v0, v1, v2))
    }
}

impl<'a, T0, T1, T2, T3, U0, U1, U2, U3> CloneIntoHold<'a, (T0, T1, T2, T3)> for (U0, U1, U2, U3)
    where U0: CloneIntoHold<'a, T0>,
          U1: CloneIntoHold<'a, T1>,
          U2: CloneIntoHold<'a, T2>,
          U3: CloneIntoHold<'a, T3>,
{
    fn try_clone_into_hold(&self, hold: &Hold<'a>) -> Result<(T0, T1, T2, T3), HoldError> {
        let v0 = self.0.try_clone_into_hold(hold)?;
        let v1 = self.1.try_clone_into_hold(hold)?;
        let v2 = self.2.try_clone_into_hold(hold)?;
        let v3 = self.3.try_clone_into_hold(hold)?;
        Ok((v0, v1, v2, v3))
    }
}
