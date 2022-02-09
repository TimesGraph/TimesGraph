use core::cmp::Ordering;
use core::fmt::{self, Debug, Display, Formatter};
use core::hash::{Hash, Hasher};
use core::marker::PhantomData;
use core::mem;
use core::ptr;
use core::slice;
use core::str;
use crate::block::Layout;
use crate::alloc::{Hold, HoldError, Stow, TryClone};
use crate::lease::Lease;
use crate::resident::{Resident, ResidentFromValue, ResidentFromClone,
                      ResidentFromCopy, ResidentUnwrap, ResidentDeref,
                      ResidentDerefMut, ResidentAsRef, ResidentAsMut,
                      ResidentPartialEq, ResidentEq, ResidentPartialOrd,
                      ResidentOrd, ResidentHash, ResidentDisplay,
                      ResidentDebug, ResidentClone, ResidentStow};

/// A single value, residing in a memory `Lease`. A `Box` is a `Resident`
/// typeclass; it doesn't store any data in its internal structure. Rather,
/// `Box` implements an access pattern for memory blocks managed by a `Lease`.
/// A composing `Lease` type defines the memory allocation and ownership
/// semantics of the composed `Box` type.
///
/// `Lease` implementations commonly composed with a `Box` include:
/// - `Own<Box<T>>`: the exclusive owner of a relocatable value.
/// - `Mut<Box<T>>`: a mutably dereferenceable strong owner of an
///   unrelocatable, reference counted value.
/// - `Ref<Box<T>>`: an immutably dereferenceable strong owner of an
///   unrelocatable, reference counted value.
/// - `Hard<Box<T>>`: an undereferenceable strong owner of a relocatable,
///   reference counted value.
/// - `Soft<Box<T>>`: an undereferenceable weak owner of a relocatable,
///   reference counted value.
///
/// `Box` implements `LeaseDeref`, and `LeaseDerefMut`, which enables deref
/// coercions from `Own<Box<T>>` and `Mut<Box<T>>` to `&T` and `&mut T`,
/// and from `Ref<Box<T>>` to `&T`. This makes leased boxes "feel" like they
/// contain a value, even though `Box` is just a typeclass for accessing the
/// value stored in the composing memory `Lease`.
pub struct Box<T: ?Sized, M = ()> {
    /// Variant over T, with drop check.
    data_marker: PhantomData<T>,
    /// Variant over M, with drop check.
    meta_marker: PhantomData<M>,
}

unsafe impl<T: Send, M: Send> Send for Box<T, M> {
}

unsafe impl<T: Sync, M: Sync> Sync for Box<T, M> {
}

impl<T: ?Sized, M> Box<T, M> {
    #[inline]
    fn as_ref(lease: &impl Lease<Data=T, Meta=M>) -> &T {
        unsafe { &*lease.data() }
    }

    #[inline]
    fn as_mut(lease: &mut impl Lease<Data=T, Meta=M>) -> &mut T {
        unsafe { &mut *lease.data() }
    }
}

impl<T: ?Sized, M> Resident for Box<T, M> {
    type Data = T;

    type Meta = M;

    #[inline]
    unsafe fn resident_size(data: *mut T, _meta: *mut M) -> usize {
        mem::size_of_val(&*data)
    }

    #[inline]
    unsafe fn resident_drop(data: *mut T, _meta: *mut M) {
        ptr::drop_in_place(data)
    }
}

impl<L: Lease<Data=T, Meta=M>, T, M> ResidentFromValue<L, T, M> for Box<T, M> {
    #[inline]
    fn new_resident_layout(data: &T, _meta: &M) -> Layout {
        Layout::for_value(data)
    }

    #[inline]
    fn new_resident_ptr(raw: *mut u8, _data: &T, _meta: &M) -> *mut T {
        raw as *mut T
    }

    #[inline]
    fn new_resident(lease: &mut L, data: T, meta: M) {
        unsafe {
            ptr::write(lease.meta(), meta);
            ptr::write(lease.data(), data);
        }
    }
}

impl<L: Lease<Data=[T], Meta=M>, T: Clone, M> ResidentFromClone<L, [T], M> for Box<[T], M> {
    #[inline]
    fn new_resident_layout(data: &[T], _meta: &M) -> Layout {
        Layout::for_value(data)
    }

    #[inline]
    fn new_resident_ptr(raw: *mut u8, data: &[T], _meta: &M) -> *mut [T] {
        unsafe { slice::from_raw_parts_mut(raw as *mut T, data.len()) }
    }

    #[inline]
    fn new_resident(lease: &mut L, data: &[T], meta: M) {
        unsafe {
            ptr::write(lease.meta(), meta);
            (*lease.data()).clone_from_slice(data);
        }
    }
}

