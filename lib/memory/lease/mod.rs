//! Memory ownership and access model.
//!
//! # Leases
//!
//! A `Lease` abstracts over the allocation and ownership semantics of a raw,
//! unsized memory block. Most `Lease` implementations compose a `Resident`,
//! type, which abstracts over the usage of the leased memory block.
//!
//! The most commonly used `Lease` implementations include:
//! - __[`Raw`]__: the exclusive owner of a relocatable, raw memory block, with
//!   resident metadata stored with the pointer.
//! - __[`Ptr`]__: the exclusive owner of a relocatable memory block, with
//!   resident metadata stored within the allocation.
//! - __[`Mut`]__: a mutably dereferenceable strong owner of an unrelocatable,
//!   reference counted memory block.
//! - __[`Ref`]__: an immutably dereferenceable strong owner of an
//!   unrelocatable, reference counted memory block.
//! - __[`Hard`]__: an undereferenceable strong owner of a relocatable,
//!   reference counted memory block.
//! - __[`Soft`]__: an undereferenceable weak owner of a relocatable,
//!   reference counted memory block.
//!
//! Commonly composed `Resident` implementations include:
//! - __[`Box`]__: stores a single value in a leased memory block.
//! - __[`Buf`]__: stores a resizeable array of values in a leased memory block.
//! - __[`String`]__: stores a resizeable Unicode string in a leased memory block.
//!
//! ## Lease residents
//!
//! `Lease` implementations conditionally implement core Rust traits, such as
//! `Deref`, `Eq`, and `Hash`, dependent upon whether a `Lease`'s parameterized
//! `Resident` type implements corresponding `Resident*` traits, like
//! `ResodentDeref`, `ResidentEq`, and `ResidentHash`, respectively. These
//! `Resident*` proxy traits differ from their corresponding core Rust traits
//! in that they take a `Lease` type in lieu of a `Self` type as their first
//! argument. Implementations of the `Resident*` proxy traits often convert
//! their `Lease` argument into some concrete reference type, and then delegate
//! to that type's implementation of the corresponding core Rust trait.
//!
//! For example, the `Lease` type `Raw<R: Resident>` implements `Hash` if
//! and only if `R` implements `ResidentHash`. Furthermore, the `Resident` type
//! `Box<T>` implements `ResidentHash` if and only if `T` implemenets `Hash`.
//! Consequently, the composed type `Raw<Box<T>>` transitively implements `Hash`
//! if and only if `T` implements `Hash`. The `ResidentHash` indirection
//! effectively dereferences the `Lease` into a reference to type `T`.
//!
//! Indirection through `Resident*` proxy traits typically has zero cost;
//! each trait is specialized by a concrete `Lease` type, and its methods get
//! frequently inlined.
//!
//! ## Raw leases
//!
//! A [`Raw`] lease represents an exclusive reference to the `Resident` stored
//! in its memory block. Similar to Rust's `std::boxed::Box`, a `Raw` lease is
//! not reference counted, and stores no metadata on the heap. `Raw` leases can
//! be freely dereferenced. And resident metadata, such as the `BufHeader`
//! associated with every `Buf` resident, is stored directly in the `Raw`
//! struct, similar to how `std::vec::Vec` stores its length and capacity in
//! the `Vec` struct, rather than on the heap.
//!
//! Type aliases for `Raw` leases with common `Resident` types include:
//! - __[`RawBox`]__: an exclusively owned value.
//! - __[`RawBuf`]__: an exclusively owned resizeable array of values.
//! - __[`RawString`]__: an exclusively owned resizeable Unicode string.
//!
//! [`Raw`]: lease::Raw
//! [`Ptr`]: lease::Ptr
//! [`Mut`]: lease::Mut
//! [`Ref`]: lease::Ref
//! [`Hard`]: lease::Hard
//! [`Soft`]: lease::Soft
//!
//! [`Box`]: resident::Box
//! [`Buf`]: resident::Buf
//! [`String`]: resident::String
//!
//! [`RawBox`]: lease::RawBox
//! [`RawBuf`]: lease::RawBuf
//! [`RawString`]: lease::RawString

use crate::block::Layout;
use crate::alloc::{Holder, HoldError};
use crate::resident::{Box, Buf, String};

mod raw;
mod ptr;
mod arc;
mod r#mut;
mod r#ref;
mod hard;
mod soft;

