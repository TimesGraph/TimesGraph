//! Memory usage model.
//!
//! # Residents
//!
//! A memory `Resident` abstracts over the usage of a memory `Lease`, which
//! itself abstracts over the allocation and ownership semantics of a raw,
//! unsized memory block.
//!
//! The most commonly used `Resident` implementations include:
//! - __[`Box`]__: stores a single value in a raw memory block.
//! - __[`Buf`]__: stores a resizeable array of values in a raw memory block.
//! - __[`String`]__: stores a resizeable Unicode string in a raw memory block.
//!
//! `Lease` implementations that compose a `Resident` include:
//! - __[`Raw`]__: the exclusive owner of a relocatable, raw memory block,
//!   with resident metadata stored with the pointer.
//! - __[`Ptr`]__: the exclusive owner of a relocatable, raw memory block,
//!   with resident metadata stored within the allocation.
//! - __[`Mut`]__: a mutably dereferenceable strong owner of an unrelocatable,
//!   reference counted memory block.
//! - __[`Ref`]__: an immutably dereferenceable strong owner of an unrelocatable,
//!   reference counted memory block.
//! - __[`Hard`]__: an undereferenceable strong owner of a relocatable, reference
//!   counted memory block.
//! - __[`Soft`]__: an undereferenceable weak owner of a relocatable, reference
//!   counted memory block.
//!
//! [`Box`]: resident::Box
//! [`Buf`]: resident::Buf
//! [`String`]: resident::String
//!
//! [`Raw`]: lease::Raw
//! [`Ptr`]: lease::Ptr
//! [`Mut`]: lease::Mut
//! [`Ref`]: lease::Ref
//! [`Hard`]: lease::Hard
//! [`Soft`]: lease::Soft

use core::cmp::Ordering;
use core::fmt;
use core::hash::Hasher;
use crate::block::{Layout, LayoutError};
use crate::alloc::{Hold, HoldError};
use crate::lease::Lease;

mod r#box;
mod buf;
mod string;

pub use self::r#box::Box;
pub use self::buf::{Buf, BufHeader, BufLease, BufIter, BufDrain};
pub use self::string::{String, StringLease};

/// A type that can reside in a raw, unsized memory block. A memory `Resident`
/// abstracts over the usage of a memory `Lease`, which itself abstracts over
/// the allocation and ownership semantics of a raw, unsized memory block.
///
/// # Requirements
///
/// `Resident` implementation don't store data internally; they proxy all data
/// accesses through a passed-in `Lease`, which acts as the de facto `self`
/// argument for all `Resident` methods. A memory `Lease` grants its `Resident`
/// temporary access to a data pointer, of type `Data`, which may be a fat,
/// unsized pointer. A `Lease` also provides its `Resident` access to a sized
/// metadata pointer, of type `Meta`.
///
/// A `Lease` manages the lifetime, placement, and ownership semantics of its
/// data and metadata pointers. A `Lease` may alias or relocate those pointers
/// at any time, so long as it doesn't violate Rust's borrowing semantics.
///
/// A `Lease` delegates to its `Resident` to determine the size and alignment
/// of any leased memory blocks; a `Lease` cannot assume that the size of its
/// memory block equals the size of its `Resident`'s `*Data`.
///
/// For example, a `Box` uses the size of its pointed-to `data`, possibly via
/// a dynamically sized fat pointer, as the size of its memory block. Whereas
/// a `Buf` determines its memory block size by multiplying the static size
/// of its `Data` type by a capacity value stored in the `Lease`'s associated
/// metadata. Abstracting over these size and usage patterns is the primary
/// reason for `Resident`'s existence.
pub trait Resident {
    /// The type of pointed-to data stored in leased memory blocks. The size of
    /// leased memory blocks must be a positive multiple of the `Data` size.
    type Data: ?Sized;

    /// The type of metadata stored with leased memory blocks. `Meta` data must
    /// contain sufficient information to resolve the size of any resided-in
    /// memory `Lease`.
    type Meta;

    /// Returns the size in bytes of the `Resident` with the given `data` and `meta` data.
    unsafe fn resident_size(data: *mut Self::Data, meta: *mut Self::Meta) -> usize;

    /// Drops the `Resident` with the given `data` and `meta` data.
    unsafe fn resident_drop(data: *mut Self::Data, meta: *mut Self::Meta);
}

/// A type that can initialize a new memory `Lease` with a `Resident` value.
pub trait ResidentFromValue<L: Lease, T, M = ()>: Resident {
    /// Returns the memory layout for a resident with the given `data` and `meta` data.
    fn new_resident_layout(data: &T, meta: &M) -> Layout;

    /// Converts a `raw` pointer into a `Data` pointer for a resident with the
    /// given `data` and `meta` data.
    fn new_resident_ptr(raw: *mut u8, data: &T, meta: &M) -> *mut Self::Data;

