use core::cmp::Ordering;
use core::fmt::{self, Debug, Display, Pointer, Formatter};
use core::hash::{Hash, Hasher};
use core::marker::PhantomData;
use core::mem;
use core::ops::{Deref, DerefMut, Index, IndexMut, Add, AddAssign};
use core::ptr::{self, NonNull};
use crate::block::{self, Block, Layout};
use crate::alloc::{AllocTag, Hold, Holder, HoldError, Stow, StowFrom, TryClone, CloneIntoHold};
use crate::lease::{Lease, DynamicLease};
use crate::resident::{Resident, ResidentFromValue, ResidentFromClone,
                      ResidentFromCloneUnchecked, ResidentFromCopy,
                      ResidentFromCopyUnchecked, ResidentFromEmpty,
                      ResidentWithCapacity, ResidentUnwrap, ResidentDeref,
                      ResidentDerefMut, ResidentAsRef, ResidentAsMut,
                      ResidentIndex, ResidentIndexMut, ResidentAdd,
                      ResidentAddAssign, ResidentIntoIterator,
                      ResidentIntoRefIterator, ResidentIntoMutIterator,
                      ResidentPartialEq, ResidentEq, ResidentPartialOrd,
                      ResidentOrd, ResidentHash, ResidentDisplay, ResidentDebug,
                      ResidentClone, ResidentStow};

/// An exclusive reference to a `Resident` occupying an owned, `Hold`-allocated
/// memory block, with resident metadata stored with the pointer.
pub struct Raw<'a, R: Resident> {
    /// Pointer to the owned memory block.
    data: NonNull<R::Data>,
    /// Resident metadata.
    meta: R::Meta,
    /// Variant over R::Data, with drop check.
    data_lifetime: PhantomData<R::Data>,
    /// Variant over 'a.
    hold_lifetime: PhantomData<&'a ()>,
}

unsafe impl<'a, R: Resident> Send for Raw<'a, R> where R::Data: Send, R::Meta: Send {
}

unsafe impl<'a, R: Resident> Sync for Raw<'a, R> where R::Data: Sync, R::Meta: Sync {
}

