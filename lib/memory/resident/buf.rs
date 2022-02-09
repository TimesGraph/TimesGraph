use core::cmp::Ordering;
use core::fmt::{self, Debug, Formatter};
use core::hash::{Hash, Hasher};
use core::intrinsics::{arith_offset, assume};
use core::iter::{FusedIterator, TrustedLen};
use core::marker::PhantomData;
use core::mem;
use core::ops::{Bound, Deref, DerefMut, Index, IndexMut, RangeBounds};
use core::ptr;
use core::slice::{self, SliceIndex};
use crate::block::{Layout, LayoutError, ZSP};
use crate::alloc::{Hold, Holder, HoldError, TryClone, CloneIntoHold};
use crate::lease::{Lease, DynamicLease};
use crate::resident::{Resident, ResidentFromClone, ResidentFromCopy,
                      ResidentFromEmpty, ResidentWithCapacity, ResidentDeref,
                      ResidentDerefMut, ResidentAsRef, ResidentAsMut, ResidentIndex,
                      ResidentIndexMut, ResidentIntoIterator,
                      ResidentIntoRefIterator, ResidentIntoMutIterator,
                      ResidentPartialEq, ResidentEq, ResidentPartialOrd, ResidentOrd,
                      ResidentHash, ResidentDebug, ResidentClone, ResidentStow};

/// A resizeable array of values, residing in a memory `Lease`. A `Buf` is a
/// `Resident` typeclass; it doesn't store any data in its internal structure.
/// Rather, `Buf` implements an access pattern for memory blocks managed by a
/// `Lease`. A composing `Lease` type defines the memory allocation and
/// ownership semantics of the composed `Buf` type.
///
/// `Lease` implementations commonly composed with a `Buf` include:
/// - `Own<Buf<T>>`: the exclusive owner of a relocatable, resizeable array.
/// - `Mut<Buf<T>>`: a mutably dereferenceable strong owner of an
///   unrelocatable, resizeable array.
/// - `Ref<Buf<T>>`: an immutably dereferenceable strong owner of an
///   unrelocatable, resizeable array.
/// - `Hard<Buf<T>>`: an undereferenceable strong owner of a relocatable array.
/// - `Soft<Buf<T>>`: an undereferenceable weak owner of a relocatable array.
///
/// `Buf` implements `LeaseDeref`, and `LeaseDerefMut`, which enables deref
/// coercions from `Own<Buf<T>>`, `Mut<Buf<T>>`, and `Ref<Buf<T>>` to
/// `&BufLease<L, T>`. `BufLease` in turn deref coerces to `&[T]` and
/// `&mut [T]`. This makes leased bufs "feel" like they contain a slice, even
/// though `Buf` is just a typeclass for accessing the slice stored in the
/// composing memory `Lease`. Most buffer operations are defined on `BufLease`;
/// `Buf` can't do much on its own since it doesn't actually contain any data.
pub struct Buf<T, M = ()> {
    /// Variant over T, with drop check.
    data_marker: PhantomData<T>,
    /// Variant over BufHeader<M>, with drop check.
    meta_marker: PhantomData<BufHeader<M>>,
}

/// The `Meta` structure associated with every `Lease` in which a `Buf` resides.
#[derive(Clone, Copy)]
pub struct BufHeader<M = ()> {
    /// Number of elements contained in the `Buf`.
    pub len: usize,
    /// Number of slots allocated in the `Lease`.
    pub cap: usize,
    /// User-provided metadata.
    pub meta: M,
}

pub struct BufLease<L: Lease<Data=T, Meta=BufHeader<M>>, T, M = ()> {
    /// Memory `Lease` in which the `Buf` resides.
    lease: L,
}

pub struct BufIter<L: Lease<Data=T, Meta=BufHeader<M>>, T, M = ()> {
    /// Memory `Lease` in which the `Buf` resides.
    lease: L,
    /// Pointer to the next element to iterate over.
    head: *const T,
    /// Pointer to the address after the last element to iterate over.
    foot: *const T,
}

pub struct BufDrain<'a, L: Lease<Data=T, Meta=BufHeader<M>> + 'a, T: 'a, M: 'a = ()> {
    /// Buffer to drain.
    buf: &'a mut BufLease<L, T, M>,
    /// Inclusive lower bound index of the drained slice.
    lower: usize,
    /// Exclusive upper bound index of the drained slice.
    upper: usize,
    /// Pointer to the next element to iterate over.
    head: *const T,
    /// Pointer to the address after the last element to iterate over.
    foot: *const T,
}

unsafe impl<T: Send, M: Send> Send for Buf<T, M> {
}

unsafe impl<T: Sync, M: Sync> Sync for Buf<T, M> {
}

impl<T, M> Buf<T, M> {
    #[inline]
    fn header(lease: &impl Lease<Data=T, Meta=BufHeader<M>>) -> &BufHeader<M> {
        unsafe { &*lease.meta() }
    }

    #[inline]
    fn header_mut(lease: &mut impl Lease<Data=T, Meta=BufHeader<M>>) -> &mut BufHeader<M> {
        unsafe { &mut *lease.meta() }
    }