    /// Initializes the `lease` with a resident from the given `data` and `meta` data.
    fn new_resident(lease: &mut L, data: T, meta: M);
}

/// A type that can initialize a new memory `Lease` with a `Resident` clone of a value.
pub trait ResidentFromClone<L: Lease, T: ?Sized, M = ()>: Resident {
    /// Returns the memory layout for a resident with a clone of the given `data` and `meta` data.
    fn new_resident_layout(data: &T, meta: &M) -> Layout;

    /// Converts a `raw` pointer into a `Data` pointer for a resident with a
    /// clone of the given `data` and `meta` data.
    fn new_resident_ptr(raw: *mut u8, data: &T, meta: &M) -> *mut Self::Data;

    /// Initializes the `lease` with a resident from a clone of the given `data` and `meta` data.
    fn new_resident(lease: &mut L, data: &T, meta: M);
}

/// A type that can initialize a new memory `Lease` with an unchecked `Resident` clone of a value.
pub trait ResidentFromCloneUnchecked<L: Lease, T: ?Sized, M = ()>: Resident {
    /// Returns the memory layout for a resident with a clone of the given `data` and `meta` data.
    fn new_resident_layout(data: &T, meta: &M) -> Layout;

    /// Converts a `raw` pointer into a `Data` pointer for a resident with a
    /// clone of the given `data` and `meta` data.
    fn new_resident_ptr(raw: *mut u8, data: &T, meta: &M) -> *mut Self::Data;

    /// Initializes the `lease` with a resident from a clone of the given `data` and `meta` data.
    fn new_resident(lease: &mut L, data: &T, meta: M);
}

/// A type that can initialize a new memory `Lease` with a `Resident` copy of a value.
pub trait ResidentFromCopy<L: Lease, T: ?Sized, M = ()>: Resident {
    /// Returns the memory layout for a resident with a copy of the given `data` and `meta` data.
    fn new_resident_layout(data: &T, meta: &M) -> Layout;

    /// Converts a `raw` pointer into a `Data` pointer for a resident with a
    /// copy of the given `data` and `meta` data.
    fn new_resident_ptr(raw: *mut u8, data: &T, meta: &M) -> *mut Self::Data;

    /// Initializes the `lease` with a resident from a copy of the given `data` and `meta` data.
    fn new_resident(lease: &mut L, data: &T, meta: M);
}

/// A type that can initialize a new memory `Lease` with an unchecked `Resident` copy of a value.
pub trait ResidentFromCopyUnchecked<L: Lease, T: ?Sized, M = ()>: Resident {
    /// Returns the memory layout for a resident with a copy of the given `data` and `meta` data.
    fn new_resident_layout(data: &T, meta: &M) -> Layout;

    /// Converts a `raw` pointer into a `Data` pointer for a resident with a
    /// copy of the given `data` and `meta` data.
    fn new_resident_ptr(raw: *mut u8, data: &T, meta: &M) -> *mut Self::Data;

    /// Initializes the `lease` with a resident from a copy of the given `data` and `meta` data.
    fn new_resident(lease: &mut L, data: &T, meta: M);
}

/// A type that can initialize a new memory `Lease` with an empty `Resident`.
pub trait ResidentFromEmpty<L: Lease, M = ()>: Resident {
    /// Returns the memory layout for an empty resident with the given `meta` data.
    fn new_resident_layout(meta: &M) -> Layout;

    /// Converts a `raw` pointer into a `Data` pointer for an empty resident
    /// with the given `meta` data.
    fn new_resident_ptr(raw: *mut u8, meta: &M) -> *mut Self::Data;

    /// Initializes the `lease` with an empty resident from the given `meta` data.
    fn new_resident(lease: &mut L, meta: M);
}

/// A type that can initialize a new memory `Lease` with a preallocated capacity,
/// and an empty `Resident`.
pub trait ResidentWithCapacity<L: Lease, M = ()>: Resident {
    /// Returns the memory layout for an empty resident with the given
    /// pre-allocated capacity and `meta` data.
    fn new_resident_layout(cap: usize, meta: &M) -> Result<Layout, LayoutError>;

    /// Converts a `raw` pointer into a `Data` pointer for an empty resident
    /// with the given preallocated capacity and `meta` data.
    fn new_resident_ptr(raw: *mut u8, cap: usize, meta: &M) -> *mut Self::Data;

    /// Initializes the `lease` with an empty resident from the given
    /// pre-allocated capacity and `meta` data.
    fn new_resident(lease: &mut L, cap: usize, meta: M);
}

/// An unwrappable `Resident` of a raw memory `Lease`.
pub trait ResidentUnwrap<L: Lease>: Resident {
    /// The type the resident unwraps to.
    type Target;

