use core::cmp::Ordering;
use core::fmt::{self, Debug, Display, Formatter, Write};
use core::hash::{Hash, Hasher};
use core::intrinsics::assume;
use core::marker::PhantomData;
use core::mem;
use core::ops::{Deref, DerefMut, Index, IndexMut};
use core::ops::{Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive};
use core::ptr;
use core::slice;
use core::str;
use crate::block::{Layout, LayoutError};
use crate::alloc::{Hold, HoldError, TryClone};
use crate::lease::{Lease, DynamicLease};
use crate::resident::{Resident, ResidentFromCopy, ResidentFromEmpty,
                      ResidentWithCapacity, ResidentDeref, ResidentDerefMut,
                      ResidentAsRef, ResidentIndex, ResidentIndexMut, ResidentAdd,
                      ResidentAddAssign, ResidentPartialEq, ResidentEq,
                      ResidentPartialOrd, ResidentOrd, ResidentHash, ResidentDisplay,
                      ResidentDebug, ResidentClone, ResidentStow, BufHeader, BufLease};

/// A resizeable array of Unicode code points, residing in a memory `Lease`.
/// A `String` is a `Resident` typeclass; it doesn't store any data in its
/// internal structure. Rather, `String` implements an access pattern for
/// memory blocks managed by a `Lease`. A composing `Lease` type defines the
/// memory allocation and ownership semantics of the composed `String` type.
///
/// `Lease` implementations commonly composed with a `String` include:
/// - `Own<String>`: the exclusive owner of a relocatable, resizeable string.
/// - `Mut<String>`: a mutably dereferenceable strong owner of an unrelocatable,
///   resizeable string.
/// - `Ref<String>`: an immutably dereferenceable strong owner of an unrelocatable,
///   resizeable string.
/// - `Hard<String>`: an undereferenceable strong owner of a relocatable string.
/// - `Soft<String>`: an undereferenceable weak owner of a relocatable string.
///
/// `String` implements `LeaseDeref`, and `LeaseDerefMut`, which enables deref
/// coercions from `Own<String>`, `Mut<String>`, and `Ref<String>` to
/// `&StringLease<L>`. `StringLease` in turn deref coerces to `&str` and
/// `&mut str`. This makes leased strings "feel" like they contain a `str`,
/// even though `String` is just a typeclass for accessing the `str` stored in
/// the composing memory `Lease`. Most string operations are defined on
/// `StringLease`; `String` can't do much on its own since it doesn't actually
/// contain any data.
pub struct String<M = ()> {
    /// Variant over BufHeader<M>, with drop check.
    meta_marker: PhantomData<BufHeader<M>>,
}

pub struct StringLease<L: Lease<Data=u8, Meta=BufHeader<M>>, M = ()> {
    /// Memory `Lease` in which the `String` resides.
    lease: L,
}

unsafe impl<M: Send> Send for String<M> {
}

unsafe impl<M: Sync> Sync for String<M> {
}

impl<M> String<M> {
    #[inline]
    fn header(lease: &impl Lease<Data=u8, Meta=BufHeader<M>>) -> &BufHeader<M> {
        unsafe { &*lease.meta() }
    }

    #[inline]
    fn header_mut(lease: &mut impl Lease<Data=u8, Meta=BufHeader<M>>) -> &mut BufHeader<M> {
        unsafe { &mut *lease.meta() }
    }

    #[inline]
    fn as_slice(lease: &impl Lease<Data=u8, Meta=BufHeader<M>>) -> &[u8] {
        unsafe {
            let data = lease.data();
            assume(!data.is_null());
            slice::from_raw_parts(data, String::header(lease).len)
        }
    }

    #[inline]
    fn as_mut_slice(lease: &mut impl Lease<Data=u8, Meta=BufHeader<M>>) -> &mut [u8] {
        unsafe {
            let data = lease.data();
            assume(!data.is_null());
            slice::from_raw_parts_mut(data, String::header(lease).len)
        }
    }