    #[inline]
    fn as_slice(lease: &impl Lease<Data=T, Meta=BufHeader<M>>) -> &[T] {
        unsafe {
            let data = lease.data();
            assume(!data.is_null());
            slice::from_raw_parts(data, Buf::header(lease).len)
        }
    }

    #[inline]
    fn as_mut_slice(lease: &mut impl Lease<Data=T, Meta=BufHeader<M>>) -> &mut [T] {
        unsafe {
            let data = lease.data();
            assume(!data.is_null());
            slice::from_raw_parts_mut(data, Buf::header(lease).len)
        }
    }
}

impl<T, M> Resident for Buf<T, M> {
    type Data = T;

    type Meta = BufHeader<M>;

    #[inline]
    unsafe fn resident_size(_data: *mut T, meta: *mut BufHeader<M>) -> usize {
        mem::size_of::<T>().wrapping_mul((*meta).cap)
    }

    #[inline]
    unsafe fn resident_drop(data: *mut T, meta: *mut BufHeader<M>) {
        ptr::drop_in_place(slice::from_raw_parts_mut(data, (*meta).len));
    }
}

//impl<L: Lease<Data=T, Meta=BufHeader<M>>, T: Clone, M> ResidentFromClone<L, [T], M> for Buf<T, M> {
//    #[inline]
//    fn new_resident_layout(data: &[T], _meta: &M) -> Layout {
//        Layout::for_value(data)
//    }
//
//    #[inline]
//    fn new_resident_ptr(raw: *mut u8, _data: &[T], _meta: &M) -> *mut T {
//        raw as *mut T
//    }
//
//    #[inline]
//    fn new_resident(lease: &mut L, data: &[T], meta: M) {
//        unsafe {
//            let len = data.len();
//            ptr::write(lease.meta(), BufHeader {
//                len: len,
//                cap: len,
//                meta: meta,
//            });
//            slice::from_raw_parts_mut(lease.data(), len).clone_from_slice(data);
//        }
//    }
//}

impl<'a, L: Lease<Data=T, Meta=BufHeader<M>> + Holder<'a>, T, U: CloneIntoHold<'a, T>, M> ResidentFromClone<L, [U], M> for Buf<T, M> {
    #[inline]
    fn new_resident_layout(data: &[U], _meta: &M) -> Layout {
        Layout::for_value(data)
    }

    #[inline]
    fn new_resident_ptr(raw: *mut u8, _data: &[U], _meta: &M) -> *mut T {
        raw as *mut T
    }

    #[inline]
    fn new_resident(lease: &mut L, data: &[U], meta: M) {
        unsafe {
            let len = data.len();
            ptr::write(lease.meta(), BufHeader {
                len: len,
                cap: len,
                meta: meta,
            });
            let holder = lease.holder();
            let mut src = &*data as *const [U] as *const U;
            let mut dst = lease.data();
            let mut i = 0;
            while i < len {
                match (&*src).try_clone_into_hold(holder) {
                    Ok(value) => ptr::write(dst, value),
                    Err(_) => {
                        while i > 0 {
                            i = i.wrapping_sub(1);
                            dst = dst.wrapping_sub(1);
                            src = src.wrapping_sub(1);
                            ptr::drop_in_place(dst);
                        }
                        (*(*lease).meta()).len = 0;
                        panic!();
                    },
                };
                src = src.wrapping_add(1);
                dst = dst.wrapping_add(1);
                i = i.wrapping_add(1);
            }
        }
    }
}

impl<L: Lease<Data=T, Meta=BufHeader<M>>, T: Copy, M> ResidentFromCopy<L, [T], M> for Buf<T, M> {
    #[inline]
    fn new_resident_layout(data: &[T], _meta: &M) -> Layout {
        Layout::for_value(data)
    }

    #[inline]
    fn new_resident_ptr(raw: *mut u8, _data: &[T], _meta: &M) -> *mut T {
        raw as *mut T
    }