impl<L: Lease<Data=[T], Meta=M>, T: Copy, M> ResidentFromCopy<L, [T], M> for Box<[T], M> {
    #[inline]
    fn new_resident_layout(data: &[T], _meta: &M) -> Layout {
        Layout::for_value(data)
    }

    #[inline]
    fn new_resident_ptr(raw: *mut u8, data: &[T], _meta: &M) -> *mut [T] {
        unsafe { slice::from_raw_parts_mut(raw as *mut T, data.len()) }
    }

    #[inline]
    fn new_resident(lease: &mut L, data: &[T], meta: M) {
        unsafe {
            ptr::write(lease.meta(), meta);
            (*lease.data()).copy_from_slice(data);
        }
    }
}

impl<L: Lease<Data=str, Meta=M>, M> ResidentFromCopy<L, str, M> for Box<str, M> {
    #[inline]
    fn new_resident_layout(data: &str, _meta: &M) -> Layout {
        Layout::for_value(data)
    }

    #[inline]
    fn new_resident_ptr(raw: *mut u8, data: &str, _meta: &M) -> *mut str {
        unsafe { str::from_utf8_unchecked_mut(slice::from_raw_parts_mut(raw, data.len())) }
    }

    #[inline]
    fn new_resident(lease: &mut L, data: &str, meta: M) {
        unsafe {
            ptr::write(lease.meta(), meta);
            (*lease.data()).as_bytes_mut().copy_from_slice(data.as_bytes());
        }
    }
}

impl<L: Lease> ResidentUnwrap<L> for Box<L::Data, L::Meta> where L::Data: Sized {
    type Target = L::Data;

    #[inline]
    fn resident_unwrap(lease: &L) -> L::Data {
        unsafe { ptr::read(lease.data()) }
    }
}

impl<L: Lease> ResidentDeref<L> for Box<L::Data, L::Meta> {
    type Target = L::Data;

    #[inline]
    fn resident_deref(lease: &L) -> &L::Data {
        Box::as_ref(lease)
    }
}

impl<L: Lease> ResidentDerefMut<L> for Box<L::Data, L::Meta> {
    #[inline]
    fn resident_deref_mut(lease: &mut L) -> &mut L::Data {
        Box::as_mut(lease)
    }
}

impl<L: Lease> ResidentAsRef<L, L::Data> for Box<L::Data, L::Meta> {
    #[inline]
    fn resident_as_ref(lease: &L) -> &L::Data {
        Box::as_ref(lease)
    }
}

impl<L: Lease> ResidentAsMut<L, L::Data> for Box<L::Data, L::Meta> {
    #[inline]
    fn resident_as_mut(lease: &mut L) -> &mut L::Data {
        Box::as_mut(lease)
    }
}

impl<L: Lease> ResidentPartialEq<L> for Box<L::Data, L::Meta> where L::Data: PartialEq {
    #[inline]
    fn resident_eq(lease: &L, other: &L) -> bool {
        Box::as_ref(lease).eq(Box::as_ref(other))
    }

    #[inline]
    fn resident_ne(lease: &L, other: &L) -> bool {
        Box::as_ref(lease).ne(Box::as_ref(other))
    }
}

impl<L: Lease> ResidentEq<L> for Box<L::Data, L::Meta> where L::Data: Eq {
}

impl<L: Lease> ResidentPartialOrd<L> for Box<L::Data, L::Meta> where L::Data: PartialOrd {
    #[inline]
    fn resident_partial_cmp(lease: &L, other: &L) -> Option<Ordering> {
        Box::as_ref(lease).partial_cmp(Box::as_ref(other))
    }

    #[inline]
    fn resident_lt(lease: &L, other: &L) -> bool {
        Box::as_ref(lease).lt(Box::as_ref(other))
    }

    #[inline]
    fn resident_le(lease: &L, other: &L) -> bool {
        Box::as_ref(lease).le(Box::as_ref(other))
    }

    #[inline]
    fn resident_ge(lease: &L, other: &L) -> bool {
        Box::as_ref(lease).ge(Box::as_ref(other))
    }

    #[inline]
    fn resident_gt(lease: &L, other: &L) -> bool {
        Box::as_ref(lease).gt(Box::as_ref(other))
    }
}

impl<L: Lease> ResidentOrd<L> for Box<L::Data, L::Meta> where L::Data: Ord {
    #[inline]
    fn resident_cmp(lease: &L, other: &L) -> Ordering {
        Box::as_ref(lease).cmp(Box::as_ref(other))
    }
}

impl<L: Lease> ResidentHash<L> for Box<L::Data, L::Meta> where L::Data: Hash {
    #[inline]
    fn resident_hash<H: Hasher>(lease: &L, state: &mut H) {
        Box::as_ref(lease).hash(state)
    }
}