    #[inline]
    fn as_str(lease: &impl Lease<Data=u8, Meta=BufHeader<M>>) -> &str {
        unsafe {
            str::from_utf8_unchecked(String::as_slice(lease))
        }
    }

    #[inline]
    fn as_mut_str(lease: &mut impl Lease<Data=u8, Meta=BufHeader<M>>) -> &mut str {
        unsafe {
            str::from_utf8_unchecked_mut(String::as_mut_slice(lease))
        }
    }
}

impl<M> Resident for String<M> {
    type Data = u8;

    type Meta = BufHeader<M>;

    #[inline]
    unsafe fn resident_size(_data: *mut u8, meta: *mut BufHeader<M>) -> usize {
        (*meta).cap
    }

    #[inline]
    unsafe fn resident_drop(_data: *mut u8, _meta: *mut BufHeader<M>) {
        // nop
    }
}

impl<L: Lease<Data=u8, Meta=BufHeader<M>>, M> ResidentFromCopy<L, str, M> for String<M> {
    #[inline]
    fn new_resident_layout(data: &str, _meta: &M) -> Layout {
        Layout::for_value(data)
    }

    #[inline]
    fn new_resident_ptr(raw: *mut u8, _data: &str, _meta: &M) -> *mut u8 {
        raw
    }

    #[inline]
    fn new_resident(lease: &mut L, data: &str, meta: M) {
        unsafe {
            let len = data.len();
            ptr::write(lease.meta(), BufHeader {
                len: len,
                cap: len,
                meta: meta,
            });
            ptr::copy_nonoverlapping(data.as_ptr(), lease.data(), len);
        }
    }
}

impl<L: Lease<Data=u8, Meta=BufHeader<M>>, M> ResidentFromEmpty<L, M> for String<M> {
    #[inline]
    fn new_resident_layout(_meta: &M) -> Layout {
        Layout::empty()
    }

    #[inline]
    fn new_resident_ptr(raw: *mut u8, _meta: &M) -> *mut u8 {
        raw
    }

    #[inline]
    fn new_resident(lease: &mut L, meta: M) {
        unsafe {
            ptr::write(lease.meta(), BufHeader {
                len: 0,
                cap: 0,
                meta: meta,
            });
        }
    }
}

impl<L: Lease<Data=u8, Meta=BufHeader<M>>, M> ResidentWithCapacity<L, M> for String<M> {
    #[inline]
    fn new_resident_layout(cap: usize, _meta: &M) -> Result<Layout, LayoutError> {
        Layout::for_array::<u8>(cap)
    }

    #[inline]
    fn new_resident_ptr(raw: *mut u8, _cap: usize, _meta: &M) -> *mut u8 {
        raw
    }

    #[inline]
    fn new_resident(lease: &mut L, cap: usize, meta: M) {
        unsafe {
            ptr::write(lease.meta(), BufHeader {
                len: 0,
                cap: cap,
                meta: meta,
            });
        }
    }
}

impl<L: Lease<Data=u8, Meta=BufHeader<M>>, M> ResidentDeref<L> for String<M> {
    type Target = StringLease<L, M>;

    #[inline]
    fn resident_deref(lease: &L) -> &StringLease<L, M> {
        unsafe { mem::transmute::<&L, &StringLease<L, M>>(lease) }
    }
}

impl<L: Lease<Data=u8, Meta=BufHeader<M>>, M> ResidentDerefMut<L> for String<M> {
    #[inline]
    fn resident_deref_mut(lease: &mut L) -> &mut StringLease<L, M> {
        unsafe { mem::transmute::<&mut L, &mut StringLease<L, M>>(lease) }
    }
}

impl<L: Lease<Data=u8, Meta=BufHeader<M>>, M> ResidentAsRef<L, str> for String<M> {
    #[inline]
    fn resident_as_ref(lease: &L) -> &str {
        String::as_str(lease)
    }
}

impl<L: Lease<Data=u8, Meta=BufHeader<M>>, M> ResidentAsRef<L, [u8]> for String<M> {
    #[inline]
    fn resident_as_ref(lease: &L) -> &[u8] {
        String::as_slice(lease)
    }
}

