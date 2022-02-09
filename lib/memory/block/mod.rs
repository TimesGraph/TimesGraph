//! Untyped memory model.

use core::ptr;

mod block;
mod layout;

pub use self::block::Block;
pub use self::layout::{Layout, LayoutError};
/// zero sentinel pointer
/// Non-zero sentinel pointer to a zero-sized value.
pub const ZSP: *mut u8 = 1 as *mut u8;

#[inline]
pub(crate) unsafe fn set_address<T: ?Sized>(mut pointer: *mut T, address: usize) -> *mut T {
    // Overwrite the address component of the pointer with the new address.
    ptr::write(&mut pointer as *mut *mut T as *mut usize, address);
    // Return the rewritten pointer.
    pointer
}