pub use self::raw::Raw;
pub use self::ptr::Ptr;
pub use self::arc::{Arc, ArcHeader, ArcError};
pub use self::arc::{HARD_COUNT_MAX, SOFT_COUNT_MAX, REF_COUNT_MAX};
pub use self::r#mut::Mut;
pub use self::r#ref::Ref;
pub use self::hard::Hard;
pub use self::soft::Soft;

/// Exclusive reference to a value stored in a `Hold`-allocated memory block,
/// with optional metadata stored alongside the pointer.
///
/// Storing metadata in the pointer structure keeps the allocated memory block
/// exactly the size of the value.
///
/// # Examples
///
/// Move a value from the stack to the global hold by creating a `RawBox`:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::RawBox;
/// let value = 5;
/// let boxed = RawBox::new(value);
/// # assert_eq!(*boxed, 5);
/// ```
///
/// Move a value from a `RawBox` back to the stack by dereferencing:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::RawBox;
/// let boxed = RawBox::new(5);
/// let value = *boxed;
/// # assert_eq!(value, 5);
/// ```
pub type RawBox<'a, T, M = ()> = Raw<'a, Box<T, M>>;

/// Exclusive reference to a resizeable array of values stored in a
/// `Hold`-allocated memory block, with buffer length and capacity metadata
/// stored alongside the pointer.
///
/// Storing buffer metadata in the pointer structure keeps the allocated memory
/// block exactly the size of the buffer capacity.
///
/// # Examples
///
/// Create an empty `RawBuf` that will allocate space in the global hold:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::RawBuf;
/// let buf = RawBuf::<u8>::empty();
/// # assert_eq!(buf.len(), 0);
/// # assert_eq!(buf.cap(), 0);
/// ```
///
/// Clone a slice into a newly allocated `RawBuf`:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::RawBuf;
/// let buf = RawBuf::<u8>::from_clone(&[2, 3]);
/// # assert_eq!(buf.len(), 2);
/// # assert_eq!(buf.cap(), 2);
/// ```
///
/// Push values onto the end of a `RawBuf`, growing its capacity as needed:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::RawBuf;
/// let mut buf = RawBuf::<u8>::from_clone(&[1, 2]);
/// buf.push(3);
/// # assert_eq!(buf.len(), 3);
/// ```
///
/// Pop values off of the end of a `RawBuf`:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::RawBuf;
/// let mut buf = RawBuf::<u8>::from_clone(&[1, 2]);
/// let two = buf.pop();
/// # assert_eq!(two, Some(2));
/// # assert_eq!(buf.len(), 1);
/// ```
///
/// Access elements by index:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::RawBuf;
/// let mut buf = RawBuf::<u8>::from_clone(&[1, 2, 3]);
/// let three = buf[2];
/// # assert_eq!(three, 3);
/// buf[1] *= 2;
/// # assert_eq!(buf[1], 4);
/// ```
pub type RawBuf<'a, T, M = ()> = Raw<'a, Buf<T, M>>;

/// Exclusive reference to a resizeable Unicode string stored in a
/// `Hold`-allocated memory block, with string length and capacity metadata
/// stored alongside the pointer.
///
/// Storing string metadata in the pointer structure keeps the allocated memory
/// block exactly the size of the string capacity.
///
/// # Examples
///
/// Copy a string literal into a `RawString` allocated in the global hold:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::RawString;
/// let s = RawString::from_copy("Hello");
/// # assert_eq!(s.len(), 5);
/// # assert_eq!(s.cap(), 5);
/// ```
///
/// Concatenate a `RawString` with another `str` using the `+` operator:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::RawString;
/// let s = RawString::from_copy("Hello");
/// let message = s + " world";
/// # assert_eq!(message.len(), 11);
/// ```
pub type RawString<'a, M = ()> = Raw<'a, String<M>>;

/// Exclusive reference to a value stored in a `Hold`-allocated memory block,
/// with optional metadata stored inside the allocation.
///
/// Storing metadata in the memory block keeps the pointer structure exactly
/// the size of a pointer.
///
/// # Examples
///
/// Move a value from the stack to the global hold by creating a `PtrBox`:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::PtrBox;
/// let value = 5;
/// let boxed = PtrBox::new(value);
/// # assert_eq!(*boxed, 5);
/// ```
///
/// Move a value from a `PtrBox` back to the stack by dereferencing:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::PtrBox;
/// let boxed = PtrBox::new(5);
/// let value = *boxed;
/// # assert_eq!(value, 5);
/// ```
pub type PtrBox<'a, T, M = ()> = Ptr<'a, Box<T, M>>;