impl<L: Lease<Data=u8, Meta=BufHeader<M>>, M> ResidentIndex<L, Range<usize>> for String<M> {
    type Output = str;

    #[inline]
    fn resident_index(lease: &L, index: Range<usize>) -> &str {
        String::as_str(lease).index(index)
    }
}

impl<L: Lease<Data=u8, Meta=BufHeader<M>>, M> ResidentIndex<L, RangeFrom<usize>> for String<M> {
    type Output = str;

    #[inline]
    fn resident_index(lease: &L, index: RangeFrom<usize>) -> &str {
        String::as_str(lease).index(index)
    }
}

impl<L: Lease<Data=u8, Meta=BufHeader<M>>, M> ResidentIndex<L, RangeFull> for String<M> {
    type Output = str;

    #[inline]
    fn resident_index(lease: &L, index: RangeFull) -> &str {
        String::as_str(lease).index(index)
    }
}

impl<L: Lease<Data=u8, Meta=BufHeader<M>>, M> ResidentIndex<L, RangeInclusive<usize>> for String<M> {
    type Output = str;

    #[inline]
    fn resident_index(lease: &L, index: RangeInclusive<usize>) -> &str {
        String::as_str(lease).index(index)
    }
}

impl<L: Lease<Data=u8, Meta=BufHeader<M>>, M> ResidentIndex<L, RangeTo<usize>> for String<M> {
    type Output = str;

    #[inline]
    fn resident_index(lease: &L, index: RangeTo<usize>) -> &str {
        String::as_str(lease).index(index)
    }
}

impl<L: Lease<Data=u8, Meta=BufHeader<M>>, M> ResidentIndex<L, RangeToInclusive<usize>> for String<M> {
    type Output = str;

    #[inline]
    fn resident_index(lease: &L, index: RangeToInclusive<usize>) -> &str {
        String::as_str(lease).index(index)
    }
}

impl<L: Lease<Data=u8, Meta=BufHeader<M>>, M> ResidentIndexMut<L, Range<usize>> for String<M> {
    #[inline]
    fn resident_index_mut(lease: &mut L, index: Range<usize>) -> &mut str {
        String::as_mut_str(lease).index_mut(index)
    }
}

impl<L: Lease<Data=u8, Meta=BufHeader<M>>, M> ResidentIndexMut<L, RangeFrom<usize>> for String<M> {
    #[inline]
    fn resident_index_mut(lease: &mut L, index: RangeFrom<usize>) -> &mut str {
        String::as_mut_str(lease).index_mut(index)
    }
}

impl<L: Lease<Data=u8, Meta=BufHeader<M>>, M> ResidentIndexMut<L, RangeFull> for String<M> {
    #[inline]
    fn resident_index_mut(lease: &mut L, index: RangeFull) -> &mut str {
        String::as_mut_str(lease).index_mut(index)
    }
}

impl<L: Lease<Data=u8, Meta=BufHeader<M>>, M> ResidentIndexMut<L, RangeInclusive<usize>> for String<M> {
    #[inline]
    fn resident_index_mut(lease: &mut L, index: RangeInclusive<usize>) -> &mut str {
        String::as_mut_str(lease).index_mut(index)
    }
}

impl<L: Lease<Data=u8, Meta=BufHeader<M>>, M> ResidentIndexMut<L, RangeTo<usize>> for String<M> {
    #[inline]
    fn resident_index_mut(lease: &mut L, index: RangeTo<usize>) -> &mut str {
        String::as_mut_str(lease).index_mut(index)
    }
}

impl<L: Lease<Data=u8, Meta=BufHeader<M>>, M> ResidentIndexMut<L, RangeToInclusive<usize>> for String<M> {
    #[inline]
    fn resident_index_mut(lease: &mut L, index: RangeToInclusive<usize>) -> &mut str {
        String::as_mut_str(lease).index_mut(index)
    }
}

impl<'a, 'b, L: DynamicLease<'a, Data=u8, Meta=BufHeader<M>>, M> ResidentAdd<L, &'b str> for String<M> {
    type Output = L;