impl<'a, R: Resident> Raw<'a, R> {
    #[inline]
    pub fn try_hold_new_meta<T, M>(hold: &dyn Hold<'a>, data: T, meta: M)
        -> Result<Raw<'a, R>, HoldError>
        where R: ResidentFromValue<Raw<'a, R>, T, M>
    {
        unsafe {
            let layout = R::new_resident_layout(&data, &meta);
            let block = hold.alloc(layout)?;
            let mut lease = Raw {
                data: NonNull::new_unchecked(R::new_resident_ptr(block.as_ptr(), &data, &meta)),
                meta: mem::uninitialized(),
                data_lifetime: PhantomData,
                hold_lifetime: PhantomData,
            };
            R::new_resident(&mut lease, data, meta);
            Ok(lease)
        }
    }

    #[inline]
    pub fn try_hold_clone_meta<T: ?Sized, M>(hold: &dyn Hold<'a>, data: &T, meta: M)
        -> Result<Raw<'a, R>, HoldError>
        where R: ResidentFromClone<Raw<'a, R>, T, M>
    {
        unsafe {
            let layout = R::new_resident_layout(data, &meta);
            let block = hold.alloc(layout)?;
            let mut lease = Raw {
                data: NonNull::new_unchecked(R::new_resident_ptr(block.as_ptr(), data, &meta)),
                meta: mem::uninitialized(),
                data_lifetime: PhantomData,
                hold_lifetime: PhantomData,
            };
            R::new_resident(&mut lease, data, meta);
            Ok(lease)
        }
    }

    #[inline]
    pub unsafe fn try_hold_clone_unchecked_meta<T: ?Sized, M>(hold: &dyn Hold<'a>, data: &T, meta: M)
        -> Result<Raw<'a, R>, HoldError>
        where R: ResidentFromCloneUnchecked<Raw<'a, R>, T, M>
    {
        let layout = R::new_resident_layout(data, &meta);
        let block = hold.alloc(layout)?;
        let mut lease = Raw {
            data: NonNull::new_unchecked(R::new_resident_ptr(block.as_ptr(), data, &meta)),
            meta: mem::uninitialized(),
            data_lifetime: PhantomData,
            hold_lifetime: PhantomData,
        };
        R::new_resident(&mut lease, data, meta);
        Ok(lease)
    }

    #[inline]
    pub fn try_hold_copy_meta<T: ?Sized, M>(hold: &dyn Hold<'a>, data: &T, meta: M)
        -> Result<Raw<'a, R>, HoldError>
        where R: ResidentFromCopy<Raw<'a, R>, T, M>
    {
        unsafe {
            let layout = R::new_resident_layout(data, &meta);
            let block = hold.alloc(layout)?;
            let mut lease = Raw {
                data: NonNull::new_unchecked(R::new_resident_ptr(block.as_ptr(), data, &meta)),
                meta: mem::uninitialized(),
                data_lifetime: PhantomData,
                hold_lifetime: PhantomData,
            };
            R::new_resident(&mut lease, data, meta);
            Ok(lease)
        }
    }

    #[inline]
    pub unsafe fn try_hold_copy_unchecked_meta<T: ?Sized, M>(hold: &dyn Hold<'a>, data: &T, meta: M)
        -> Result<Raw<'a, R>, HoldError>
        where R: ResidentFromCopyUnchecked<Raw<'a, R>, T, M>
    {
        let layout = R::new_resident_layout(data, &meta);
        let block = hold.alloc(layout)?;
        let mut lease = Raw {
            data: NonNull::new_unchecked(R::new_resident_ptr(block.as_ptr(), data, &meta)),
            meta: mem::uninitialized(),
            data_lifetime: PhantomData,
            hold_lifetime: PhantomData,
        };
        R::new_resident(&mut lease, data, meta);
        Ok(lease)
    }

    #[inline]
    pub fn try_hold_empty_meta<M>(hold: &dyn Hold<'a>, meta: M)
        -> Result<Raw<'a, R>, HoldError>
        where R: ResidentFromEmpty<Raw<'a, R>, M>
    {
        unsafe {
            let layout = R::new_resident_layout(&meta);
            let block = hold.alloc(layout)?;
            let mut lease = Raw {
                data: NonNull::new_unchecked(R::new_resident_ptr(block.as_ptr(), &meta)),
                meta: mem::uninitialized(),
                data_lifetime: PhantomData,
                hold_lifetime: PhantomData,
            };
            R::new_resident(&mut lease, meta);
            Ok(lease)
        }
    }

    #[inline]
    pub fn try_hold_cap_meta<M>(hold: &dyn Hold<'a>, cap: usize, meta: M)
        -> Result<Raw<'a, R>, HoldError>
        where R: ResidentWithCapacity<Raw<'a, R>, M>
    {
        unsafe {
            let layout = R::new_resident_layout(cap, &meta)?;
            let block = hold.alloc(layout)?;
            let mut lease = Raw {
                data: NonNull::new_unchecked(R::new_resident_ptr(block.as_ptr(), cap, &meta)),
                meta: mem::uninitialized(),
                data_lifetime: PhantomData,
                hold_lifetime: PhantomData,
            };
            R::new_resident(&mut lease, cap, meta);
            Ok(lease)
        }
    }

    #[inline]
    pub fn try_hold_new<T>(hold: &dyn Hold<'a>, data: T) -> Result<Raw<'a, R>, HoldError>
        where R: ResidentFromValue<Raw<'a, R>, T>
    {
        Raw::try_hold_new_meta(hold, data, ())
    }

    #[inline]
    pub fn try_hold_clone<T: ?Sized>(hold: &dyn Hold<'a>, data: &T) -> Result<Raw<'a, R>, HoldError>
        where R: ResidentFromClone<Raw<'a, R>, T>
    {
        Raw::try_hold_clone_meta(hold, data, ())
    }

    #[inline]
    pub unsafe fn try_hold_clone_unchecked<T: ?Sized>(hold: &dyn Hold<'a>, data: &T) -> Result<Raw<'a, R>, HoldError>
        where R: ResidentFromCloneUnchecked<Raw<'a, R>, T>
    {
        Raw::try_hold_clone_unchecked_meta(hold, data, ())
    }

    #[inline]
    pub fn try_hold_copy<T: ?Sized>(hold: &dyn Hold<'a>, data: &T) -> Result<Raw<'a, R>, HoldError>
        where R: ResidentFromCopy<Raw<'a, R>, T>
    {
        Raw::try_hold_copy_meta(hold, data, ())
    }

    #[inline]
    pub unsafe fn try_hold_copy_unchecked<T: ?Sized>(hold: &dyn Hold<'a>, data: &T) -> Result<Raw<'a, R>, HoldError>
        where R: ResidentFromCopyUnchecked<Raw<'a, R>, T>
    {
        Raw::try_hold_copy_unchecked_meta(hold, data, ())
    }

    #[inline]
    pub fn try_hold_empty(hold: &dyn Hold<'a>) -> Result<Raw<'a, R>, HoldError>
        where R: ResidentFromEmpty<Raw<'a, R>>
    {
        Raw::try_hold_empty_meta(hold, ())
    }

    #[inline]
    pub fn try_hold_cap(hold: &dyn Hold<'a>, cap: usize) -> Result<Raw<'a, R>, HoldError>
        where R: ResidentWithCapacity<Raw<'a, R>>
    {
        Raw::try_hold_cap_meta(hold, cap, ())
    }

    #[inline]
    pub fn hold_new<T>(hold: &dyn Hold<'a>, data: T) -> Raw<'a, R>
        where R: ResidentFromValue<Raw<'a, R>, T>
    {
        Raw::try_hold_new(hold, data).unwrap()
    }

    #[inline]
    pub fn hold_clone<T: ?Sized>(hold: &dyn Hold<'a>, data: &T) -> Raw<'a, R>
        where R: ResidentFromClone<Raw<'a, R>, T>
    {
        Raw::try_hold_clone(hold, data).unwrap()
    }

    #[inline]
    pub unsafe fn hold_clone_unchecked<T: ?Sized>(hold: &dyn Hold<'a>, data: &T) -> Raw<'a, R>
        where R: ResidentFromCloneUnchecked<Raw<'a, R>, T>
    {
        Raw::try_hold_clone_unchecked(hold, data).unwrap()
    }

    #[inline]
    pub fn hold_copy<T: ?Sized>(hold: &dyn Hold<'a>, data: &T) -> Raw<'a, R>
        where R: ResidentFromCopy<Raw<'a, R>, T>
    {
        Raw::try_hold_copy(hold, data).unwrap()
    }

    #[inline]
    pub unsafe fn hold_copy_unchecked<T: ?Sized>(hold: &dyn Hold<'a>, data: &T) -> Raw<'a, R>
        where R: ResidentFromCopyUnchecked<Raw<'a, R>, T>
    {
        Raw::try_hold_copy_unchecked(hold, data).unwrap()
    }

    #[inline]
    pub fn hold_empty(hold: &dyn Hold<'a>) -> Raw<'a, R>
        where R: ResidentFromEmpty<Raw<'a, R>>
    {
        Raw::try_hold_empty(hold).unwrap()
    }

    #[inline]
    pub fn hold_cap(hold: &dyn Hold<'a>, cap: usize) -> Raw<'a, R>
        where R: ResidentWithCapacity<Raw<'a, R>>
    {
        Raw::try_hold_cap(hold, cap).unwrap()
    }

    #[inline]
    pub fn new<T>(data: T) -> Raw<'a, R>
        where R: ResidentFromValue<Raw<'a, R>, T>
    {
        Raw::hold_new(Hold::global(), data)
    }

    #[inline]
    pub fn from_clone<T: ?Sized>(data: &T) -> Raw<'a, R>
        where R: ResidentFromClone<Raw<'a, R>, T>
    {
        Raw::hold_clone(Hold::global(), data)
    }

    #[inline]
    pub unsafe fn from_clone_unchecked<T: ?Sized>(data: &T) -> Raw<'a, R>
        where R: ResidentFromCloneUnchecked<Raw<'a, R>, T>
    {
        Raw::hold_clone_unchecked(Hold::global(), data)
    }

    #[inline]
    pub fn from_copy<T: ?Sized>(data: &T) -> Raw<'a, R>
        where R: ResidentFromCopy<Raw<'a, R>, T>
    {
        Raw::hold_copy(Hold::global(), data)
    }

    #[inline]
    pub unsafe fn from_copy_unchecked<T: ?Sized>(data: &T) -> Raw<'a, R>
        where R: ResidentFromCopyUnchecked<Raw<'a, R>, T>
    {
        Raw::hold_copy_unchecked(Hold::global(), data)
    }

    #[inline]
    pub fn empty() -> Raw<'a, R>
        where R: ResidentFromEmpty<Raw<'a, R>>
    {
        Raw::hold_empty(Hold::global())
    }

    #[inline]
    pub fn with_cap(cap: usize) -> Raw<'a, R>
        where R: ResidentWithCapacity<Raw<'a, R>>
    {
        Raw::hold_cap(Hold::global(), cap)
    }

    #[inline]
    pub unsafe fn from_raw_meta(data: *mut R::Data, meta: R::Meta) -> Raw<'a, R> {
        Raw {
            data: NonNull::new_unchecked(data),
            meta: meta,
            data_lifetime: PhantomData,
            hold_lifetime: PhantomData,
        }
    }

    #[inline]
    pub unsafe fn into_raw_meta(this: Raw<'a, R>) -> (*mut R::Data, R::Meta) {
        let data = this.data.as_ptr();
        let meta = ptr::read(&this.meta);
        mem::forget(this);
        (data, meta)
    }

    pub fn unwrap(mut this: Raw<'a, R>) -> R::Target where R: ResidentUnwrap<Raw<'a, R>> {
        unsafe {
            let data = this.data.as_ptr();
            let size = R::resident_size(data, &mut this.meta);
            let resident = R::resident_unwrap(&this);
            ptr::drop_in_place(&mut this.meta);
            mem::forget(this);
            let block = Block::from_raw_parts(data as *mut u8, size);
            AllocTag::from_ptr(data as *mut u8).dealloc(block);
            resident
        }
    }
}

impl<'a, R: Resident<Meta=()>> Raw<'a, R> {
    #[inline]
    pub unsafe fn from_raw(data: *mut R::Data) -> Raw<'a, R> {
        Raw {
            data: NonNull::new_unchecked(data),
            meta: (),
            data_lifetime: PhantomData,
            hold_lifetime: PhantomData,
        }
    }

    #[inline]
    pub unsafe fn into_raw(this: Raw<'a, R>) -> *mut R::Data {
        let data = this.data.as_ptr();
        mem::forget(this);
        data
    }
}

impl<'a, R: Resident> Holder<'a> for Raw<'a, R> {
    #[inline]
    fn holder(&self) -> &'a dyn Hold<'a> {
        AllocTag::from_ptr(self.data.as_ptr() as *mut u8).holder()
    }
}

impl<'a, R: Resident> Lease for Raw<'a, R> {
    type Data = R::Data;

    type Meta = R::Meta;

    #[inline]
    fn data(&self) -> *mut R::Data {
        self.data.as_ptr()
    }

    #[inline]
    fn meta(&self) -> *mut R::Meta {
        &self.meta as *const R::Meta as *mut R::Meta
    }
}

impl<'a, R: Resident> DynamicLease<'a> for Raw<'a, R> {
    unsafe fn resize(&mut self, layout: Layout) -> Result<(), HoldError> {
        let old_data = self.data.as_ptr();
        let old_size = R::resident_size(old_data, &mut self.meta);
        let old_block = Block::from_raw_parts(old_data as *mut u8, old_size);
        let hold = AllocTag::from_ptr(old_data as *mut u8).holder();
        match hold.resize(old_block, layout) {
            Ok(_) => Ok(()),
            Err(_) => Err(HoldError::OutOfMemory),
        }
    }

    unsafe fn realloc(&mut self, layout: Layout) -> Result<(), HoldError> {
        let old_data = self.data.as_ptr();
        let old_size = R::resident_size(old_data, &mut self.meta);
        let old_block = Block::from_raw_parts(old_data as *mut u8, old_size);
        let hold = AllocTag::from_ptr(old_data as *mut u8).holder();
        match hold.realloc(old_block, layout) {
            Ok(new_block) => {
                // Get a fat pointer to the reallocated resident.
                let new_data = block::set_address(old_data, new_block.as_ptr() as usize);
                self.data = NonNull::new_unchecked(new_data);
                Ok(())
            },
            Err(_) => Err(HoldError::OutOfMemory),
        }
    }
}

impl<'a, R: ResidentDeref<Raw<'a, R>>> Deref for Raw<'a, R> {
    type Target = R::Target;

    #[inline]
    fn deref(&self) -> &R::Target {
        R::resident_deref(self)
    }
}

impl<'a, R: ResidentDerefMut<Raw<'a, R>>> DerefMut for Raw<'a, R> {
    #[inline]
    fn deref_mut(&mut self) -> &mut R::Target {
        R::resident_deref_mut(self)
    }
}

impl<'a, R: ResidentAsRef<Raw<'a, R>, T>, T: ?Sized> AsRef<T> for Raw<'a, R> {
    #[inline]
    fn as_ref(&self) -> &T {
        R::resident_as_ref(self)
    }
}

impl<'a, R: ResidentAsMut<Raw<'a, R>, T>, T: ?Sized> AsMut<T> for Raw<'a, R> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        R::resident_as_mut(self)
    }
}

impl<'a, R: ResidentIndex<Raw<'a, R>, Idx>, Idx> Index<Idx> for Raw<'a, R> {
    type Output = R::Output;

    #[inline]
    fn index(&self, index: Idx) -> &R::Output {
        R::resident_index(self, index)
    }
}

impl<'a, R: ResidentIndexMut<Raw<'a, R>, Idx>, Idx> IndexMut<Idx> for Raw<'a, R> {
    #[inline]
    fn index_mut(&mut self, index: Idx) -> &mut R::Output {
        R::resident_index_mut(self, index)
    }
}

impl<'a, R: ResidentAdd<Raw<'a, R>, Rhs>, Rhs> Add<Rhs> for Raw<'a, R> {
    type Output = R::Output;

    #[inline]
    fn add(self, rhs: Rhs) -> R::Output {
        R::resident_add(self, rhs)
    }
}

impl<'a, R: ResidentAddAssign<Raw<'a, R>, Rhs>, Rhs> AddAssign<Rhs> for Raw<'a, R> {
    #[inline]
    fn add_assign(&mut self, rhs: Rhs) {
        R::resident_add_assign(self, rhs);
    }
}

impl<'a, R: ResidentIntoIterator<Raw<'a, R>>> IntoIterator for Raw<'a, R> {
    type Item = R::Item;

    type IntoIter = R::IntoIter;

    #[inline]
    fn into_iter(self) -> R::IntoIter {
        R::resident_into_iter(self)
    }
}

impl<'a, R: ResidentIntoRefIterator<'a, Raw<'a, R>>> IntoIterator for &'a Raw<'a, R> {
    type Item = R::Item;

    type IntoIter = R::IntoIter;

    #[inline]
    fn into_iter(self) -> R::IntoIter {
        R::resident_into_iter(self)
    }
}

impl<'a, R: ResidentIntoMutIterator<'a, Raw<'a, R>>> IntoIterator for &'a mut Raw<'a, R> {
    type Item = R::Item;

    type IntoIter = R::IntoIter;

    #[inline]
    fn into_iter(self) -> R::IntoIter {
        R::resident_into_iter(self)
    }
}

impl<'a, R: ResidentPartialEq<Raw<'a, R>, T>, T: ?Sized> PartialEq<T> for Raw<'a, R> {
    #[inline]
    fn eq(&self, other: &T) -> bool {
        R::resident_eq(self, other)
    }

    #[inline]
    fn ne(&self, other: &T) -> bool {
        R::resident_ne(self, other)
    }
}

impl<'a, R: ResidentEq<Raw<'a, R>>> Eq for Raw<'a, R> {
}

impl<'a, R: ResidentPartialOrd<Raw<'a, R>, T>, T: ?Sized> PartialOrd<T> for Raw<'a, R> {
    #[inline]
    fn partial_cmp(&self, other: &T) -> Option<Ordering> {
        R::resident_partial_cmp(self, other)
    }

    #[inline]
    fn lt(&self, other: &T) -> bool {
        R::resident_lt(self, other)
    }

    #[inline]
    fn le(&self, other: &T) -> bool {
        R::resident_le(self, other)
    }

    #[inline]
    fn ge(&self, other: &T) -> bool {
        R::resident_ge(self, other)
    }

    #[inline]
    fn gt(&self, other: &T) -> bool {
        R::resident_gt(self, other)
    }
}

impl<'a, R: ResidentOrd<Raw<'a, R>>> Ord for Raw<'a, R> {
    #[inline]
    fn cmp(&self, other: &Raw<'a, R>) -> Ordering {
        R::resident_cmp(self, other)
    }
}

impl<'a, R: ResidentHash<Raw<'a, R>>> Hash for Raw<'a, R> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        R::resident_hash(self, state)
    }
}

