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
/// memory block, with resident metadata stored within the allocation.
pub struct Ptr<'a, R: Resident> {
    /// Pointer to the owned memory block.
    data: NonNull<R::Data>,
    /// Variant over R::Data, with drop check.
    data_lifetime: PhantomData<R::Data>,
    /// Variant over R::Meta, with drop check.
    meta_lifetime: PhantomData<R::Meta>,
    /// Variant over 'a.
    hold_lifetime: PhantomData<&'a ()>,
}

unsafe impl<'a, R: Resident> Send for Ptr<'a, R> where R::Data: Send, R::Meta: Send {
}

unsafe impl<'a, R: Resident> Sync for Ptr<'a, R> where R::Data: Sync, R::Meta: Sync {
}

impl<'a, R: Resident> Ptr<'a, R> {
    #[inline]
    pub fn try_hold_new_meta<T, M>(hold: &dyn Hold<'a>, data: T, meta: M)
        -> Result<Ptr<'a, R>, HoldError>
        where R: ResidentFromValue<Ptr<'a, R>, T, M>
    {
        unsafe {
            // Compute the layout of the allocation structure, capturing the
            // offset of its resident field.
            let (layout, offset) = Layout::for_type::<R::Meta>()
                .extended(R::new_resident_layout(&data, &meta))?;
            // Allocate a block of memory to hold the allocation structure,
            // bailing on failure.
            let block = hold.alloc(layout)?;
            // Get a pointer to the header field of the new allocation.
            let header = block.as_ptr() as *mut R::Meta;
            // Get a raw pointer to the resident field of the new allocation.
            let field = (header as *mut u8).wrapping_add(offset);
            // Get a fat pointer to the resident field.
            let resident = R::new_resident_ptr(field, &data, &meta);
            // Construct a new Ptr lease.
            let mut lease = Ptr::from_raw(resident);
            // Initialize the new resident.
            R::new_resident(&mut lease, data, meta);
            // Return the new lease.
            Ok(lease)
        }
    }

    #[inline]
    pub fn try_hold_clone_meta<T: ?Sized, M>(hold: &dyn Hold<'a>, data: &T, meta: M)
        -> Result<Ptr<'a, R>, HoldError>
        where R: ResidentFromClone<Ptr<'a, R>, T, M>
    {
        unsafe {
            // Compute the layout of the allocation structure, capturing the
            // offset of its resident field.
            let (layout, offset) = Layout::for_type::<R::Meta>()
                .extended(R::new_resident_layout(data, &meta))?;
            // Allocate a block of memory to hold the allocation structure,
            // bailing on failure.
            let block = hold.alloc(layout)?;
            // Get a pointer to the header field of the new allocation.
            let header = block.as_ptr() as *mut R::Meta;
            // Get a raw pointer to the resident field of the new allocation.
            let field = (header as *mut u8).wrapping_add(offset);
            // Get a fat pointer to the resident field.
            let resident = R::new_resident_ptr(field, data, &meta);
            // Construct a new Ptr lease.
            let mut lease = Ptr::from_raw(resident);
            // Initialize the new resident.
            R::new_resident(&mut lease, data, meta);
            // Return the new lease.
            Ok(lease)
        }
    }

    #[inline]
    pub unsafe fn try_hold_clone_unchecked_meta<T: ?Sized, M>(hold: &dyn Hold<'a>, data: &T, meta: M)
        -> Result<Ptr<'a, R>, HoldError>
        where R: ResidentFromCloneUnchecked<Ptr<'a, R>, T, M>
    {
        // Compute the layout of the allocation structure, capturing the
        // offset of its resident field.
        let (layout, offset) = Layout::for_type::<R::Meta>()
            .extended(R::new_resident_layout(data, &meta))?;
        // Allocate a block of memory to hold the allocation structure,
        // bailing on failure.
        let block = hold.alloc(layout)?;
        // Get a pointer to the header field of the new allocation.
        let header = block.as_ptr() as *mut R::Meta;
        // Get a raw pointer to the resident field of the new allocation.
        let field = (header as *mut u8).wrapping_add(offset);
        // Get a fat pointer to the resident field.
        let resident = R::new_resident_ptr(field, data, &meta);
        // Construct a new Ptr lease.
        let mut lease = Ptr::from_raw(resident);
        // Initialize the new resident.
        R::new_resident(&mut lease, data, meta);
        // Return the new lease.
        Ok(lease)
    }

    #[inline]
    pub fn try_hold_copy_meta<T: ?Sized, M>(hold: &dyn Hold<'a>, data: &T, meta: M)
        -> Result<Ptr<'a, R>, HoldError>
        where R: ResidentFromCopy<Ptr<'a, R>, T, M>
    {
        unsafe {
            // Compute the layout of the allocation structure, capturing the
            // offset of its resident field.
            let (layout, offset) = Layout::for_type::<R::Meta>()
                .extended(R::new_resident_layout(data, &meta))?;
            // Allocate a block of memory to hold the allocation structure,
            // bailing on failure.
            let block = hold.alloc(layout)?;
            // Get a pointer to the header field of the new allocation.
            let header = block.as_ptr() as *mut R::Meta;
            // Get a raw pointer to the resident field of the new allocation.
            let field = (header as *mut u8).wrapping_add(offset);
            // Get a fat pointer to the resident field.
            let resident = R::new_resident_ptr(field, data, &meta);
            // Construct a new Ptr lease.
            let mut lease = Ptr::from_raw(resident);
            // Initialize the new resident.
            R::new_resident(&mut lease, data, meta);
            // Return the new lease.
            Ok(lease)
        }
    }

    #[inline]
    pub unsafe fn try_hold_copy_unchecked_meta<T: ?Sized, M>(hold: &dyn Hold<'a>, data: &T, meta: M)
        -> Result<Ptr<'a, R>, HoldError>
        where R: ResidentFromCopyUnchecked<Ptr<'a, R>, T, M>
    {
        // Compute the layout of the allocation structure, capturing the
        // offset of its resident field.
        let (layout, offset) = Layout::for_type::<R::Meta>()
            .extended(R::new_resident_layout(data, &meta))?;
        // Allocate a block of memory to hold the allocation structure,
        // bailing on failure.
        let block = hold.alloc(layout)?;
        // Get a pointer to the header field of the new allocation.
        let header = block.as_ptr() as *mut R::Meta;
        // Get a raw pointer to the resident field of the new allocation.
        let field = (header as *mut u8).wrapping_add(offset);
        // Get a fat pointer to the resident field.
        let resident = R::new_resident_ptr(field, data, &meta);
        // Construct a new Ptr lease.
        let mut lease = Ptr::from_raw(resident);
        // Initialize the new resident.
        R::new_resident(&mut lease, data, meta);
        // Return the new lease.
        Ok(lease)
    }

    #[inline]
    pub fn try_hold_empty_meta<M>(hold: &dyn Hold<'a>, meta: M)
        -> Result<Ptr<'a, R>, HoldError>
        where R: ResidentFromEmpty<Ptr<'a, R>, M>
    {
        unsafe {
            // Compute the layout of the allocation structure, capturing the
            // offset of its resident field.
            let (layout, offset) = Layout::for_type::<R::Meta>()
                .extended(R::new_resident_layout(&meta))?;
            // Allocate a block of memory to hold the allocation structure,
            // bailing on failure.
            let block = hold.alloc(layout)?;
            // Get a pointer to the header field of the new allocation.
            let header = block.as_ptr() as *mut R::Meta;
            // Get a raw pointer to the resident field of the new arc.
            let field = (header as *mut u8).wrapping_add(offset);
            // Get a fat pointer to the resident field.
            let resident = R::new_resident_ptr(field, &meta);
            // Construct a new Ptr lease.
            let mut lease = Ptr::from_raw(resident);
            // Initialize the new resident.
            R::new_resident(&mut lease, meta);
            // Return the new lease.
            Ok(lease)
        }
    }

    #[inline]
    pub fn try_hold_cap_meta<M>(hold: &dyn Hold<'a>, cap: usize, meta: M)
        -> Result<Ptr<'a, R>, HoldError>
        where R: ResidentWithCapacity<Ptr<'a, R>, M>
    {
        unsafe {
            // Compute the layout of the allocation structure, capturing the
            // offset of its resident field.
            let (layout, offset) = Layout::for_type::<R::Meta>()
                .extended(R::new_resident_layout(cap, &meta)?)?;
            // Allocate a block of memory to hold the allocation structure,
            // bailing on failure.
            let block = hold.alloc(layout)?;
            // Get a pointer to the header field of the new allocation.
            let header = block.as_ptr() as *mut R::Meta;
            // Get a raw pointer to the resident field of the new allocation.
            let field = (header as *mut u8).wrapping_add(offset);
            // Get a fat pointer to the resident field.
            let resident = R::new_resident_ptr(field, cap, &meta);
            // Construct a new Ptr lease.
            let mut lease = Ptr::from_raw(resident);
            // Initialize the new resident.
            R::new_resident(&mut lease, cap, meta);
            // Return the new lease.
            Ok(lease)
        }
    }

    #[inline]
    pub fn try_hold_new<T>(hold: &dyn Hold<'a>, data: T) -> Result<Ptr<'a, R>, HoldError>
        where R: ResidentFromValue<Ptr<'a, R>, T>
    {
        Ptr::try_hold_new_meta(hold, data, ())
    }

    #[inline]
    pub fn try_hold_clone<T: ?Sized>(hold: &dyn Hold<'a>, data: &T) -> Result<Ptr<'a, R>, HoldError>
        where R: ResidentFromClone<Ptr<'a, R>, T>
    {
        Ptr::try_hold_clone_meta(hold, data, ())
    }

    #[inline]
    pub unsafe fn try_hold_clone_unchecked<T: ?Sized>(hold: &dyn Hold<'a>, data: &T) -> Result<Ptr<'a, R>, HoldError>
        where R: ResidentFromCloneUnchecked<Ptr<'a, R>, T>
    {
        Ptr::try_hold_clone_unchecked_meta(hold, data, ())
    }

    #[inline]
    pub fn try_hold_copy<T: ?Sized>(hold: &dyn Hold<'a>, data: &T) -> Result<Ptr<'a, R>, HoldError>
        where R: ResidentFromCopy<Ptr<'a, R>, T>
    {
        Ptr::try_hold_copy_meta(hold, data, ())
    }

    #[inline]
    pub unsafe fn try_hold_copy_unchecked<T: ?Sized>(hold: &dyn Hold<'a>, data: &T) -> Result<Ptr<'a, R>, HoldError>
        where R: ResidentFromCopyUnchecked<Ptr<'a, R>, T>
    {
        Ptr::try_hold_copy_unchecked_meta(hold, data, ())
    }

    #[inline]
    pub fn try_hold_empty(hold: &dyn Hold<'a>) -> Result<Ptr<'a, R>, HoldError>
        where R: ResidentFromEmpty<Ptr<'a, R>>
    {
        Ptr::try_hold_empty_meta(hold, ())
    }

    #[inline]
    pub fn try_hold_cap(hold: &dyn Hold<'a>, cap: usize) -> Result<Ptr<'a, R>, HoldError>
        where R: ResidentWithCapacity<Ptr<'a, R>>
    {
        Ptr::try_hold_cap_meta(hold, cap, ())
    }

    #[inline]
    pub fn hold_new<T>(hold: &dyn Hold<'a>, data: T) -> Ptr<'a, R>
        where R: ResidentFromValue<Ptr<'a, R>, T>
    {
        Ptr::try_hold_new(hold, data).unwrap()
    }

    #[inline]
    pub fn hold_clone<T: ?Sized>(hold: &dyn Hold<'a>, data: &T) -> Ptr<'a, R>
        where R: ResidentFromClone<Ptr<'a, R>, T>
    {
        Ptr::try_hold_clone(hold, data).unwrap()
    }

    #[inline]
    pub unsafe fn hold_clone_unchecked<T: ?Sized>(hold: &dyn Hold<'a>, data: &T) -> Ptr<'a, R>
        where R: ResidentFromCloneUnchecked<Ptr<'a, R>, T>
    {
        Ptr::try_hold_clone_unchecked(hold, data).unwrap()
    }

    #[inline]
    pub fn hold_copy<T: ?Sized>(hold: &dyn Hold<'a>, data: &T) -> Ptr<'a, R>
        where R: ResidentFromCopy<Ptr<'a, R>, T>
    {
        Ptr::try_hold_copy(hold, data).unwrap()
    }

    #[inline]
    pub unsafe fn hold_copy_unchecked<T: ?Sized>(hold: &dyn Hold<'a>, data: &T) -> Ptr<'a, R>
        where R: ResidentFromCopyUnchecked<Ptr<'a, R>, T>
    {
        Ptr::try_hold_copy_unchecked(hold, data).unwrap()
    }

    #[inline]
    pub fn hold_empty(hold: &dyn Hold<'a>) -> Ptr<'a, R>
        where R: ResidentFromEmpty<Ptr<'a, R>>
    {
        Ptr::try_hold_empty(hold).unwrap()
    }

    #[inline]
    pub fn hold_cap(hold: &dyn Hold<'a>, cap: usize) -> Ptr<'a, R>
        where R: ResidentWithCapacity<Ptr<'a, R>>
    {
        Ptr::try_hold_cap(hold, cap).unwrap()
    }

    #[inline]
    pub fn new<T>(data: T) -> Ptr<'a, R>
        where R: ResidentFromValue<Ptr<'a, R>, T>
    {
        Ptr::hold_new(Hold::global(), data)
    }

    #[inline]
    pub fn from_clone<T: ?Sized>(data: &T) -> Ptr<'a, R>
        where R: ResidentFromClone<Ptr<'a, R>, T>
    {
        Ptr::hold_clone(Hold::global(), data)
    }

    #[inline]
    pub unsafe fn from_clone_unchecked<T: ?Sized>(data: &T) -> Ptr<'a, R>
        where R: ResidentFromCloneUnchecked<Ptr<'a, R>, T>
    {
        Ptr::hold_clone_unchecked(Hold::global(), data)
    }

    #[inline]
    pub fn from_copy<T: ?Sized>(data: &T) -> Ptr<'a, R>
        where R: ResidentFromCopy<Ptr<'a, R>, T>
    {
        Ptr::hold_copy(Hold::global(), data)
    }

    #[inline]
    pub unsafe fn from_copy_unchecked<T: ?Sized>(data: &T) -> Ptr<'a, R>
        where R: ResidentFromCopyUnchecked<Ptr<'a, R>, T>
    {
        Ptr::hold_copy_unchecked(Hold::global(), data)
    }

    #[inline]
    pub fn empty() -> Ptr<'a, R>
        where R: ResidentFromEmpty<Ptr<'a, R>>
    {
        Ptr::hold_empty(Hold::global())
    }

    #[inline]
    pub fn with_cap(cap: usize) -> Ptr<'a, R>
        where R: ResidentWithCapacity<Ptr<'a, R>>
    {
        Ptr::hold_cap(Hold::global(), cap)
    }

    /// Constructs a `Ptr` lease from a raw pointer returned by `Ptr::into_raw`.
    #[inline]
    pub unsafe fn from_raw(data: *mut R::Data) -> Ptr<'a, R> {
        Ptr {
            data: NonNull::new_unchecked(data),
            data_lifetime: PhantomData,
            meta_lifetime: PhantomData,
            hold_lifetime: PhantomData,
        }
    }

    /// Returns a pointer to the metadata preceding the resident `data` pointer.
    #[inline]
    pub(crate) fn header(this: &Ptr<'a, R>) -> *mut R::Meta {
        // Get a pointer to the resident data.
        let data = this.data.as_ptr();
        // Get the alignment of the resident.
        let align = mem::align_of_val(unsafe { &*data });
        // Get the offset of the resident data in the allocation by rounding up
        // the size of the metadata to the alignment of the resident.
        let offset = mem::size_of::<R::Meta>()
            .wrapping_add(align).wrapping_sub(1) & !align.wrapping_sub(1);
        // Return a pointer to the metadata by subtracting the resident data's
        // offset in the allocation.
        (data as *mut u8).wrapping_sub(offset) as *mut R::Meta
    }

    /// Returns a reference to the user-provided metadata associated with the
    /// owned resident.
    #[inline]
    pub fn metadata<'b>(this: &'b Ptr<'a, R>) -> &'b R::Meta {
        unsafe { &*Ptr::header(this) }
    }

    /// Returns a mutable reference to the user-provided metadata associated
    /// with the owned resident.
    #[inline]
    pub fn metadata_mut<'b>(this: &'b mut Ptr<'a, R>) -> &'b mut R::Meta {
        unsafe { &mut *Ptr::header(this) }
    }

    /// Converts this `Ptr` lease into a raw pointer to the owned resident.
    /// Use `Ptr::from_raw` to reconstitute the returned pointer back into
    /// a `Ptr` lease.
    ///
    /// # Safety
    ///
    /// A memory leak will occur unless the returned pointer is eventually
    /// converted back into an `Ptr` lease, with the same metadata type,
    /// and dropped.
    #[inline]
    pub unsafe fn into_raw(this: Ptr<'a, R>) -> *mut R::Data {
        let data = this.data.as_ptr();
        mem::forget(this);
        data
    }

    /// Consumes this `Ptr` lease, and returns the owned resident.
    pub fn unwrap(this: Ptr<'a, R>) -> R::Target where R: ResidentUnwrap<Ptr<'a, R>> {
        unsafe {
            // Get a pointer to the owned resident.
            let data = this.data.as_ptr();
            // Get the alignment of the resident.
            let align = mem::align_of_val(&*data);
            // Get the offset of the resident data  in the allocation structure
            // by rounding up the size of the metadata to the alignment of the
            // resident data.
            let offset = mem::size_of::<R::Meta>()
                .wrapping_add(align).wrapping_sub(1) & !align.wrapping_sub(1);
            // Get a pointer to the metadata by subtracting the resident data's
            // offset in the allocation structure.
            let header = (data as *mut u8).wrapping_sub(offset) as *mut R::Meta;
            // Compute the total size of the allocation structure.
            let size = offset.wrapping_add(R::resident_size(data, header));
            // Read the resident out of the allocation structure.
            let resident = R::resident_unwrap(&this);
            // Drop the resident metadata.
            ptr::drop_in_place(header);
            // Get the block of memory containing the allocation structure.
            let block = Block::from_raw_parts(header as *mut u8, size);
            // Deallocate the block.
            AllocTag::from_ptr(header as *mut u8).dealloc(block);
            // Discard the original lease, whose resident we took, and whose
            // memory block we deallocated.
            mem::forget(this);
            // Return the unwrapped resident.
            resident
        }
    }
}

impl<'a, R: Resident> Holder<'a> for Ptr<'a, R> {
    #[inline]
    fn holder(&self) -> &'a dyn Hold<'a> {
        AllocTag::from_ptr(Ptr::header(self) as *mut u8).holder()
    }
}

impl<'a, R: Resident> Lease for Ptr<'a, R> {
    type Data = R::Data;

    type Meta = R::Meta;

    #[inline]
    fn data(&self) -> *mut R::Data {
        self.data.as_ptr()
    }

    #[inline]
    fn meta(&self) -> *mut R::Meta {
        Ptr::header(self)
    }
}

impl<'a, R: Resident> DynamicLease<'a> for Ptr<'a, R> {
    unsafe fn realloc(&mut self, new_layout: Layout) -> Result<(), HoldError> {
        // Get a pointer to the resident data.
        let old_data = self.data.as_ptr();
        // Get the alignment of the resident.
        let align = mem::align_of_val(&*old_data);
        // Compute the layout of the allocation header.
        let header_layout = Layout::for_type::<R::Meta>();
        // Get the offset of the resident data in the allocation structure by
        // rounding up the size of the metadata to the alignment of the
        // resident data.
        let offset = header_layout.size().wrapping_add(align).wrapping_sub(1) & !align.wrapping_sub(1);
        // Get a pointer to the current metadata by subtracting the resident
        // data's offset in the allocation structure.
        let old_meta = (old_data as *mut u8).wrapping_sub(offset) as *mut R::Meta;
        // Compute the total size of the allocation structure.
        let size = offset.wrapping_add(R::resident_size(old_data, old_meta));
        // Extend the allocation header to include the new layout of the resident.
        let new_layout = header_layout.extended(new_layout)?.0;
        // Get the currently leased memory block.
        let old_block = Block::from_raw_parts(old_meta as *mut u8, size);
        // Get a pointer to the hold that allocated the current memory block.
        let hold = AllocTag::from_ptr(old_meta as *mut u8).holder();
        // Reallocate the leased memory block.
        match hold.realloc(old_block, new_layout) {
            // Reallocation succeeded.
            Ok(new_block) => {
                // Get a pointer to the reallocated metadata.
                let new_meta = new_block.as_ptr() as *mut R::Meta;
                // Get a fat pointer to the reallocated resident.
                let new_data = block::set_address(old_data, (new_meta as usize).wrapping_add(offset));
                // Update the lease to point to the reallocated resident.
                self.data = NonNull::new_unchecked(new_data);
                // Return successfully.
                Ok(())
           },
           // Reallocation failed.
           Err(error) => Err(error),
        }
    }

    unsafe fn resize(&mut self, new_layout: Layout) -> Result<(), HoldError> {
        // Get a pointer to the resident data.
        let old_data = self.data.as_ptr();
        // Get the alignment of the resident.
        let align = mem::align_of_val(&*old_data);
        // Compute the layout of the allocation header.
        let header_layout = Layout::for_type::<R::Meta>();
        // Get the offset of the resident data in the allocation structure by
        // rounding up the size of the metadata to the alignment of the
        // resident data.
        let offset = header_layout.size().wrapping_add(align).wrapping_sub(1) & !align.wrapping_sub(1);
        // Get a pointer to the current metadata by subtracting the resident
        // data's offset in the allocation structure.
        let old_meta = (old_data as *mut u8).wrapping_sub(offset) as *mut R::Meta;
        // Compute the total size of the allocation structure.
        let size = offset.wrapping_add(R::resident_size(old_data, old_meta));
        // Extend the allocation header to include the new layout of the resident.
        let new_layout = header_layout.extended(new_layout)?.0;
        // Get the currently leased memory block.
        let old_block = Block::from_raw_parts(old_meta as *mut u8, size);
        // Get a pointer to the hold that allocated the current memory block.
        let hold = AllocTag::from_ptr(old_meta as *mut u8).holder();
        // Reallocate the leased memory block.
        match hold.resize(old_block, new_layout) {
            // Reallocation succeeded.
            Ok(new_block) => {
                // Get a pointer to the reallocated metadata.
                let new_meta = new_block.as_ptr() as *mut R::Meta;
                // Get a fat pointer to the reallocated resident.
                let new_data = block::set_address(old_data, (new_meta as usize).wrapping_add(offset));
                // Update the lease to point to the resized resident.
                self.data = NonNull::new_unchecked(new_data);
                // Return successfully.
                Ok(())
           },
           // Reallocation failed.
           Err(error) => Err(error),
        }
    }
}

impl<'a, R: ResidentDeref<Ptr<'a, R>>> Deref for Ptr<'a, R> {
    type Target = R::Target;

    #[inline]
    fn deref(&self) -> &R::Target {
        R::resident_deref(self)
    }
}

impl<'a, R: ResidentDerefMut<Ptr<'a, R>>> DerefMut for Ptr<'a, R> {
    #[inline]
    fn deref_mut(&mut self) -> &mut R::Target {
        R::resident_deref_mut(self)
    }
}

impl<'a, R: ResidentAsRef<Ptr<'a, R>, T>, T: ?Sized> AsRef<T> for Ptr<'a, R> {
    #[inline]
    fn as_ref(&self) -> &T {
        R::resident_as_ref(self)
    }
}

impl<'a, R: ResidentAsMut<Ptr<'a, R>, T>, T: ?Sized> AsMut<T> for Ptr<'a, R> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        R::resident_as_mut(self)
    }
}

impl<'a, R: ResidentIndex<Ptr<'a, R>, Idx>, Idx> Index<Idx> for Ptr<'a, R> {
    type Output = R::Output;

    #[inline]
    fn index(&self, index: Idx) -> &R::Output {
        R::resident_index(self, index)
    }
}

impl<'a, R: ResidentIndexMut<Ptr<'a, R>, Idx>, Idx> IndexMut<Idx> for Ptr<'a, R> {
    #[inline]
    fn index_mut(&mut self, index: Idx) -> &mut R::Output {
        R::resident_index_mut(self, index)
    }
}

impl<'a, R: ResidentAdd<Ptr<'a, R>, Rhs>, Rhs> Add<Rhs> for Ptr<'a, R> {
    type Output = R::Output;

    #[inline]
    fn add(self, rhs: Rhs) -> R::Output {
        R::resident_add(self, rhs)
    }
}

impl<'a, R: ResidentAddAssign<Ptr<'a, R>, Rhs>, Rhs> AddAssign<Rhs> for Ptr<'a, R> {
    #[inline]
    fn add_assign(&mut self, rhs: Rhs) {
        R::resident_add_assign(self, rhs);
    }
}

impl<'a, R: ResidentIntoIterator<Ptr<'a, R>>> IntoIterator for Ptr<'a, R> {
    type Item = R::Item;

    type IntoIter = R::IntoIter;

    #[inline]
    fn into_iter(self) -> R::IntoIter {
        R::resident_into_iter(self)
    }
}

impl<'a, R: ResidentIntoRefIterator<'a, Ptr<'a, R>>> IntoIterator for &'a Ptr<'a, R> {
    type Item = R::Item;

    type IntoIter = R::IntoIter;

    #[inline]
    fn into_iter(self) -> R::IntoIter {
        R::resident_into_iter(self)
    }
}

impl<'a, R: ResidentIntoMutIterator<'a, Ptr<'a, R>>> IntoIterator for &'a mut Ptr<'a, R> {
    type Item = R::Item;

    type IntoIter = R::IntoIter;

    #[inline]
    fn into_iter(self) -> R::IntoIter {
        R::resident_into_iter(self)
    }
}

impl<'a, R: ResidentPartialEq<Ptr<'a, R>, T>, T: ?Sized> PartialEq<T> for Ptr<'a, R> {
    #[inline]
    fn eq(&self, other: &T) -> bool {
        R::resident_eq(self, other)
    }

    #[inline]
    fn ne(&self, other: &T) -> bool {
        R::resident_ne(self, other)
    }
}

impl<'a, R: ResidentEq<Ptr<'a, R>>> Eq for Ptr<'a, R> {
}

impl<'a, R: ResidentPartialOrd<Ptr<'a, R>, T>, T: ?Sized> PartialOrd<T> for Ptr<'a, R> {
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

impl<'a, R: ResidentOrd<Ptr<'a, R>>> Ord for Ptr<'a, R> {
    #[inline]
    fn cmp(&self, other: &Ptr<'a, R>) -> Ordering {
        R::resident_cmp(self, other)
    }
}

impl<'a, R: ResidentHash<Ptr<'a, R>>> Hash for Ptr<'a, R> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        R::resident_hash(self, state);
    }
}

impl<'a, R: ResidentDisplay<Ptr<'a, R>>> Display for Ptr<'a, R> {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        R::resident_fmt(self, f)
    }
}