    #[inline]
    fn resident_add(mut lease: L, rhs: &'b str) -> L {
        String::resident_deref_mut(&mut lease).push_str(rhs);
        lease
    }
}

impl<'a, 'b, L: DynamicLease<'a, Data=u8, Meta=BufHeader<M>>, M> ResidentAddAssign<L, &'b str> for String<M> {
    #[inline]
    fn resident_add_assign(lease: &mut L, rhs: &'b str) {
        String::resident_deref_mut(lease).push_str(rhs);
    }
}

impl<L: Lease<Data=u8, Meta=BufHeader<M>>, M> ResidentPartialEq<L> for String<M> {
    #[inline]
    fn resident_eq(lease: &L, other: &L) -> bool {
        String::as_str(lease).eq(String::as_str(other))
    }

    #[inline]
    fn resident_ne(lease: &L, other: &L) -> bool {
        String::as_str(lease).ne(String::as_str(other))
    }
}

impl<L: Lease<Data=u8, Meta=BufHeader<M>>, M> ResidentEq<L> for String<M> {
}

impl<L: Lease<Data=u8, Meta=BufHeader<M>>, M> ResidentPartialOrd<L> for String<M> {
    #[inline]
    fn resident_partial_cmp(lease: &L, other: &L) -> Option<Ordering> {
        String::as_str(lease).partial_cmp(String::as_str(other))
    }

    #[inline]
    fn resident_lt(lease: &L, other: &L) -> bool {
        String::as_str(lease).lt(String::as_str(other))
    }

    #[inline]
    fn resident_le(lease: &L, other: &L) -> bool {
        String::as_str(lease).le(String::as_str(other))
    }

    #[inline]
    fn resident_ge(lease: &L, other: &L) -> bool {
        String::as_str(lease).ge(String::as_str(other))
    }

    #[inline]
    fn resident_gt(lease: &L, other: &L) -> bool {
        String::as_str(lease).gt(String::as_str(other))
    }
}

impl<L: Lease<Data=u8, Meta=BufHeader<M>>, M> ResidentOrd<L> for String<M> {
    #[inline]
    fn resident_cmp(lease: &L, other: &L) -> Ordering {
        String::as_str(lease).cmp(String::as_str(other))
    }
}

impl<L: Lease<Data=u8, Meta=BufHeader<M>>, M> ResidentHash<L> for String<M> {
    #[inline]
    fn resident_hash<H: Hasher>(lease: &L, state: &mut H) {
        String::as_str(lease).hash(state)
    }
}

impl<L: Lease<Data=u8, Meta=BufHeader<M>>, M> ResidentDisplay<L> for String<M> {
    #[inline]
    fn resident_fmt(lease: &L, f: &mut Formatter) -> fmt::Result {
        Display::fmt(String::as_str(lease), f)
    }
}

impl<L: Lease<Data=u8, Meta=BufHeader<M>>, M> ResidentDebug<L> for String<M> {
    #[inline]
    fn resident_fmt(lease: &L, f: &mut Formatter) -> fmt::Result {
        Debug::fmt(String::as_str(lease), f)
    }
}

impl<L1, L2, M> ResidentClone<L1, L2> for String<M>
    where L1: Lease<Data=u8, Meta=BufHeader<M>>,
          L2: Lease<Data=u8, Meta=BufHeader<M>>,
          M: TryClone,
{
    #[inline]
    fn new_resident_layout(lease: &L1) -> Layout {
        unsafe { Layout::for_array_unchecked::<u8>(String::header(lease).len) }
    }

    #[inline]
    fn resident_clone(src: &L1, dst: &mut L2) -> Result<(), HoldError> {
        unsafe {
            let src_meta = src.meta();
            let dst_meta = dst.meta();
            ptr::write(dst_meta, (*src_meta).try_clone()?);
            let len = (*dst_meta).len;
            (*dst_meta).cap = len;
            ptr::copy_nonoverlapping(src.data(), dst.data(), len);
            Ok(())
        }
    }
}