/// Exclusive reference to a resizeable array of values stored in a
/// `Hold`-allocated memory block, with buffer length and capacity metadata
/// stored inside the allocation.
///
/// Storing buffer metadata in the memory block keeps the pointer structure
/// exactly the size of an element pointer.
///
/// # Examples
///
/// Create an empty `PtrBuf` that will allocate space in the global hold:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::PtrBuf;
/// let buf = PtrBuf::<u8>::empty();
/// # assert_eq!(buf.len(), 0);
/// # assert_eq!(buf.cap(), 0);
/// ```
///
/// Clone a slice into a newly allocated `PtrBuf`:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::PtrBuf;
/// let buf = PtrBuf::<u8>::from_clone(&[2, 3]);
/// # assert_eq!(buf.len(), 2);
/// # assert_eq!(buf.cap(), 2);
/// ```
///
/// Push values onto the end of a `PtrBuf`, growing its capacity as needed:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::PtrBuf;
/// let mut buf = PtrBuf::<u8>::from_clone(&[1, 2]);
/// buf.push(3);
/// # assert_eq!(buf.len(), 3);
/// ```
///
/// Pop values off of the end of a `PtrBuf`:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::PtrBuf;
/// let mut buf = PtrBuf::<u8>::from_clone(&[1, 2]);
/// let two = buf.pop();
/// # assert_eq!(two, Some(2));
/// # assert_eq!(buf.len(), 1);
/// ```
///
/// Access elements by index:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::PtrBuf;
/// let mut buf = PtrBuf::<u8>::from_clone(&[1, 2, 3]);
/// let three = buf[2];
/// # assert_eq!(three, 3);
/// buf[1] *= 2;
/// # assert_eq!(buf[1], 4);
/// ```
pub type PtrBuf<'a, T, M = ()> = Ptr<'a, Buf<T, M>>;

/// Exclusive reference to a resizeable Unicode string stored in a
/// `Hold`-allocated memory block, with string length and capacity metadata
/// stored inside the allocation.
///
/// Storing string metadata in the memory block keeps the pointer structure
/// exactly the size of a thin pointer.
///
/// # Examples
///
/// Copy a string literal into a `PtrString` allocated in the global hold:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::PtrString;
/// let s = PtrString::from_copy("Hello");
/// # assert_eq!(s.len(), 5);
/// # assert_eq!(s.cap(), 5);
/// ```
///
/// Concatenate a `PtrString` with another `str` using the `+` operator:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::PtrString;
/// let s = PtrString::from_copy("Hello");
/// let message = s + " world";
/// # assert_eq!(message.len(), 11);
/// ```
pub type PtrString<'a, M = ()> = Ptr<'a, String<M>>;

/// Mutably dereferenceable strong owner of a value stored in a
/// `Hold`-allocated, atomically reference counted memory block.
///
/// # Examples
///
/// Move a value from the stack to the global hold by creating a `MutBox`:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::MutBox;
/// let value = 5;
/// let boxed = MutBox::new(value);
/// # assert_eq!(*boxed, 5);
/// ```
///
/// Move a value from a `MutBox` back to the stack by dereferencing:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::MutBox;
/// let boxed = MutBox::new(5);
/// let value = *boxed;
/// # assert_eq!(value, 5);
/// ```
pub type MutBox<'a, T, M = ()> = Mut<'a, Box<T, M>>;