impl<L: Lease> ResidentDisplay<L> for Box<L::Data, L::Meta> where L::Data: Display {
    #[inline]
    fn resident_fmt(lease: &L, f: &mut Formatter) -> fmt::Result {
        Display::fmt(Box::as_ref(lease), f)
    }
}

impl<L: Lease> ResidentDebug<L> for Box<L::Data, L::Meta> where L::Data: Debug {
    #[inline]
    fn resident_fmt(lease: &L, f: &mut Formatter) -> fmt::Result {
        Debug::fmt(Box::as_ref(lease), f)
    }
}

impl<L1, L2, T, M> ResidentClone<L1, L2> for Box<T, M>
    where L1: Lease<Data=T, Meta=M>,
          L2: Lease<Data=T, Meta=M>,
          T: ?Sized + TryClone,
          M: TryClone,
{
    #[inline]
    fn new_resident_layout(lease: &L1) -> Layout {
        Layout::for_value(Box::as_ref(lease))
    }

    #[inline]
    fn resident_clone(src: &L1, dst: &mut L2) -> Result<(), HoldError> {
        unsafe {
            ptr::write(dst.meta(), (*src.meta()).try_clone()?);
            ptr::write(dst.data(), (*src.data()).try_clone()?);
            Ok(())
        }
    }
}

impl<L1, L2, T, M> ResidentClone<L1, L2> for Box<[T], M>
    where L1: Lease<Data=[T], Meta=M>,
          L2: Lease<Data=[T], Meta=M>,
          T: TryClone,
          M: TryClone,
{
    #[inline]
    fn new_resident_layout(lease: &L1) -> Layout {
        Layout::for_value(Box::as_ref(lease))
    }

    #[inline]
    fn resident_clone(src: &L1, dst: &mut L2) -> Result<(), HoldError> {
        unsafe {
            ptr::write(dst.meta(), (*src.meta()).try_clone()?);
            let src_data = src.data();
            let dst_data = dst.data();
            let len = (*src_data).len();
            let mut src_data = src_data as *const T;
            let mut dst_data = dst_data as *mut T;
            let mut i = 0;
            while i < len {
                let src_elem = match (*dst_data).try_clone() {
                    Ok(elem) => elem,
                    Err(error) => {
                        while i > 0 {
                            dst_data = dst_data.wrapping_sub(1);
                            i = i.wrapping_sub(1);
                            ptr::drop_in_place(dst_data);
                        }
                        return Err(error);
                    }
                };
                ptr::write(dst_data, src_elem);
                src_data = src_data.wrapping_add(1);
                dst_data = dst_data.wrapping_add(1);
                i = i.wrapping_add(1);
            }
            Ok(())
        }
    }
}

impl<L1, L2, M> ResidentClone<L1, L2> for Box<str, L1::Meta>
    where L1: Lease<Data=str, Meta=M>,
          L2: Lease<Data=str, Meta=M>,
          M: TryClone,
{
    #[inline]
    fn new_resident_layout(lease: &L1) -> Layout {
        Layout::for_value(Box::as_ref(lease))
    }

    #[inline]
    fn resident_clone(src: &L1, dst: &mut L2) -> Result<(), HoldError> {
        unsafe {
            ptr::write(dst.meta(), (*src.meta()).try_clone()?);
            let src_data = src.data();
            let dst_data = dst.data();
            let len = (*src_data).len();
            ptr::copy_nonoverlapping(src_data as *const u8, dst_data as *mut u8, len);
            Ok(())
        }
    }
}

impl<'b, L1: Lease, L2: Lease> ResidentStow<'b, L1, L2> for Box<L1::Data, L1::Meta>
    where L1::Data: Stow<'b, L2::Data>,
          L1::Meta: Stow<'b, L2::Meta>,
{
    #[inline]
    fn new_resident_layout(lease: &L1) -> Layout {
        Layout::for_value(Box::as_ref(lease))
    }

    #[inline]
    unsafe fn resident_stow(src: &mut L1, dst: &mut L2, hold: &Hold<'b>) -> Result<(), HoldError> {
        if let err @ Err(_) = L1::Meta::stow(src.meta(), dst.meta(), hold) {
            return err;
        }
        if let err @ Err(_) = L1::Data::stow(src.data(), dst.data(), hold) {
            L1::Meta::unstow(src.meta(), dst.meta());
            return err;
        }
        Ok(())
    }

    #[inline]
    unsafe fn resident_unstow(src: &mut L1, dst: &mut L2) {
        L1::Data::unstow(src.data(), dst.data());
        L1::Meta::unstow(src.meta(), dst.meta());
    }
}