impl<'b, L1, L2, M> ResidentStow<'b, L1, L2> for String<M>
    where L1: Lease<Data=u8, Meta=BufHeader<M>>,
          L2: Lease<Data=u8, Meta=BufHeader<M>>,
          M: TryClone,
{
    #[inline]
    fn new_resident_layout(lease: &L1) -> Layout {
        unsafe { Layout::for_array_unchecked::<u8>(String::header(lease).len) }
    }

    #[inline]
    unsafe fn resident_stow(src: &mut L1, dst: &mut L2, _hold: &Hold<'b>) -> Result<(), HoldError> {
        let src_meta = src.meta();
        let dst_meta = dst.meta();
        ptr::copy_nonoverlapping(src_meta, dst_meta, 1);
        let len = (*dst_meta).len;
        (*dst_meta).cap = len;
        ptr::copy_nonoverlapping(src.data(), dst.data(), len);
        Ok(())
    }

    #[inline]
    unsafe fn resident_unstow(_src: &mut L1, _dst: &mut L2) {
        unimplemented!();
    }
}

impl<L: Lease<Data=u8, Meta=BufHeader<M>>, M> StringLease<L, M> {
    #[inline]
    fn header(&self) -> &BufHeader<M> {
        String::header(&self.lease)
    }

    #[inline]
    fn header_mut(&mut self) -> &mut BufHeader<M> {
        String::header_mut(&mut self.lease)
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
       self.header().len == 0
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.header().len
    }

    #[inline]
    pub unsafe fn set_len(&mut self, new_len: usize) {
        self.header_mut().len = new_len;
    }

    #[inline]
    pub fn cap(&self) -> usize {
        self.header().cap
    }

    #[inline]
    pub fn meta(&self) -> &M {
        &self.header().meta
    }

    #[inline]
    pub fn meta_mut(&mut self) -> &mut M {
        &mut self.header_mut().meta
    }

    #[inline]
    pub fn as_ptr(&self) -> *const u8 {
        self.lease.data()
    }

    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.lease.data()
    }

    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        String::as_slice(&self.lease)
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        String::as_str(&self.lease)
    }

    #[inline]
    pub fn as_mut_str(&mut self) -> &mut str {
        String::as_mut_str(&mut self.lease)
    }

    pub fn pop(&mut self) -> Option<char> {
        unsafe {
            let c = self.as_str().chars().rev().next()?;
            let header = self.lease.meta();
            (*header).len = (*header).len.wrapping_sub(c.len_utf8());
            Some(c)
        }
    }

    pub fn remove(&mut self, index: usize) -> char {
        unsafe {
            let c = self.as_str()[index..].chars().next().unwrap();
            let n = c.len_utf8();
            let next = index.wrapping_add(n);
            let header = self.lease.meta();
            let len = (*header).len;
            let data = self.lease.data();
            ptr::copy(data.wrapping_add(next),
                      data.wrapping_add(index),
                      len.wrapping_sub(next));
            (*header).len = len.wrapping_sub(n);
            c
        }
    }

    pub fn clear(&mut self) {
        self.header_mut().len = 0;
    }
}

impl<'a, L: DynamicLease<'a, Data=u8, Meta=BufHeader<M>>, M> StringLease<L, M> {
    pub fn try_reserve(&mut self, ext: usize) -> Result<(), HoldError> {
        let buf = unsafe { mem::transmute::<&mut StringLease<L, M>, &mut BufLease<L, u8, M>>(self) };
        buf.try_reserve(ext)
    }

    pub fn reserve(&mut self, ext: usize) {
        self.try_reserve(ext).unwrap();
    }

    pub fn try_reserve_exact(&mut self, ext: usize) -> Result<(), HoldError> {
        let buf = unsafe { mem::transmute::<&mut StringLease<L, M>, &mut BufLease<L, u8, M>>(self) };
        buf.try_reserve_exact(ext)
    }

    pub fn reserve_exact(&mut self, ext: usize) {
        self.try_reserve_exact(ext).unwrap();
    }