/// Mutably dereferenceable strong owner of a resizeable array of values
/// stored in a `Hold`-allocated, atomically reference counted memory block.
///
/// # Examples
///
/// Create an empty `MutBuf` that will allocate space in the global hold:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::MutBuf;
/// let buf = MutBuf::<u8>::empty();
/// # assert_eq!(buf.len(), 0);
/// # assert_eq!(buf.cap(), 0);
/// ```
///
/// Clone a slice into a newly allocated `MutBuf`:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::MutBuf;
/// let buf = MutBuf::<u8>::from_clone(&[2, 3]);
/// # assert_eq!(buf.len(), 2);
/// # assert_eq!(buf.cap(), 2);
/// ```
///
/// Push values onto the end of a `MutBuf`, growing its capacity as needed:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::MutBuf;
/// let mut buf = MutBuf::<u8>::from_clone(&[1, 2]);
/// buf.push(3);
/// # assert_eq!(buf.len(), 3);
/// ```
///
/// Pop values off of the end of a `MutBuf`:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::MutBuf;
/// let mut buf = MutBuf::<u8>::from_clone(&[1, 2]);
/// let two = buf.pop();
/// # assert_eq!(two, Some(2));
/// # assert_eq!(buf.len(), 1);
/// ```
///
/// Access elements by index:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::MutBuf;
/// let mut buf = MutBuf::<u8>::from_clone(&[1, 2, 3]);
/// let three = buf[2];
/// # assert_eq!(three, 3);
/// buf[1] *= 2;
/// # assert_eq!(buf[1], 4);
/// ```
pub type MutBuf<'a, T, M = ()> = Mut<'a, Buf<T, M>>;

/// Mutably dereferenceable strong owner of a resizeable Unicode string
/// stored in a `Hold`-allocated, atomically reference counted memory block.
///
/// # Examples
///
/// Copy a string literal into a `MutString` allocated in the global hold:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::MutString;
/// let s = MutString::from_copy("Hello");
/// # assert_eq!(s.len(), 5);
/// # assert_eq!(s.cap(), 5);
/// ```
///
/// Concatenate a `MutString` with another `str` using the `+` operator:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::MutString;
/// let s = MutString::from_copy("Hello");
/// let message = s + " world";
/// # assert_eq!(message.len(), 11);
/// ```
pub type MutString<'a, M = ()> = Mut<'a, String<M>>;

/// Immutably dereferenceable strong owner of a value stored in a
/// `Hold`-allocated, atomically reference counted memory block.
///
/// # Examples
///
/// Move a value from the stack to the global hold by creating a `RefBox`:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::RefBox;
/// let value = 5;
/// let boxed = RefBox::new(value);
/// # assert_eq!(*boxed, 5);
/// ```
///
/// Move a value from a `RefBox` back to the stack by dereferencing:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::RefBox;
/// let boxed = RefBox::new(5);
/// let value = *boxed;
/// # assert_eq!(value, 5);
/// ```
pub type RefBox<'a, T, M = ()> = Ref<'a, Box<T, M>>;

/// Immutably dereferenceable strong owner of a resizeable array of values
/// stored in a `Hold`-allocated, atomically reference counted memory block.
///
/// # Examples
///
/// Create an empty `RefBuf` that will allocate space in the global hold:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::RefBuf;
/// let buf = RefBuf::<u8>::empty();
/// # assert_eq!(buf.len(), 0);
/// # assert_eq!(buf.cap(), 0);
/// ```
///
/// Clone a slice into a newly allocated `RefBuf`:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::RefBuf;
/// let buf = RefBuf::<u8>::from_clone(&[2, 3]);
/// # assert_eq!(buf.len(), 2);
/// # assert_eq!(buf.cap(), 2);
/// ```
///
/// Access elements by index:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::RefBuf;
/// let buf = RefBuf::<u8>::from_clone(&[1, 2, 3]);
/// let three = buf[2];
/// # assert_eq!(three, 3);
/// ```
pub type RefBuf<'a, T, M = ()> = Ref<'a, Buf<T, M>>;

/// Immutably dereferenceable strong owner of a resizeable Unicode string
/// stored in a `Hold`-allocated, atomically reference counted memory block.
///
/// # Examples
///
/// Copy a string literal into a `RefString` allocated in the global hold:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::RefString;
/// let s = RefString::from_copy("Hello");
/// # assert_eq!(s.len(), 5);
/// # assert_eq!(s.cap(), 5);
/// ```
pub type RefString<'a, M = ()> = Ref<'a, String<M>>;

/// Undereferenceable strong owner of a value stored in a `Hold`-allocated,
/// atomically reference counted memory block.
///
/// # Examples
///
/// Move a value from the stack to the global hold by creating a `HardBox`:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::HardBox;
/// let value = 5;
/// let boxed = HardBox::new(value);
/// # assert_eq!(*boxed.into_ref(), 5);
/// ```
///
/// Move a value from a `HardBox` back to the stack by unwrapping it:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::HardBox;
/// let boxed = HardBox::new(5);
/// let value = boxed.unwrap();
/// # assert_eq!(value, 5);
/// ```
pub type HardBox<'a, T, M = ()> = Hard<'a, Box<T, M>>;

