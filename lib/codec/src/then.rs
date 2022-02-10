#[derive(Clone, Debug)]
pub enum Then<C, D, E> {
    Cont(C),
    Done(D),
    Fail(E),
}
pub use self::Then::{Cont, Done, Fail};
