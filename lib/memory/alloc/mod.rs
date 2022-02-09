//! Dynamic memory allocators and operators.

mod heap;
mod hold;
mod tag;

mod stow;
mod clone;

mod addr;
mod slab;
mod pack;
mod pool;

pub use self::heap::{Heap, HeapError};
pub use self::hold::{Hold, HoldScope, LocalHold, Holder, HoldError};
pub use self::tag::AllocTag;

pub use self::stow::{Stow, StowFrom, StowInto};
pub use self::clone::{TryClone, CloneIntoHold};

pub use self::addr::{AddrSpace, ExtentList};
pub use self::slab::Slab;
pub use self::pack::Pack;
pub use self::pool::Pool;