    /// Returns the unwrapped value for the resident of the given `lease`,
    /// leaving the `lease` in an uninitialized state.
    fn resident_unwrap(lease: &L) -> Self::Target;
}

/// An immutably dereferenceable `Resident` of a raw memory `Lease`.
pub trait ResidentDeref<L: Lease>: Resident {
    /// The type the resident dereferences to.
    type Target: ?Sized;

    /// Immutably dereferences the resident of the `lease`.
    fn resident_deref(lease: &L) -> &Self::Target;
}

/// A mutably dereferenceable resident of a raw memory `Lease`.
pub trait ResidentDerefMut<L: Lease>: ResidentDeref<L> {
    /// Mutably dereferences the resident of the `lease`.
    fn resident_deref_mut(lease: &mut L) -> &mut Self::Target;
}

/// An immutably referenceable `Resident` of a raw memory `Lease`.
pub trait ResidentAsRef<L: Lease, T: ?Sized>: Resident {
    /// Returns the resident of the `lease` as an immutable reference to type `T`.
    fn resident_as_ref(lease: &L) -> &T;
}

/// A mutably referenceable `Resident` of a raw memory `Lease`.
pub trait ResidentAsMut<L: Lease, T: ?Sized>: Resident {
    /// Returns the resident of the `lease` as a mutable reference to type `T`.
    fn resident_as_mut(lease: &mut L) -> &mut T;
}

/// An immutably indexed `Resident of a raw memory `Lease`.
pub trait ResidentIndex<L: Lease, Idx>: Resident {
    /// The type of values indexed by the resident for the parameterized `Idx` type.
    type Output: ?Sized;

    /// Returns an immutable reference to the value at the given `index` of the
    /// resident of the `lease`.
    fn resident_index(lease: &L, index: Idx) -> &Self::Output;
}

/// A mutably indexed `Resident of a raw memory `Lease`.
pub trait ResidentIndexMut<L: Lease, Idx>: ResidentIndex<L, Idx> {
    /// Returns a mutable reference to the value at the given `index` of the
    /// resident of the `lease`.
    fn resident_index_mut(lease: &mut L, index: Idx) -> &mut Self::Output;
}

/// A `+` operable `Resident` of a raw memory `Lease`.
pub trait ResidentAdd<L: Lease, Rhs = Self>: Resident {
    /// The resulting type of applying the `+` operator.
    type Output;

    /// Returns the addition of `rhs` to the resident of the `lease`.
    fn resident_add(lease: L, rhs: Rhs) -> Self::Output;
}

/// A `+=` operable `Resident` of a raw memory `Lease`.
pub trait ResidentAddAssign<L: Lease, Rhs = Self>: Resident {
    /// Adds `rhs` to the resident of the `lease`.
    fn resident_add_assign(lease: &mut L, rhs: Rhs);
}

/// A consuming iterable `Resident` of a raw memory `Lease`.
pub trait ResidentIntoIterator<L: Lease>: Resident {
    /// The type of element to iterator over.
    type Item;

    /// The type of iterator to return.
    type IntoIter: Iterator<Item=Self::Item>;

    /// Returns a new `Iterator` that consumes the elements of the `lease`.
    fn resident_into_iter(lease: L) -> Self::IntoIter;
}

/// An immutably iterable `Resident` of a raw memory `Lease`.
pub trait ResidentIntoRefIterator<'a, L: Lease>: Resident {
    /// The type of element to iterator over.
    type Item;

    /// The type of iterator to return.
    type IntoIter: Iterator<Item=Self::Item>;

    /// Returns a new `Iterator` that consumes the elements of the `lease`.
    fn resident_into_iter(lease: &'a L) -> Self::IntoIter;
}

/// A mutably iterable `Resident` of a raw memory `Lease`.
pub trait ResidentIntoMutIterator<'a, L: Lease>: Resident {
    /// The type of element to iterator over.
    type Item;

    /// The type of iterator to return.
    type IntoIter: Iterator<Item=Self::Item>;

    /// Returns a new `Iterator` that consumes the elements of the `lease`.
    fn resident_into_iter(lease: &'a mut L) -> Self::IntoIter;
}

/// A partially comparable `Resident` of a raw memory `Lease`.
pub trait ResidentPartialEq<L: Lease, T: ?Sized = L>: Resident {
    /// Returns `true` if the resident of the `lease` equals some `other` value.
    fn resident_eq(lease: &L, other: &T) -> bool;

    /// Returns `false` if the resident of the `lease` equals some `other` value.
    #[inline]
    fn resident_ne(lease: &L, other: &T) -> bool {
        !Self::resident_eq(lease, other)
    }
}

/// A comparable `Resident` of a raw memory `Lease`.
pub trait ResidentEq<L: Lease>: ResidentPartialEq<L> {
}