/// Undereferenceable strong owner of a resizeable array of values stored in a
/// `Hold`-allocated, atomically reference counted memory block.
///
/// # Examples
///
/// Create an empty `HardBuf` that will allocate space in the global hold:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::HardBuf;
/// let buf = HardBuf::<u8>::empty();
/// # assert_eq!(buf.to_ref().len(), 0);
/// # assert_eq!(buf.to_ref().cap(), 0);
/// ```
///
/// Clone a slice into a newly allocated `HardBuf`:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::HardBuf;
/// let buf = HardBuf::<u8>::from_clone(&[2, 3]);
/// # assert_eq!(buf.to_ref().len(), 2);
/// # assert_eq!(buf.to_ref().cap(), 2);
/// ```
///
/// Obtain a mutably dereferenceable `MutBuf` lease from a `HardBuf`, copying
/// its contents into a new, uniquely referenced buffer if and only if the old
/// buffer is aliased, and push values onto the end of the dealiased buffer,
/// growing its capacity as needed:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::HardBuf;
/// let mut buf = HardBuf::<u8>::from_clone(&[1, 2]);
/// buf.to_unique().push(3);
/// # assert_eq!(buf.to_ref().len(), 3);
/// ```
///
/// Obtain an immutably dereferenceable `RefBuf` lease from a `HardBuf`, and
/// access its elements by index:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::HardBuf;
/// let buf = HardBuf::<u8>::from_clone(&[1, 2, 3]);
/// let three = buf.to_ref()[2];
/// # assert_eq!(three, 3);
/// ```
pub type HardBuf<'a, T, M = ()> = Hard<'a, Buf<T, M>>;

/// Undereferenceable strong owner of a resizeable Unicode string stored in a
/// `Hold`-allocated, atomically reference counted memory block.
///
/// # Examples
///
/// Copy a string literal into a `HardString` allocated in the global hold:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::HardString;
/// let s = HardString::from_copy("Hello");
/// # assert_eq!(s.to_ref().len(), 5);
/// # assert_eq!(s.to_ref().cap(), 5);
/// ```
///
/// Obtain a mutably dereferenceable `MutString` lease from a `HardString`,
/// copying its contents into a new, uniquely referenced string if and only if
/// the old string is aliased, and concatenate it with another `str` using the
/// `+` operator:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::HardString;
/// let s = HardString::from_copy("Hello");
/// let message = s.to_unique() + " world";
/// # assert_eq!(message.len(), 11);
/// ```
pub type HardString<'a, M = ()> = Hard<'a, String<M>>;

/// Undereferenceable weak owner of a value stored in a `Hold`-allocated,
/// atomically reference counted memory block.
///
/// # Examples
///
/// Create a strongly referenced box, then obtain a weak reference to it:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::HardBox;
/// let hard = HardBox::new(5);
/// let soft = hard.to_soft();
/// # assert_eq!(*soft.into_ref(), 5);
/// ```
///
/// Recover a strong reference from a soft reference:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::HardBox;
/// let hard = HardBox::new(5);
/// let soft = hard.to_soft();
/// let recover = soft.try_to_ref().unwrap();
/// # assert_eq!(*recover, 5);
/// ```
pub type SoftBox<'a, T, M = ()> = Soft<'a, Box<T, M>>;

/// Undereferenceable weak owner of a resizeable array of values stored in a
/// `Hold`-allocated, atomically reference counted memory block.
///
/// # Examples
///
/// Create a strongly referenced buffer, then obtain a weak reference to it:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::HardBuf;
/// let hard = HardBuf::<u8>::from_clone(&[2, 3]);
/// let soft = hard.to_soft();
/// # assert_eq!(soft.to_ref().len(), 2);
/// # assert_eq!(soft.to_ref().cap(), 2);
/// ```
///
/// Recover a strong mutable reference from a soft reference, and push values
/// onto the end of the buffer, growing its capacity as needed:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::HardBuf;
/// let hard = HardBuf::<u8>::from_clone(&[1, 2]);
/// let soft = hard.to_soft();
/// let mut recover = unsafe { soft.try_to_mut() }.unwrap();
/// recover.push(3);
/// # std::mem::drop(recover);            // Drop MutBuf to prevent
/// # assert_eq!(soft.to_ref().len(), 3); // deadlock with assert.
/// ```
///
/// Recover a strong immutable reference from a soft reference, and access its
/// its elements by index:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::HardBuf;
/// let hard = HardBuf::<u8>::from_clone(&[1, 2, 3]);
/// let soft = hard.to_soft();
/// let recover = soft.try_to_ref().unwrap();
/// let three = recover[2];
/// # assert_eq!(three, 3);
/// ```
pub type SoftBuf<'a, T, M = ()> = Soft<'a, Buf<T, M>>;