    #[inline]
    fn new_resident(lease: &mut L, data: &[T], meta: M) {
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

impl<L: Lease<Data=u8, Meta=BufHeader<M>>, M> ResidentFromCopy<L, str, M> for Buf<u8, M> {
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

impl<L: Lease<Data=T, Meta=BufHeader<M>>, T, M> ResidentFromEmpty<L, M> for Buf<T, M> {
    #[inline]
    fn new_resident_layout(_meta: &M) -> Layout {
        Layout::empty()
    }

    #[inline]
    fn new_resident_ptr(raw: *mut u8, _meta: &M) -> *mut T {
        raw as *mut T
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

impl<L: Lease<Data=T, Meta=BufHeader<M>>, T, M> ResidentWithCapacity<L, M> for Buf<T, M> {
    #[inline]
    fn new_resident_layout(cap: usize, _meta: &M) -> Result<Layout, LayoutError> {
        Layout::for_array::<T>(cap)
    }

    #[inline]
    fn new_resident_ptr(raw: *mut u8, _cap: usize, _meta: &M) -> *mut T {
        raw as *mut T
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

impl<L: Lease<Data=T, Meta=BufHeader<M>>, T, M> ResidentDeref<L> for Buf<T, M> {
    type Target = BufLease<L, T, M>;

    #[inline]
    fn resident_deref(lease: &L) -> &BufLease<L, T, M> {
        unsafe { mem::transmute::<&L, &BufLease<L, T, M>>(lease) }
    }
}

impl<L: Lease<Data=T, Meta=BufHeader<M>>, T, M> ResidentDerefMut<L> for Buf<T, M> {
    #[inline]
    fn resident_deref_mut(lease: &mut L) -> &mut BufLease<L, T, M> {
        unsafe { mem::transmute::<&mut L, &mut BufLease<L, T, M>>(lease) }
    }
}

impl<L: Lease<Data=T, Meta=BufHeader<M>>, T, M> ResidentAsRef<L, [T]> for Buf<T, M> {
    #[inline]
    fn resident_as_ref(lease: &L) -> &[T] {
        Buf::as_slice(lease)
    }
}

impl<L: Lease<Data=T, Meta=BufHeader<M>>, T, M> ResidentAsMut<L, [T]> for Buf<T, M> {
    #[inline]
    fn resident_as_mut(lease: &mut L) -> &mut [T] {
        Buf::as_mut_slice(lease)
    }
}

impl<L: Lease<Meta=BufHeader<M>>, Idx: SliceIndex<[L::Data]>, M> ResidentIndex<L, Idx> for Buf<L::Data, M>
    where L::Data: Sized
{
    type Output = Idx::Output;

    #[inline]
    fn resident_index(lease: &L, index: Idx) -> &Idx::Output {
        Buf::as_slice(lease).index(index)
    }
}

impl<L: Lease<Meta=BufHeader<M>>, Idx: SliceIndex<[L::Data]>, M> ResidentIndexMut<L, Idx> for Buf<L::Data, M>
    where L::Data: Sized
{
    #[inline]
    fn resident_index_mut(lease: &mut L, index: Idx) -> &mut Idx::Output {
        Buf::as_mut_slice(lease).index_mut(index)
    }
}

impl<L: Lease<Data=T, Meta=BufHeader<M>>, T, M> ResidentIntoIterator<L> for Buf<T, M> {
    type Item = T;

    type IntoIter = BufIter<L, T, M>;

    fn resident_into_iter(lease: L) -> BufIter<L, T, M> {
        unsafe {
            let len = Buf::header(&lease).len;
            let head = lease.data();
            assume(!head.is_null());
            let foot = if mem::size_of::<T>() != 0 {
                head.wrapping_add(len)
            } else {
                arith_offset(head as *const u8, len as isize) as *const T
            };
            BufIter {
                lease: lease,
                head: head,
                foot: foot,
            }
        }
    }
}

impl<'a, L: Lease<Data=T, Meta=BufHeader<M>>, T: 'a, M> ResidentIntoRefIterator<'a, L> for Buf<T, M> {
    type Item = &'a T;

    type IntoIter = slice::Iter<'a, T>;

    fn resident_into_iter(lease: &'a L) -> slice::Iter<'a, T> {
        Buf::as_slice(lease).into_iter()
    }
}

impl<'a, L: Lease<Data=T, Meta=BufHeader<M>>, T: 'a, M> ResidentIntoMutIterator<'a, L> for Buf<T, M> {
    type Item = &'a mut T;

    type IntoIter = slice::IterMut<'a, T>;

    fn resident_into_iter(lease: &'a mut L) -> slice::IterMut<'a, T> {
        Buf::as_mut_slice(lease).into_iter()
    }
}

impl<L: Lease<Data=T, Meta=BufHeader<M>>, T: PartialEq, M> ResidentPartialEq<L> for Buf<T, M> {
    #[inline]
    fn resident_eq(lease: &L, other: &L) -> bool {
        Buf::as_slice(lease).eq(Buf::as_slice(other))
    }

    #[inline]
    fn resident_ne(lease: &L, other: &L) -> bool {
        Buf::as_slice(lease).ne(Buf::as_slice(other))
    }
}

impl<L: Lease<Data=T, Meta=BufHeader<M>>, T: Eq, M> ResidentEq<L> for Buf<T, M> {
}

impl<L: Lease<Data=T, Meta=BufHeader<M>>, T: PartialOrd, M> ResidentPartialOrd<L> for Buf<T, M> {
    #[inline]
    fn resident_partial_cmp(lease: &L, other: &L) -> Option<Ordering> {
        Buf::as_slice(lease).partial_cmp(Buf::as_slice(other))
    }

    #[inline]
    fn resident_lt(lease: &L, other: &L) -> bool {
        Buf::as_slice(lease).lt(Buf::as_slice(other))
    }

    #[inline]
    fn resident_le(lease: &L, other: &L) -> bool {
        Buf::as_slice(lease).le(Buf::as_slice(other))
    }

    #[inline]
    fn resident_ge(lease: &L, other: &L) -> bool {
        Buf::as_slice(lease).ge(Buf::as_slice(other))
    }

    #[inline]
    fn resident_gt(lease: &L, other: &L) -> bool {
        Buf::as_slice(lease).gt(Buf::as_slice(other))
    }
}

impl<L: Lease<Data=T, Meta=BufHeader<M>>, T: Ord, M> ResidentOrd<L> for Buf<T, M> {
    #[inline]
    fn resident_cmp(lease: &L, other: &L) -> Ordering {
        Buf::as_slice(lease).cmp(Buf::as_slice(other))
    }
}

impl<L: Lease<Data=T, Meta=BufHeader<M>>, T: Hash, M> ResidentHash<L> for Buf<T, M> {
    #[inline]
    fn resident_hash<H: Hasher>(lease: &L, state: &mut H) {
        Buf::as_slice(lease).hash(state)
    }
}

impl<L: Lease<Data=T, Meta=BufHeader<M>>, T: Debug, M> ResidentDebug<L> for Buf<T, M> {
    #[inline]
    fn resident_fmt(lease: &L, f: &mut Formatter) -> fmt::Result {
        Debug::fmt(Buf::as_slice(lease), f)
    }
}

impl<L1, L2, T, M> ResidentClone<L1, L2> for Buf<T, M>
    where L1: Lease<Data=T, Meta=BufHeader<M>>,
          L2: Lease<Data=T, Meta=BufHeader<M>>,
          T: TryClone,
          M: TryClone,
{
    #[inline]
    fn new_resident_layout(lease: &L1) -> Layout {
        unsafe { Layout::for_array_unchecked::<T>(Buf::header(lease).len) }
    }

    #[inline]
    fn resident_clone(src: &L1, dst: &mut L2) -> Result<(), HoldError> {
        unsafe {
            let src_meta = src.meta();
            let dst_meta = dst.meta();
            ptr::write(dst_meta, (*src_meta).try_clone()?);
            let len = (*dst_meta).len;
            (*dst_meta).cap = len;
            let mut src_data = src.data();
            let mut dst_data = dst.data();
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

impl<'b, L1, L2, T, M> ResidentStow<'b, L1, L2> for Buf<T, M>
    where L1: Lease<Data=T, Meta=BufHeader<M>>,
          L2: Lease<Data=T, Meta=BufHeader<M>>,
          T: TryClone,
          M: TryClone,
{
    #[inline]
    fn new_resident_layout(lease: &L1) -> Layout {
        unsafe { Layout::for_array_unchecked::<T>(Buf::header(lease).len) }
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

impl<M: TryClone> TryClone for BufHeader<M> {
    #[inline]
    fn try_clone(&self) -> Result<BufHeader<M>, HoldError> {
        Ok(BufHeader {
            len: self.len,
            cap: self.cap,
            meta: self.meta.try_clone()?,
        })
    }
}

impl<L: Lease<Data=T, Meta=BufHeader<M>>, T, M> BufLease<L, T, M> {
    #[inline]
    fn header(&self) -> &BufHeader<M> {
        Buf::header(&self.lease)
    }

    #[inline]
    fn header_mut(&mut self) -> &mut BufHeader<M> {
        Buf::header_mut(&mut self.lease)
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
        if mem::size_of::<T>() != 0 {
            self.header().cap
        } else {
            !0
        }
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
    pub fn as_ptr(&self) -> *const T {
        self.lease.data()
    }

    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.lease.data()
    }

    #[inline]
    pub fn as_slice(&self) -> &[T] {
        Buf::as_slice(&self.lease)
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        Buf::as_mut_slice(&mut self.lease)
    }

    pub fn pop(&mut self) -> Option<T> {
        unsafe {
            let header = self.lease.meta();
            let len = (*header).len;
            if len != 0 {
                let len = len.wrapping_sub(1);
                (*header).len = len;
                let data = self.lease.data().wrapping_add(len);
                Some(ptr::read(data))
            } else {
                None
            }
        }
    }

    pub fn remove(&mut self, index: usize) -> T {
        unsafe {
            let header = self.lease.meta();
            let len = (*header).len;
            assert!(index <= len);
            let data = self.lease.data().wrapping_add(index);
            let elem = ptr::read(data);
            ptr::copy(data.wrapping_add(1), data, len.wrapping_sub(index).wrapping_add(1));
            (*header).len = len.wrapping_sub(1);
            elem
        }
    }

    pub fn truncate(&mut self, new_len: usize) {
        unsafe {
            let header = self.lease.meta();
            let old_len = (*header).len;
            if old_len > new_len {
                let tail = self.lease.data().wrapping_add(new_len);
                ptr::drop_in_place(slice::from_raw_parts_mut(tail, old_len.wrapping_sub(new_len)));
                (*header).len = new_len;
            }
        }
    }

    pub fn clear(&mut self) {
        self.truncate(0);
    }

    pub fn drain<R: RangeBounds<usize>>(&mut self, range: R) -> BufDrain<L, T, M> {
        let len = self.header().len;
        let lower = match range.start_bound() {
            Bound::Unbounded => 0,
            Bound::Included(&n) => n,
            Bound::Excluded(&n) => n + 1,
        };
        let upper = match range.end_bound() {
            Bound::Unbounded => len,
            Bound::Included(&n) => n + 1,
            Bound::Excluded(&n) => n,
        };
        assert!(lower <= upper);
        assert!(upper <= len);
        let data = self.lease.data();
        BufDrain {
            buf: self,
            lower: lower,
            upper: upper,
            head: data.wrapping_add(lower),
            foot: data.wrapping_add(upper),
        }
    }
}

impl<'a, L: DynamicLease<'a, Data=T, Meta=BufHeader<M>>, T, M> BufLease<L, T, M> {
    pub fn try_reserve(&mut self, ext: usize) -> Result<(), HoldError> {
        unsafe {
            let header = self.lease.meta();
            let len = (*header).len;
            let old_cap = (*header).cap;
            if old_cap.wrapping_sub(len) >= ext {
                return Ok(());
            }
            let new_cap = match len.checked_add(ext) {
                Some(cap) => cap,
                None => return Err(HoldError::Oversized),
            };
            let new_cap = match new_cap.checked_next_power_of_two() {
                Some(cap) => cap,
                None => new_cap,
            };
            let new_layout = Layout::for_array::<T>(new_cap)?;
            match self.lease.realloc(new_layout) {
                ok @ Ok(_) => {
                    (*header).cap = new_cap;
                    ok
                },
                err @ Err(_) => err,
            }
        }
    }

    pub fn reserve(&mut self, ext: usize) {
        self.try_reserve(ext).unwrap();
    }

    pub fn try_reserve_exact(&mut self, ext: usize) -> Result<(), HoldError> {
        unsafe {
            let header = self.lease.meta();
            let len = (*header).len;
            let old_cap = (*header).cap;
            if old_cap.wrapping_sub(len) >= ext {
                return Ok(());
            }
            let new_cap = match len.checked_add(ext) {
                Some(cap) => cap,
                None => return Err(HoldError::Oversized),
            };
            let new_layout = Layout::for_array::<T>(new_cap)?;
            match self.lease.realloc(new_layout) {
                ok @ Ok(_) => {
                    (*header).cap = new_cap;
                    ok
                },
                err @ Err(_) => err,
            }
        }
    }

    pub fn reserve_exact(&mut self, ext: usize) {
        self.try_reserve_exact(ext).unwrap();
    }

    pub fn try_reserve_in_place(&mut self, ext: usize) -> Result<(), HoldError> {
        unsafe {
            let header = self.lease.meta();
            let len = (*header).len;
            let old_cap = (*header).cap;
            if old_cap.wrapping_sub(len) >= ext {
                return Ok(());
            }
            let new_cap = match len.checked_add(ext) {
                Some(cap) => cap,
                None => return Err(HoldError::Oversized),
            };
            let new_cap = match new_cap.checked_next_power_of_two() {
                Some(cap) => cap,
                None => new_cap,
            };
            let new_layout = Layout::for_array::<T>(new_cap)?;
            match self.lease.resize(new_layout) {
                ok @ Ok(_) => {
                    (*header).cap = new_cap;
                    ok
                },
                err @ Err(_) => err,
            }
        }
    }

    pub fn try_reserve_in_place_exact(&mut self, ext: usize) -> Result<(), HoldError> {
        unsafe {
            let header = self.lease.meta();
            let len = (*header).len;
            let old_cap = (*header).cap;
            if old_cap.wrapping_sub(len) >= ext {
                return Ok(());
            }
            let new_cap = match len.checked_add(ext) {
                Some(cap) => cap,
                None => return Err(HoldError::Oversized),
            };
            let new_layout = Layout::for_array::<T>(new_cap)?;
            match self.lease.resize(new_layout) {
                ok @ Ok(_) => {
                    (*header).cap = new_cap;
                    ok
                },
                err @ Err(_) => err,
            }
        }
    }

    pub fn try_push(&mut self, elem: T) -> Result<(), HoldError> {
        unsafe {
            self.try_reserve(1)?;
            let header = self.lease.meta();
            let len = (*header).len;
            let data = self.lease.data().wrapping_add(len);
            ptr::write(data, elem);
            (*header).len = len.wrapping_add(1);
            Ok(())
        }
    }

    pub fn push(&mut self, elem: T) {
        self.try_push(elem).unwrap();
    }

    pub fn try_insert(&mut self, index: usize, elem: T) -> Result<(), HoldError> {
        unsafe {
            let header = self.lease.meta();
            let len = (*header).len;
            assert!(index <= len);
            self.try_reserve(1)?;
            let data = self.lease.data().wrapping_add(len);
            ptr::copy(data, data.wrapping_add(1), len.wrapping_sub(index));
            ptr::write(data, elem);
            (*header).len = len.wrapping_add(1);
            Ok(())
        }
    }

    pub fn insert(&mut self, index: usize, elem: T) {
        self.try_insert(index, elem).unwrap();
    }

    #[inline]
    pub fn try_extend<I: IntoIterator<Item=T>>(&mut self, iter: I) -> Result<(), HoldError> {
        <Self as SpecExtend<T, I::IntoIter>>::spec_try_extend(self, iter.into_iter())
    }

    #[inline]
    pub fn extend<I: IntoIterator<Item=T>>(&mut self, iter: I) {
        self.try_extend(iter).unwrap();
    }
}

impl<'a, L: DynamicLease<'a, Data=T, Meta=BufHeader<M>>, T: Clone, M> BufLease<L, T, M> {
    #[inline]
    pub fn try_extend_from_slice(&mut self, slice: &[T]) -> Result<(), HoldError> {
        self.spec_try_extend(slice.iter())
    }

    #[inline]
    pub fn extend_from_slice(&mut self, slice: &[T]) {
        self.try_extend_from_slice(slice).unwrap();
    }
}

impl<'a, L: DynamicLease<'a, Data=T, Meta=BufHeader<M>>, T: TryClone, M> BufLease<L, T, M> {
    #[inline]
    pub fn try_extend_clone<I: IntoIterator<Item=T>>(&mut self, iter: I) -> Result<(), HoldError> {
        <Self as SpecExtendClone<T, I::IntoIter>>::spec_try_extend_clone(self, iter.into_iter())
    }

    #[inline]
    pub fn extend_clone<I: IntoIterator<Item=T>>(&mut self, iter: I) {
        self.try_extend_clone(iter).unwrap();
    }

    #[inline]
    pub fn try_extend_clone_from_slice(&mut self, slice: &[T]) -> Result<(), HoldError> {
        self.spec_try_extend_clone(slice.iter())
    }

    #[inline]
    pub fn extend_clone_from_slice(&mut self, slice: &[T]) {
        self.try_extend_clone_from_slice(slice).unwrap();
    }
}

impl<L: Lease<Data=T, Meta=BufHeader<M>>, T, M> Deref for BufLease<L, T, M> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<L: Lease<Data=T, Meta=BufHeader<M>>, T, M> DerefMut for BufLease<L, T, M> {
    #[inline]
    fn deref_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

unsafe impl<L: Lease<Data=T, Meta=BufHeader<M>> + Send, T: Send, M: Send> Send for BufIter<L,T, M> {
}

unsafe impl<L: Lease<Data=T, Meta=BufHeader<M>> + Sync, T: Sync, M: Sync> Sync for BufIter<L,T, M> {
}

impl<L: Lease<Data=T, Meta=BufHeader<M>>, T, M> BufIter<L, T, M> {
    #[inline]
    fn header(&self) -> &BufHeader<M> {
        Buf::header(&self.lease)
    }

    #[inline]
    fn header_mut(&mut self) -> &mut BufHeader<M> {
        Buf::header_mut(&mut self.lease)
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
    pub fn as_slice(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.head, self.len()) }
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(self.head as *mut T, self.len()) }
    }
}

impl<L: Lease<Data=T, Meta=BufHeader<M>>, T, M> Iterator for BufIter<L, T, M> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<T> {
        unsafe {
            if self.head != self.foot {
                if mem::size_of::<T>() != 0 {
                    let head = self.head;
                    self.head = head.wrapping_add(1);
                    Some(ptr::read(head))
                } else {
                    self.head = arith_offset(self.head as *const u8, 1) as *mut T;
                    Some(ptr::read(ZSP as *mut T))
                }
            } else {
                None
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }

    #[inline]
    fn count(self) -> usize {
        self.len()
    }
}

impl<L: Lease<Data=T, Meta=BufHeader<M>>, T, M> DoubleEndedIterator for BufIter<L, T, M> {
    #[inline]
    fn next_back(&mut self) -> Option<T> {
        unsafe {
            if self.foot != self.head {
                if mem::size_of::<T>() != 0 {
                    self.foot = self.foot.wrapping_sub(1);
                    Some(ptr::read(self.foot))
                } else {
                    self.foot = arith_offset(self.foot as *const u8, -1) as *mut T;
                    Some(ptr::read(ZSP as *mut T))
                }
            } else {
                None
            }
        }
    }
}

impl<L: Lease<Data=T, Meta=BufHeader<M>>, T, M> ExactSizeIterator for BufIter<L, T, M> {
    #[inline]
    fn is_empty(&self) -> bool {
        self.head == self.foot
    }

    #[inline]
    fn len(&self) -> usize {
        let size = mem::size_of::<T>();
        if size != 0 {
            (self.foot as usize).wrapping_sub(self.head as usize) / size
        } else {
            (self.foot as usize).wrapping_sub(self.head as usize)
        }
    }
}

impl<L: Lease<Data=T, Meta=BufHeader<M>>, T, M> FusedIterator for BufIter<L, T, M> {
}

unsafe impl<L: Lease<Data=T, Meta=BufHeader<M>>, T, M> TrustedLen for BufIter<L, T, M> {
}

impl<L: Lease<Data=T, Meta=BufHeader<M>>, T: Debug, M> Debug for BufIter<L, T, M> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_tuple("BufIter").field(&self.as_slice()).finish()
    }
}

unsafe impl<L: Lease<Data=T, Meta=BufHeader<M>>, #[may_dangle] T, M> Drop for BufIter<L, T, M> {
    fn drop(&mut self) {
        unsafe {
            ptr::drop_in_place(self.as_mut_slice());
            self.header_mut().len = 0;
        }
    }
}

unsafe impl<'a, L: Lease<Data=T, Meta=BufHeader<M>> + Send, T: Send, M: Send> Send for BufDrain<'a, L,T, M> {
}

unsafe impl<'a, L: Lease<Data=T, Meta=BufHeader<M>> + Sync, T: Sync, M: Sync> Sync for BufDrain<'a, L,T, M> {
}

impl<'a, L: Lease<Data=T, Meta=BufHeader<M>>, T, M> BufDrain<'a, L, T, M> {
    #[inline]
    pub fn meta(&self) -> &M {
        self.buf.meta()
    }

    #[inline]
    pub fn meta_mut(&mut self) -> &mut M {
        self.buf.meta_mut()
    }

    #[inline]
    pub fn as_slice(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.head, self.len()) }
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(self.head as *mut T, self.len()) }
    }
}

impl<'a, L: Lease<Data=T, Meta=BufHeader<M>>, T, M> Iterator for BufDrain<'a, L, T, M> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<T> {
        unsafe {
            if self.head != self.foot {
                if mem::size_of::<T>() != 0 {
                    let head = self.head;
                    self.head = head.wrapping_add(1);
                    Some(ptr::read(head))
                } else {
                    self.head = arith_offset(self.head as *const u8, 1) as *mut T;
                    Some(ptr::read(ZSP as *mut T))
                }
            } else {
                None
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }

    #[inline]
    fn count(self) -> usize {
        self.len()
    }
}

impl<'a, L: Lease<Data=T, Meta=BufHeader<M>>, T, M> DoubleEndedIterator for BufDrain<'a, L, T, M> {
    #[inline]
    fn next_back(&mut self) -> Option<T> {
        unsafe {
            if self.foot != self.head {
                if mem::size_of::<T>() != 0 {
                    self.foot = self.foot.wrapping_sub(1);
                    Some(ptr::read(self.foot))
                } else {
                    self.foot = arith_offset(self.foot as *const u8, -1) as *mut T;
                    Some(ptr::read(ZSP as *mut T))
                }
            } else {
                None
            }
        }
    }
}

impl<'a, L: Lease<Data=T, Meta=BufHeader<M>>, T, M> ExactSizeIterator for BufDrain<'a, L, T, M> {
    #[inline]
    fn is_empty(&self) -> bool {
        self.head == self.foot
    }

    #[inline]
    fn len(&self) -> usize {
        let size = mem::size_of::<T>();
        if size != 0 {
            (self.foot as usize).wrapping_sub(self.head as usize) / size
        } else {
            (self.foot as usize).wrapping_sub(self.head as usize)
        }
    }
}

impl<'a, L: Lease<Data=T, Meta=BufHeader<M>>, T, M> FusedIterator for BufDrain<'a, L, T, M> {
}

unsafe impl<'a, L: Lease<Data=T, Meta=BufHeader<M>>, T, M> TrustedLen for BufDrain<'a, L, T, M> {
}

impl<'a, L: Lease<Data=T, Meta=BufHeader<M>>, T: Debug, M> Debug for BufDrain<'a, L, T, M> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_tuple("BufDrain").field(&self.as_slice()).finish()
    }
}

impl<'a, L: Lease<Data=T, Meta=BufHeader<M>>, T, M> Drop for BufDrain<'a, L, T, M> {
    fn drop(&mut self) {
        unsafe {
            ptr::drop_in_place(self.as_mut_slice());
            let meta = self.buf.lease.meta();
            let len = (*meta).len;
            if self.upper < len {
                let data = self.buf.lease.data();
                ptr::copy(data.wrapping_add(self.upper),
                          data.wrapping_add(self.lower),
                          len.wrapping_sub(self.upper));
                (*meta).len = len.wrapping_sub(self.upper.wrapping_sub(self.lower));
            }
        }
    }
}

trait SpecExtend<T, I> {
    fn spec_try_extend(&mut self, iter: I) -> Result<(), HoldError>;
}

impl<'a, L, T, M, I> SpecExtend<T, I> for BufLease<L, T, M>
    where L: DynamicLease<'a, Data=T, Meta=BufHeader<M>>,
          I: Iterator<Item=T>,
{
    default fn spec_try_extend(&mut self, mut iter: I) -> Result<(), HoldError> {
        unsafe {
            while let Some(elem)  = iter.next() {
                let len = self.len();
                if len == self.cap() {
                    let (lower, _) = iter.size_hint();
                    self.try_reserve(lower.saturating_add(1))?;
                }
                ptr::write(self.get_unchecked_mut(len), elem);
                self.set_len(len.wrapping_add(1));
            }
            Ok(())
        }
    }
}

impl<'a, L, T, M, I> SpecExtend<T, I> for BufLease<L, T, M>
    where L: DynamicLease<'a, Data=T, Meta=BufHeader<M>>,
          I: ExactSizeIterator<Item=T>,
{
    default fn spec_try_extend(&mut self, mut iter: I) -> Result<(), HoldError> {
        unsafe {
            self.try_reserve(iter.len())?;
            while let Some(elem)  = iter.next() {
                let len = self.len();
                debug_assert!(len != self.cap());
                ptr::write(self.get_unchecked_mut(len), elem);
                self.set_len(len.wrapping_add(1));
            }
            Ok(())
        }
    }
}

impl<'a, 'b, L, T, M, I> SpecExtend<&'b T, I> for BufLease<L, T, M>
    where L: DynamicLease<'a, Data=T, Meta=BufHeader<M>>,
          T: Clone + 'b,
          I: Iterator<Item=&'b T>,
{
    #[inline]
    default fn spec_try_extend(&mut self, iter: I) -> Result<(), HoldError> {
        self.spec_try_extend(iter.cloned())
    }
}

impl<'a, 'b, L, T, M> SpecExtend<&'b T, slice::Iter<'b, T>> for BufLease<L, T, M>
    where L: DynamicLease<'a, Data=T, Meta=BufHeader<M>>,
          T: Copy + 'b,
{
    fn spec_try_extend(&mut self, iter: slice::Iter<'b, T>) -> Result<(), HoldError> {
        unsafe {
            let slice = iter.as_slice();
            let count = slice.len();
            self.try_reserve(count)?;
            let len = self.len();
            self.set_len(len.wrapping_add(count));
            self.get_unchecked_mut(len..).copy_from_slice(slice);
            Ok(())
        }
    }
}

trait SpecExtendClone<T, I> {
    fn spec_try_extend_clone(&mut self, iter: I) -> Result<(), HoldError>;
}

impl<'a, L, T, M, I> SpecExtendClone<T, I> for BufLease<L, T, M>
    where L: DynamicLease<'a, Data=T, Meta=BufHeader<M>>,
          T: TryClone,
          I: Iterator<Item=T>,
{
    default fn spec_try_extend_clone(&mut self, mut iter: I) -> Result<(), HoldError> {
        unsafe {
            while let Some(item)  = iter.next() {
                let len = self.len();
                if len == self.cap() {
                    let (lower, _) = iter.size_hint();
                    self.try_reserve(lower.saturating_add(1))?;
                }
                ptr::write(self.get_unchecked_mut(len), match item.try_clone() {
                    Ok(elem) => elem,
                    Err(error) => return Err(error),
                });
                self.set_len(len.wrapping_add(1));
            }
            Ok(())
        }
    }
}

impl<'a, L, T, M, I> SpecExtendClone<T, I> for BufLease<L, T, M>
    where L: DynamicLease<'a, Data=T, Meta=BufHeader<M>>,
          T: TryClone,
          I: ExactSizeIterator<Item=T>,
{
    default fn spec_try_extend_clone(&mut self, mut iter: I) -> Result<(), HoldError> {
        unsafe {
            self.try_reserve(iter.len())?;
            while let Some(item)  = iter.next() {
                let len = self.len();
                debug_assert!(len != self.cap());
                ptr::write(self.get_unchecked_mut(len), match item.try_clone() {
                    Ok(elem) => elem,
                    Err(error) => return Err(error),
                });
                self.set_len(len.wrapping_add(1));
            }
            Ok(())
        }
    }
}

impl<'a, 'b, L, T, M, I> SpecExtendClone<&'b T, I> for BufLease<L, T, M>
    where L: DynamicLease<'a, Data=T, Meta=BufHeader<M>>,
          T: TryClone + 'b,
          I: Iterator<Item=&'b T>,
{
    default fn spec_try_extend_clone(&mut self, mut iter: I) -> Result<(), HoldError> {
        unsafe {
            while let Some(item)  = iter.next() {
                let len = self.len();
                if len == self.cap() {
                    let (lower, _) = iter.size_hint();
                    self.try_reserve(lower.saturating_add(1))?;
                }
                ptr::write(self.get_unchecked_mut(len), match item.try_clone() {
                    Ok(elem) => elem,
                    Err(error) => return Err(error),
                });
                self.set_len(len.wrapping_add(1));
            }
            Ok(())
        }
    }
}

impl<'a, 'b, L, T, M, I> SpecExtendClone<&'b T, I> for BufLease<L, T, M>
    where L: DynamicLease<'a, Data=T, Meta=BufHeader<M>>,
          T: TryClone + 'b,
          I: ExactSizeIterator<Item=&'b T>,
{
    default fn spec_try_extend_clone(&mut self, mut iter: I) -> Result<(), HoldError> {
        unsafe {
            self.try_reserve(iter.len())?;
            while let Some(item)  = iter.next() {
                let len = self.len();
                debug_assert!(len != self.cap());
                ptr::write(self.get_unchecked_mut(len), match item.try_clone() {
                    Ok(elem) => elem,
                    Err(error) => return Err(error),
                });
                self.set_len(len.wrapping_add(1));
            }
            Ok(())
        }
    }
}