impl<'a, R: ResidentDisplay<Raw<'a, R>>> Display for Raw<'a, R> {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        R::resident_fmt(self, f)
    }
}

impl<'a, R: ResidentDebug<Raw<'a, R>>> Debug for Raw<'a, R> {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        R::resident_fmt(self, f)
    }
}

impl<'a, R: Resident> Pointer for Raw<'a, R> {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Pointer::fmt(&self.data.as_ptr(), f)
    }
}

impl<'a, R: ResidentClone<Raw<'a, R>, Raw<'a, R>>> Clone for Raw<'a, R> {
    fn clone(&self) -> Raw<'a, R> {
        self.try_clone().unwrap()
    }
}

impl<'a, R: ResidentClone<Raw<'a, R>, Raw<'a, R>>> TryClone for Raw<'a, R> {
    fn try_clone(&self) -> Result<Raw<'a, R>, HoldError> {
        self.try_clone_into_hold(self.holder())
    }
}

impl<'a, 'b, R: ResidentClone<Raw<'b, R>, Raw<'a, R>>> CloneIntoHold<'a, Raw<'a, R>> for Raw<'b, R> {
    fn try_clone_into_hold(&self, hold: &Hold<'a>) -> Result<Raw<'a, R>, HoldError> {
        unsafe {
            // Get a copy of the source resident pointer.
            let src_data = self.data.as_ptr();
            // Get the preferred memory layout of a clone destination for the resident.
            let dst_layout = R::new_resident_layout(self);
            // Allocate a new memory block to hold the cloned resident, bailing on failure.
            let dst_block = hold.alloc(dst_layout)?;
            // Get a fat pointer to the destination resident.
            let dst_data = block::set_address(src_data, dst_block.as_ptr() as usize);
            // Create a destination lease for the new block, with uninitialized metadata.
            let mut dst = Raw::from_raw_meta(dst_data, mem::uninitialized());
            // Clone the resident from the source lease to the destination lease.
            match R::resident_clone(self, &mut dst) {
                // Clone succeeded; return the destination lease.
                Ok(_) => Ok(dst),
                // Clone failed.
                Err(error) => {
                    // Deallocate the destination block.
                    hold.dealloc(dst_block);
                    // Abandon the destination lease.
                    mem::forget(dst);
                    // Return the clone error.
                    Err(error)
                },
            }
        }
    }
}

