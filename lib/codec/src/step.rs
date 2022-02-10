#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Step<T> {
    In(T),
    Out,
    Over,
}
pub use self::Step::{In, Out, Over};

impl<T> Step<T> {
    #[inline]
    pub fn is_in(&self) -> bool {
        match *self {
            In(_) => true,
            _ => false,
        }
    }

    #[inline]
    pub fn is_out(&self) -> bool {
        match *self {
            Out => true,
            _ => false,
        }
    }

    #[inline]
    pub fn is_over(&self) -> bool {
        match *self {
            Over => true,
            _ => false,
        }
    }

    #[inline]
    pub fn unwrap(self) -> T {
        match self {
            In(x) => x,
            _ => panic!(),
        }
    }

    #[inline]
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Step<U> {
        match self {
            In(x) => In(f(x)),
            Out => Out,
            Over => Over,
        }
    }

    #[inline]
    pub fn map_or<U, F: FnOnce(T) -> U>(self, default: U, f: F) -> U {
        match self {
            In(x) => f(x),
            _ => default,
        }
    }

    #[inline]
    pub fn map_or_else<U, D: FnOnce() -> U, F: FnOnce(T) -> U>(self, default: D, f: F) -> U {
        match self {
            In(x) => f(x),
            _ => default(),
        }
    }

    #[inline]
    pub fn and<U>(self, alt: Step<U>) -> Step<U> {
        match self {
            In(_) => alt,
            Out => Out,
            Over => Over,
        }
    }

    #[inline]
    pub fn and_then<U, F: FnOnce(T) -> Step<U>>(self, f: F) -> Step<U> {
        match self {
            In(x) => f(x),
            Out => Out,
            Over => Over,
        }
    }

    #[inline]
    pub fn or(self, alt: Step<T>) -> Step<T> {
        match self {
            In(_) => self,
            _ => alt,
        }
    }

    #[inline]
    pub fn or_else<F: FnOnce() -> Step<T>>(self, f: F) -> Step<T> {
        match self {
            In(_) => self,
            _ => f(),
        }
    }

    #[inline]
    pub fn ok_or<E>(self, err: E) -> Result<T, E> {
        match self {
            In(x) => Ok(x),
            _ => Err(err),
        }
    }

    #[inline]
    pub fn ok_or_else<E, F: FnOnce() -> E>(self, err: F) -> Result<T, E> {
        match self {
            In(x) => Ok(x),
            _ => Err(err()),
        }
    }

    #[inline]
    pub fn into_option(self) -> Option<T> {
        match self {
            In(x) => Some(x),
            _ => None,
        }
    }
}

impl<T> Into<Option<T>> for Step<T> {
    #[inline]
    fn into(self) -> Option<T> {
        self.into_option()
    }
}