/// A partially ordered `Resident` of a raw memory `Lease`.
pub trait ResidentPartialOrd<L: Lease, T: ?Sized = L>: ResidentPartialEq<L, T> {
    /// Returns the ordering of the resident of the `lease` relative to some `other` value, if comparable.
    fn resident_partial_cmp(lease: &L, other: &T) -> Option<Ordering>;

    /// Returns `true` if the resident of the `lease` orders before some `other` value.
    #[inline]
    fn resident_lt(lease: &L, other: &T) -> bool {
        match Self::resident_partial_cmp(lease, other) {
            Some(Ordering::Less) => true,
            _ => false,
        }
    }

    /// Returns `true` if the resident of the `lease` orders before or the same as some `other` value.
    #[inline]
    fn resident_le(lease: &L, other: &T) -> bool {
        match Self::resident_partial_cmp(lease, other) {
            Some(Ordering::Less) | Some(Ordering::Equal) => true,
            _ => false,
        }
    }

    /// Returns `true` if the resident of the `lease` orders the same as or after some `other` value.
    #[inline]
    fn resident_ge(lease: &L, other: &T) -> bool {
        match Self::resident_partial_cmp(lease, other) {
            Some(Ordering::Greater) | Some(Ordering::Equal) => true,
            _ => false,
        }
    }

    /// Returns `true` if the resident of the `lease` orders after some `other` value.
    #[inline]
    fn resident_gt(lease: &L, other: &T) -> bool {
        match Self::resident_partial_cmp(lease, other) {
            Some(Ordering::Greater) => true,
            _ => false,
        }
    }
}

/// A totally ordered `Resident` of a raw memory `Lease`.
pub trait ResidentOrd<L: Lease>: ResidentEq<L> + ResidentPartialOrd<L> {
    /// Returns the relative ordering of the residents occupying the given leases.
    fn resident_cmp(lease: &L, other: &L) -> Ordering;

    #[inline]
    fn resident_lt(lease: &L, other: &L) -> bool {
        match Self::resident_cmp(lease, other) {
            Ordering::Less => true,
            _ => false,
        }
    }

    #[inline]
    fn resident_le(lease: &L, other: &L) -> bool {
        match Self::resident_cmp(lease, other) {
            Ordering::Less | Ordering::Equal => true,
            _ => false,
        }
    }

    #[inline]
    fn resident_ge(lease: &L, other: &L) -> bool {
        match Self::resident_cmp(lease, other) {
            Ordering::Greater | Ordering::Equal => true,
            _ => false,
        }
    }

    #[inline]
    fn resident_gt(lease: &L, other: &L) -> bool {
        match Self::resident_cmp(lease, other) {
            Ordering::Greater => true,
            _ => false,
        }
    }
}

/// A hashable `Resident` of a raw memory `Lease`.
pub trait ResidentHash<L: Lease>: Resident {
    /// Hashes the resident of the `lease`.
    fn resident_hash<H: Hasher>(lease: &L, state: &mut H);
}

/// A display-format-able `Resident` of a raw memory `Lease`.
pub trait ResidentDisplay<L: Lease>: Resident {
    /// Formats the resident of the `lease` for display.
    fn resident_fmt(lease: &L, f: &mut fmt::Formatter) -> fmt::Result;
}

/// A debug-format-able `Resident` of a raw memory `Lease`.
pub trait ResidentDebug<L: Lease>: Resident {
    /// Formats the resident of the `lease` for debugging.
    fn resident_fmt(lease: &L, f: &mut fmt::Formatter) -> fmt::Result;
}

/// A cloneable `Resident` of a raw memory `Lease`.
pub trait ResidentClone<L1: Lease, L2: Lease>: Resident {
    /// Returns the preferred memory layout of a clone destination for the
    /// resident of the `lease`.
    fn new_resident_layout(lease: &L1) -> Layout;

    /// Clones the resident of the `src` lease into the `dst` lease.
    fn resident_clone(src: &L1, dst: &mut L2) -> Result<(), HoldError>;
}

/// A relocatable `Resident` of a raw memory `Lease`.
pub trait ResidentStow<'b, L1: Lease, L2: Lease>: Resident {
    /// Returns the preferred memory layout of a move destination for the
    /// resident of the `lease`.
    fn new_resident_layout(lease: &L1) -> Layout;

    /// Relocates the resident from the `src` lease to the `dst` lease.
    /// Leaves the `src` lase in its original state on failure.
    unsafe fn resident_stow(src: &mut L1, dst: &mut L2, hold: &Hold<'b>) -> Result<(), HoldError>;

    /// Reverts the most recent `resident_stow` operation by relocating the
    /// resident of the `dst` lease back to the `src` lease.
    unsafe fn resident_unstow(src: &mut L1, dst: &mut L2);
}