impl<'a, R: ResidentDebug<Ptr<'a, R>>> Debug for Ptr<'a, R> {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        R::resident_fmt(self, f)
    }
}

impl<'a, R: Resident> Pointer for Ptr<'a, R> {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Pointer::fmt(&self.data.as_ptr(), f)
    }
}

impl<'a, R: ResidentClone<Ptr<'a, R>, Ptr<'a, R>>> Clone for Ptr<'a, R> {
    fn clone(&self) -> Ptr<'a, R> {
        self.try_clone().unwrap()
    }
}

impl<'a, R: ResidentClone<Ptr<'a, R>, Ptr<'a, R>>> TryClone for Ptr<'a, R> {
    fn try_clone(&self) -> Result<Ptr<'a, R>, HoldError> {
        self.try_clone_into_hold(self.holder())
    }
}

impl<'a, 'b, R: ResidentClone<Ptr<'b, R>, Ptr<'a, R>>> CloneIntoHold<'a, Ptr<'a, R>> for Ptr<'b, R> {
    fn try_clone_into_hold(&self, hold: &Hold<'a>) -> Result<Ptr<'a, R>, HoldError> {
        unsafe {
            // Get a pointer to the source resident data.
            let src_data = self.data.as_ptr();
            // Compute the layout of the destination structure with the
            // preferred memory layout of a clone destination for the resident,
            // capturing the offset of its resident field.
            let (dst_layout, offset) = Layout::for_type::<R::Meta>()
                .extended(R::new_resident_layout(self))?;
            // Allocate a block of memory to hold the destination structure,
            // bailing on failure.
            let dst_block = hold.alloc(dst_layout)?;
            // Get a pointer to the header field of the destination structure.
            let dst_meta = dst_block.as_ptr() as *mut R::Meta;
            // Get a raw pointer to the destination resident.
            let dst_field = (dst_meta as *mut u8).wrapping_add(offset);
            // Get a fat pointer to the destination resident.
            let dst_data = block::set_address(src_data, dst_field as usize);
            // Create a destination lease for the new block, with uninitialized metadata.
            let mut dst = Ptr::from_raw(dst_data);
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

impl<'a, 'b, R: ResidentStow<'b, Ptr<'a, R>, Ptr<'b, R>>> Stow<'b, Ptr<'b, R>> for Ptr<'a, R> {
    unsafe fn stow(src: *mut Ptr<'a, R>, dst: *mut Ptr<'b, R>, hold: &Hold<'b>) -> Result<(), HoldError> {
        // Get a pointer to the source resident data.
        let src_data = (*src).data.as_ptr();
        // Get the size of the resident.
        let size = R::resident_size(src_data, Ptr::header(&*src));
        // Get the memory layout of the allocation structure, capturing the
        // offset of its resident field.
        let (layout, offset) = Layout::for_type::<R::Meta>()
            .extended(Layout::from_size_align_unchecked(size, mem::align_of_val(&*src_data)))?;
        // Allocate a destination memory block to hold the relocated resident,
        // bailing on failure.
        let dst_block = hold.alloc(layout)?;
        // Get a pointer to the header field of the destination structure.
        let dst_meta = dst_block.as_ptr() as *mut R::Meta;
        // Get a raw pointer to the destination resident.
        let dst_field = (dst_meta as *mut u8).wrapping_add(offset);
        // Get a fat pointer to the destination resident.
        let dst_data = block::set_address(src_data, dst_field as usize);

        // Init the resident pointer of the destination lease.
        ptr::write(&mut (*dst).data, NonNull::new_unchecked(dst_data));
        // Try to stow the resident.
        if let err @ Err(_) = R::resident_stow(&mut *src, &mut *dst, hold) {
            // Free the newly allocated structure.
            hold.dealloc(dst_block);
            // Before returning the error.
            return err;
        }
        // Return successfully.
        Ok(())
    }

    unsafe fn unstow(_src: *mut Ptr<'a, R>, _dst: *mut Ptr<'b, R>) {
        panic!("unsupported");
    }
}

impl<'a, 'b, R: ResidentStow<'b, Ptr<'a, R>, Ptr<'b, R>>> StowFrom<'b, Ptr<'a, R>> for Ptr<'b, R> {
    fn try_stow_from(mut src: Ptr<'a, R>, hold: &Hold<'b>) -> Result<Ptr<'b, R>, (Ptr<'a, R>, HoldError)> {
        unsafe {
            let mut dst = mem::uninitialized::<Ptr<'b, R>>();
            if let Err(error) = Stow::stow(&mut src, &mut dst, hold) {
                mem::forget(dst);
                return Err((src, error));
            }
            return Ok(dst);
        }
    }
}

unsafe impl<'a, #[may_dangle] R: Resident> Drop for Ptr<'a, R> {
    fn drop(&mut self) {
        unsafe {
            // Get a pointer to the owned resident.
            let data = self.data.as_ptr();
            // Get the alignment of the resident.
            let align = mem::align_of_val(&*data);
            // Get the offset of the resident data in the allocation structure
            // by rounding up the size of the metadata to the alignment of the
            // resident data.
            let offset = mem::size_of::<R::Meta>()
                .wrapping_add(align).wrapping_sub(1) & !align.wrapping_sub(1);
            // Get a pointer to the allocation header by subtracting the
            // resident data's offset in the allocation structure.
            let header = (data as *mut u8).wrapping_sub(offset) as *mut R::Meta;
            // Compute the total size of the allocation structure.
            let size = offset.wrapping_add(R::resident_size(data, header));
            // Drop the owned resident.
            R::resident_drop(data, header);
            // Drop the resident metadata.
            ptr::drop_in_place(header);
            // Get the block of memory containing the allocation structure.
            let block = Block::from_raw_parts(header as *mut u8, size);
            // Deallocate the block.
            AllocTag::from_ptr(header as *mut u8).dealloc(block);
        }
    }
}