/// Undereferenceable weak owner of a resizeable Unicode string stored in a
/// `Hold`-allocated, atomically reference counted memory block.
///
/// # Examples
///
/// Create a strongly referenced string, then obtain a weak reference to it:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::HardString;
/// let hard = HardString::from_copy("Hello");
/// let soft = hard.to_soft();
/// # assert_eq!(soft.to_ref().len(), 5);
/// # assert_eq!(soft.to_ref().cap(), 5);
/// ```
///
/// Recover a strong mutable reference from a soft reference, and concatenate
/// it with another `str` using the `+` operator:
///
/// ```
/// # extern crate tg_c_rt;
/// # use tg_mem::lease::HardString;
/// let hard = HardString::from_copy("Hello");
/// let soft = hard.to_soft();
/// let recover = unsafe { soft.try_to_mut() }.unwrap();
/// let message = recover + " world";
/// # std::mem::drop(message);             // Drop MutString to prevent
/// # assert_eq!(soft.to_ref().len(), 11); // deadlock with assert.
/// ```
pub type SoftString<'a, M = ()> = Soft<'a, String<M>>;

/// A raw memory block, with associated metadata. A memory `Lease` abstracts
/// over the ownership semantics of a raw, unsized memory block.
///
/// # Requirements
///
/// A `Lease` manages the lifetime, placement, and ownership semantics of its
/// data and metadata pointers. A `Lease` may alias or relocate those pointers
/// at any time, so long as it doesn't violate Rust's borrowing semantics.
///
/// Each `Lease` implementation defines its own way of determining the size
/// of its memory blocks. Most `Lease` implementations delegate to a composed
/// `Resident` type to make its size determinations. A `Lease` must not
/// assume that the size of its memory block equals the size of its `data`
/// pointer.
///
/// Every `Lease` implementation also provide storage for a statically sized
/// metadata value, of type `Meta`. `Raw` leases store metadata in their
/// pointer structures. `Mut`, `Ref`, `Hard`, and `Soft` leases store
/// metadata in the memory immediately preceding their shared values.
pub trait Lease {
    /// The type of pointed-to data stored in leased memory blocks. The size of
    /// leased memory blocks must be a positive multiple of the `Data` size.
    type Data: ?Sized;

    /// The type of metadata stored with leased memory blocks. `Meta` data must
    /// contain sufficient information to resolve the size of any resided-in
    /// memory `Lease`.
    type Meta;

    /// Returns a pointer to the leased memory block. The size of the
    /// pointed-to memory block is an implementation-defined multiple of
    /// the size of the `Data` type. The `Lease` makes no guarantees about
    /// the initinialization state of the pointed-to data; the resident of
    /// the `Lease` takes full responsibility for managing its own data.
    fn data(&self) -> *mut Self::Data;

    /// Returns a pointer to the metadata associated with the leased memory
    /// block. The `Lease` makes no guarantees about the initialization state
    /// of the pointed-to metadata; the resident of the `Lease` takes full
    /// responsibility for managing its own metadata.
    fn meta(&self) -> *mut Self::Meta;
}

/// A resizeable memory `Lease`.
pub trait DynamicLease<'a>: Holder<'a> + Lease {
    /// Resizes the leased memory block in place to fit the new `layout`,
    /// leaves the existing memory block in its original state on failure.
    unsafe fn resize(&mut self, layout: Layout) -> Result<(), HoldError>;

    /// Reallocates the leased memory block to fit the new `layout`; leaves
    /// the existing memory block in its original state on failure.
    unsafe fn realloc(&mut self, layout: Layout) -> Result<(), HoldError>;
}