    pub fn try_reserve_in_place(&mut self, ext: usize) -> Result<(), HoldError> {
        let buf = unsafe { mem::transmute::<&mut StringLease<L, M>, &mut BufLease<L, u8, M>>(self) };
        buf.try_reserve_in_place(ext)
    }

    pub fn try_reserve_in_place_exact(&mut self, ext: usize) -> Result<(), HoldError> {
        let buf = unsafe { mem::transmute::<&mut StringLease<L, M>, &mut BufLease<L, u8, M>>(self) };
        buf.try_reserve_in_place_exact(ext)
    }

    pub fn try_push(&mut self, c: char) -> Result<(), HoldError> {
        unsafe {
            let mut bytes = [0u8; 4];
            let n = c.encode_utf8(&mut bytes).len();
            self.try_reserve(n)?;
            let header = self.lease.meta();
            let len = (*header).len;
            let data = self.lease.data().wrapping_add(len);
            ptr::copy_nonoverlapping(bytes.as_ptr(), data, n);
            (*header).len = len.wrapping_add(n);
            Ok(())
        }
    }

    pub fn push(&mut self, c: char) {
        self.try_push(c).unwrap();
    }

    pub fn try_push_str(&mut self, s: &str) -> Result<(), HoldError> {
        unsafe {
            let n = s.len();
            self.try_reserve(n)?;
            let header = self.lease.meta();
            let len = (*header).len;
            let data = self.lease.data().wrapping_add(len);
            ptr::copy_nonoverlapping(s.as_ptr(), data, n);
            (*header).len = len.wrapping_add(n);
            Ok(())
        }
    }

    pub fn push_str(&mut self, s: &str) {
        self.try_push_str(s).unwrap();
    }

    pub fn try_insert(&mut self, index: usize, c: char) -> Result<(), HoldError> {
        unsafe {
            assert!(self.as_str().is_char_boundary(index));
            let mut slice = [0u8; 4];
            let slice = c.encode_utf8(&mut slice).as_bytes();
            self.try_insert_slice(index, slice)
        }
    }

    pub fn insert(&mut self, index: usize, c: char) {
        self.try_insert(index, c).unwrap();
    }

    pub fn try_insert_str(&mut self, index: usize, s: &str) -> Result<(), HoldError> {
        unsafe {
            assert!(self.as_str().is_char_boundary(index));
            self.try_insert_slice(index, s.as_bytes())
        }
    }

    pub fn insert_str(&mut self, index: usize, s: &str) {
        self.try_insert_str(index, s).unwrap();
    }

    pub unsafe fn try_insert_slice(&mut self, index: usize, slice: &[u8]) -> Result<(), HoldError> {
        let n = slice.len();
        self.try_reserve(n)?;
        let header = self.lease.meta();
        let len = (*header).len;
        let data = self.lease.data();
        ptr::copy(data.wrapping_add(index),
                  data.wrapping_add(index.wrapping_add(n)),
                  len.wrapping_sub(index));
        ptr::copy(slice.as_ptr(),
                  data.wrapping_add(index),
                  n);
        (*header).len = len.wrapping_add(n);
        Ok(())
    }

    pub unsafe fn insert_slice(&mut self, index: usize, slice: &[u8]) {
        self.try_insert_slice(index, slice).unwrap();
    }
}

impl<L: Lease<Data=u8, Meta=BufHeader<M>>, M> Deref for StringLease<L, M> {
    type Target = str;

    #[inline]
    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl<L: Lease<Data=u8, Meta=BufHeader<M>>, M> DerefMut for StringLease<L, M> {
    #[inline]
    fn deref_mut(&mut self) -> &mut str {
        self.as_mut_str()
    }
}

impl<'a, L: DynamicLease<'a, Data=u8, Meta=BufHeader<M>>, M> Write for StringLease<L, M> {
    #[inline]
    fn write_char(&mut self, c: char) -> fmt::Result {
        match self.try_push(c) {
            Ok(_) => Ok(()),
            Err(_) => Err(fmt::Error),
        }
    }

    #[inline]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        match self.try_push_str(s) {
            Ok(_) => Ok(()),
            Err(_) => Err(fmt::Error),
        }
    }
}