impl<'a, 'b, R: ResidentStow<'b, Raw<'a, R>, Raw<'b, R>>> Stow<'b, Raw<'b, R>> for Raw<'a, R> {
    unsafe fn stow(src: *mut Raw<'a, R>, dst: *mut Raw<'b, R>, hold: &Hold<'b>) -> Result<(), HoldError> {
        // Get a copy of the source resident pointer.
        let src_data = (*src).data.as_ptr();
        // Get the size of the resident.
        let size = R::resident_size(src_data, &mut (*src).meta);
        // Get the memory layout of the resident.
        let layout = Layout::from_size_align_unchecked(size, mem::align_of_val(&*src_data));
        // Allocate a destination memory block to hold the relocated resident, bailing on failure.
        let dst_block = hold.alloc(layout)?;
        // Get a fat pointer to the destination resident.
        let dst_data = block::set_address(src_data, dst_block.as_ptr() as usize);
        // Init the resident pointer of the destination lease.
        ptr::write(&mut (*dst).data, NonNull::new_unchecked(dst_data));
        // Try to stow the resident.
        if let err @ Err(_) = R::resident_stow(&mut *src, &mut *dst, hold) {
            // Free the newly allocated arc.
            hold.dealloc(dst_block);
            // Before returning the error.
            return err;
        }
        // Return successfully.
        Ok(())
    }

    unsafe fn unstow(_src: *mut Raw<'a, R>, _dst: *mut Raw<'b, R>) {
        panic!("unsupported");
    }
}

impl<'a, 'b, R: ResidentStow<'b, Raw<'a, R>, Raw<'b, R>>> StowFrom<'b, Raw<'a, R>> for Raw<'b, R> {
    fn try_stow_from(mut src: Raw<'a, R>, hold: &Hold<'b>) -> Result<Raw<'b, R>, (Raw<'a, R>, HoldError)> {
        unsafe {
            let mut dst = mem::uninitialized::<Raw<'b, R>>();
            if let Err(error) = Stow::stow(&mut src, &mut dst, hold) {
                mem::forget(dst);
                return Err((src, error));
            }
            return Ok(dst);
        }
    }
}

unsafe impl<'a, #[may_dangle] R: Resident> Drop for Raw<'a, R> {
    fn drop(&mut self) {
        unsafe {
            let data = self.data.as_ptr();
            let meta = &mut self.meta;
            let size = R::resident_size(data, meta);
            R::resident_drop(data, meta);
            let block = Block::from_raw_parts(data as *mut u8, size);
            AllocTag::from_ptr(data as *mut u8).dealloc(block);
        }
    }
}
