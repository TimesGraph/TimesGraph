use core::cmp::{self, Ordering};
use core::fmt::{self, Debug, Display, Pointer, Formatter};
use core::hash::{Hash, Hasher};
use core::marker::PhantomData;
use core::mem;
use core::ops::{Deref, Index, Add};
use core::ptr::{self, NonNull};
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering::{Acquire, Relaxed, Release, SeqCst};
use crate::block::{Block, Layout};
use crate::alloc::{AllocTag, Hold, Holder, HoldError, TryClone};
use crate::lease::{arc, ArcHeader, ArcError, Lease, Mut, Hard, Soft};
use crate::resident::{Resident, ResidentFromValue, ResidentFromClone,
                      ResidentFromCloneUnchecked, ResidentFromCopy,
                      ResidentFromCopyUnchecked, ResidentFromEmpty,
                      ResidentWithCapacity, ResidentUnwrap, ResidentDeref,
                      ResidentAsRef, ResidentIndex, ResidentAdd,
                      ResidentIntoIterator, ResidentIntoRefIterator,
                      ResidentPartialEq, ResidentEq, ResidentPartialOrd,
                      ResidentOrd, ResidentHash, ResidentDisplay, ResidentDebug};

/// A thread-safe, atomically counted, immutably dereferenceable hard
/// reference to a `Resident` occupying a shared, `Hold`-allocated memory block.
pub struct Ref<'a, R: Resident> {
    /// Pointer to the resident memory block.
    data: NonNull<R::Data>,
    /// Variant over R::Data, with drop check.
    data_lifetime: PhantomData<R::Data>,
    /// Variant over ArcHeader<R::Meta>, with drop check.
    meta_lifetime: PhantomData<ArcHeader<R::Meta>>,
    /// Variant over 'a.
    hold_lifetime: PhantomData<&'a ()>,
}

unsafe impl<'a, R: Resident> Send for Ref<'a, R> where R::Data: Send, R::Meta: Send {
}

unsafe impl<'a, R: Resident> Sync for Ref<'a, R> where R::Data: Sync, R::Meta: Sync {
}

impl<'a, R: Resident> Ref<'a, R> {
    #[inline]
    pub fn try_hold_new_meta<T, M>(hold: &dyn Hold<'a>, data: T, meta: M)
        -> Result<Ref<'a, R>, HoldError>
        where R: ResidentFromValue<Ref<'a, R>, T, M>
    {
        unsafe {
            // Allocate a new arc structure.
            let resident = arc::alloc_new::<R, Ref<'a, R>, T, M>(hold, &data, &meta, arc::REF_STATUS_INIT)?;
            // Construct a new Ref lease.
            let mut lease = Ref::from_raw(resident);
            // Initialize the new resident.
            R::new_resident(&mut lease, data, meta);
            // Return the new lease.
            Ok(lease)
        }
    }

    #[inline]
    pub fn try_hold_clone_meta<T: ?Sized, M>(hold: &dyn Hold<'a>, data: &T, meta: M)
        -> Result<Ref<'a, R>, HoldError>
        where R: ResidentFromClone<Ref<'a, R>, T, M>
    {
        unsafe {
            // Allocate a new arc structure.
            let resident = arc::alloc_clone::<R, Ref<'a, R>, T, M>(hold, &data, &meta, arc::REF_STATUS_INIT)?;
            // Construct a new Ref lease.
            let mut lease = Ref::from_raw(resident);
            // Initialize the new resident.
            R::new_resident(&mut lease, data, meta);
            // Return the new lease.
            Ok(lease)
        }
    }

    #[inline]
    pub unsafe fn try_hold_clone_unchecked_meta<T: ?Sized, M>(hold: &dyn Hold<'a>, data: &T, meta: M)
        -> Result<Ref<'a, R>, HoldError>
        where R: ResidentFromCloneUnchecked<Ref<'a, R>, T, M>
    {
        // Allocate a new arc structure.
        let resident = arc::alloc_clone_unchecked::<R, Ref<'a, R>, T, M>(hold, &data, &meta, arc::REF_STATUS_INIT)?;
        // Construct a new Ref lease.
        let mut lease = Ref::from_raw(resident);
        // Initialize the new resident.
        R::new_resident(&mut lease, data, meta);
        // Return the new lease.
        Ok(lease)
    }

    #[inline]
    pub fn try_hold_copy_meta<T: ?Sized, M>(hold: &dyn Hold<'a>, data: &T, meta: M)
        -> Result<Ref<'a, R>, HoldError>
        where R: ResidentFromCopy<Ref<'a, R>, T, M>
    {
        unsafe {
            // Allocate a new arc structure.
            let resident = arc::alloc_copy::<R, Ref<'a, R>, T, M>(hold, &data, &meta, arc::REF_STATUS_INIT)?;
            // Construct a new Ref lease.
            let mut lease = Ref::from_raw(resident);
            // Initialize the new resident.
            R::new_resident(&mut lease, data, meta);
            // Return the new lease.
            Ok(lease)
        }
    }

    #[inline]
    pub unsafe fn try_hold_copy_unchecked_meta<T: ?Sized, M>(hold: &dyn Hold<'a>, data: &T, meta: M)
        -> Result<Ref<'a, R>, HoldError>
        where R: ResidentFromCopyUnchecked<Ref<'a, R>, T, M>
    {
        // Allocate a new arc structure.
        let resident = arc::alloc_copy_unchecked::<R, Ref<'a, R>, T, M>(hold, &data, &meta, arc::REF_STATUS_INIT)?;
        // Construct a new Ref lease.
        let mut lease = Ref::from_raw(resident);
        // Initialize the new resident.
        R::new_resident(&mut lease, data, meta);
        // Return the new lease.
        Ok(lease)
    }

    #[inline]
    pub fn try_hold_empty_meta<M>(hold: &dyn Hold<'a>, meta: M)
        -> Result<Ref<'a, R>, HoldError>
        where R: ResidentFromEmpty<Ref<'a, R>, M>
    {
        unsafe {
            // Allocate a new arc structure.
            let resident = arc::alloc_empty::<R, Ref<'a, R>, M>(hold, &meta, arc::REF_STATUS_INIT)?;
            // Construct a new Ref lease.
            let mut lease = Ref::from_raw(resident);
            // Initialize the new resident.
            R::new_resident(&mut lease, meta);
            // Return the new lease.
            Ok(lease)
        }
    }

    #[inline]
    pub fn try_hold_cap_meta<M>(hold: &dyn Hold<'a>, cap: usize, meta: M)
        -> Result<Ref<'a, R>, HoldError>
        where R: ResidentWithCapacity<Ref<'a, R>, M>
    {
        unsafe {
            // Allocate a new arc structure.
            let resident = arc::alloc_cap::<R, Ref<'a, R>, M>(hold, cap, &meta, arc::REF_STATUS_INIT)?;
            // Construct a new Ref lease.
            let mut lease = Ref::from_raw(resident);
            // Initialize the new resident.
            R::new_resident(&mut lease, cap, meta);
            // Return the new lease.
            Ok(lease)
        }
    }

    #[inline]
    pub fn try_hold_new<T>(hold: &dyn Hold<'a>, data: T) -> Result<Ref<'a, R>, HoldError>
        where R: ResidentFromValue<Ref<'a, R>, T>
    {
        Ref::try_hold_new_meta(hold, data, ())
    }

    #[inline]
    pub fn try_hold_clone<T: ?Sized>(hold: &dyn Hold<'a>, data: &T) -> Result<Ref<'a, R>, HoldError>
        where R: ResidentFromClone<Ref<'a, R>, T>
    {
        Ref::try_hold_clone_meta(hold, data, ())
    }

    #[inline]
    pub unsafe fn try_hold_clone_unchecked<T: ?Sized>(hold: &dyn Hold<'a>, data: &T) -> Result<Ref<'a, R>, HoldError>
        where R: ResidentFromCloneUnchecked<Ref<'a, R>, T>
    {
        Ref::try_hold_clone_unchecked_meta(hold, data, ())
    }

    #[inline]
    pub fn try_hold_copy<T: ?Sized>(hold: &dyn Hold<'a>, data: &T) -> Result<Ref<'a, R>, HoldError>
        where R: ResidentFromCopy<Ref<'a, R>, T>
    {
        Ref::try_hold_copy_meta(hold, data, ())
    }

    #[inline]
    pub unsafe fn try_hold_copy_unchecked<T: ?Sized>(hold: &dyn Hold<'a>, data: &T) -> Result<Ref<'a, R>, HoldError>
        where R: ResidentFromCopyUnchecked<Ref<'a, R>, T>
    {
        Ref::try_hold_copy_unchecked_meta(hold, data, ())
    }

    #[inline]
    pub fn try_hold_empty(hold: &dyn Hold<'a>) -> Result<Ref<'a, R>, HoldError>
        where R: ResidentFromEmpty<Ref<'a, R>>
    {
        Ref::try_hold_empty_meta(hold, ())
    }

    #[inline]
    pub fn try_hold_cap(hold: &dyn Hold<'a>, cap: usize) -> Result<Ref<'a, R>, HoldError>
        where R: ResidentWithCapacity<Ref<'a, R>>
    {
        Ref::try_hold_cap_meta(hold, cap, ())
    }

    #[inline]
    pub fn hold_new<T>(hold: &dyn Hold<'a>, data: T) -> Ref<'a, R>
        where R: ResidentFromValue<Ref<'a, R>, T>
    {
        Ref::try_hold_new(hold, data).unwrap()
    }

    #[inline]
    pub fn hold_clone<T: ?Sized>(hold: &dyn Hold<'a>, data: &T) -> Ref<'a, R>
        where R: ResidentFromClone<Ref<'a, R>, T>
    {
        Ref::try_hold_clone(hold, data).unwrap()
    }

    #[inline]
    pub unsafe fn hold_clone_unchecked<T: ?Sized>(hold: &dyn Hold<'a>, data: &T) -> Ref<'a, R>
        where R: ResidentFromCloneUnchecked<Ref<'a, R>, T>
    {
        Ref::try_hold_clone_unchecked(hold, data).unwrap()
    }

    #[inline]
    pub fn hold_copy<T: ?Sized>(hold: &dyn Hold<'a>, data: &T) -> Ref<'a, R>
        where R: ResidentFromCopy<Ref<'a, R>, T>
    {
        Ref::try_hold_copy(hold, data).unwrap()
    }

    #[inline]
    pub unsafe fn hold_copy_unchecked<T: ?Sized>(hold: &dyn Hold<'a>, data: &T) -> Ref<'a, R>
        where R: ResidentFromCopyUnchecked<Ref<'a, R>, T>
    {
        Ref::try_hold_copy_unchecked(hold, data).unwrap()
    }

    #[inline]
    pub fn hold_empty(hold: &dyn Hold<'a>) -> Ref<'a, R>
        where R: ResidentFromEmpty<Ref<'a, R>>
    {
        Ref::try_hold_empty(hold).unwrap()
    }

    #[inline]
    pub fn hold_cap(hold: &dyn Hold<'a>, cap: usize) -> Ref<'a, R>
        where R: ResidentWithCapacity<Ref<'a, R>>
    {
        Ref::try_hold_cap(hold, cap).unwrap()
    }

    #[inline]
    pub fn new<T>(data: T) -> Ref<'a, R>
        where R: ResidentFromValue<Ref<'a, R>, T>
    {
        Ref::hold_new(Hold::global(), data)
    }

    #[inline]
    pub fn from_clone<T: ?Sized>(data: &T) -> Ref<'a, R>
        where R: ResidentFromClone<Ref<'a, R>, T>
    {
        Ref::hold_clone(Hold::global(), data)
    }

    #[inline]
    pub unsafe fn from_clone_unchecked<T: ?Sized>(data: &T) -> Ref<'a, R>
        where R: ResidentFromCloneUnchecked<Ref<'a, R>, T>
    {
        Ref::hold_clone_unchecked(Hold::global(), data)
    }

    #[inline]
    pub fn from_copy<T: ?Sized>(data: &T) -> Ref<'a, R>
        where R: ResidentFromCopy<Ref<'a, R>, T>
    {
        Ref::hold_copy(Hold::global(), data)
    }

    #[inline]
    pub unsafe fn from_copy_unchecked<T: ?Sized>(data: &T) -> Ref<'a, R>
        where R: ResidentFromCopyUnchecked<Ref<'a, R>, T>
    {
        Ref::hold_copy_unchecked(Hold::global(), data)
    }

    #[inline]
    pub fn empty() -> Ref<'a, R>
        where R: ResidentFromEmpty<Ref<'a, R>>
    {
        Ref::hold_empty(Hold::global())
    }

    #[inline]
    pub fn with_cap(cap: usize) -> Ref<'a, R>
        where R: ResidentWithCapacity<Ref<'a, R>>
    {
        Ref::hold_cap(Hold::global(), cap)
    }

    /// Constructs a `Ref` lease from a raw pointer returned by `Ref::into_raw`.
    #[inline]
    pub unsafe fn from_raw(data: *mut R::Data) -> Ref<'a, R> {
        Ref {
            data: NonNull::new_unchecked(data),
            data_lifetime: PhantomData,
            meta_lifetime: PhantomData,
            hold_lifetime: PhantomData,
        }
    }

    /// Returns a pointer to the `ArcHeader` preceding the shared resident.
    #[inline]
    fn header(this: &Ref<'a, R>) -> *mut ArcHeader<R::Meta> {
        arc::header::<R>(this.data.as_ptr())
    }

    /// Returns the number of hard references to the shared resident.
    #[inline]
    pub fn hard_count(this: &Ref<'a, R>) -> usize {
        unsafe { (*Ref::header(this)).hard_count() }
    }

    /// Returns the number of soft references to the shared resident.
    #[inline]
    pub fn soft_count(this: &Ref<'a, R>) -> usize {
        unsafe { (*Ref::header(this)).soft_count() }
    }

    /// Returns the number of immutable references to the shared resident.
    #[inline]
    pub fn ref_count(this: &Ref<'a, R>) -> usize {
        unsafe { (*Ref::header(this)).ref_count() }
    }

    /// Returns a reference to the user-provided metadata associated with the
    /// shared resident.
    #[inline]
    pub fn metadata<'b>(this: &'b Ref<'a, R>) -> &'b R::Meta {
        unsafe { &(*Ref::header(this)).meta }
    }

    /// Returns a mutable lease to a clone of the shared resident, returning
    /// an error on allocation failure.
    pub fn try_to_unique(this: &Ref<'a, R>) -> Result<Mut<'a, R>, ArcError>
        where R::Data: TryClone,
              R::Meta: TryClone,
    {
        unsafe {
            // Get a pointer to the shared resident.
            let old_data = this.data.as_ptr();
            // Get the alignment of the resident.
            let align = mem::align_of_val(&*old_data);
            // Get the offset of the resident in the arc structure by rounding up
            // the size of the arc header to the alignment of the resident.
            let offset = mem::size_of::<ArcHeader<R::Meta>>()
                .wrapping_add(align).wrapping_sub(1) & !align.wrapping_sub(1);
            // Get a pointer to the shared header by subtracting the resident's
            // offset in the arc structure.
            let old_header = (old_data as *mut u8).wrapping_sub(offset) as *mut ArcHeader<R::Meta>;
            // Compute the total size of the arc structure.
            let size = offset.wrapping_add(R::resident_size(old_data, &mut (*old_header).meta));
            // Compute the layout of the arc structure.
            let layout = Layout::from_size_align_unchecked(size, cmp::max(align, mem::align_of::<ArcHeader<R::Meta>>()));
            // Get a reference to the hold that allocated the original arc.
            let hold = this.holder();
            // Allocate a block of memory to hold the new arc structure, bailing on failure.
            let new_block = hold.alloc(layout)?;
            // Get a pointer to the header field of the new arc.
            let new_header = new_block.as_ptr() as *mut ArcHeader<R::Meta>;
            // Initialize the new relocation address to zero.
            ptr::write(&mut (*new_header).relocation, AtomicUsize::new(0));
            // Initialize the lease status field.
            ptr::write(&mut (*new_header).status, AtomicUsize::new(arc::MUT_STATUS_INIT));
            // Try to clone the metadata.
            let new_metadata = match (*old_header).meta.try_clone() {
                // Clone succeeded.
                Ok(metadata) => metadata,
                // Clone failed.
                Err(error) => {
                    // Free the newly allocated arc.
                    hold.dealloc(new_block);
                    // Return the allocation error.
                    return Err(ArcError::from(error));
                },
            };
            // Move the cloned metadata into the new arc header.
            ptr::write(&mut (*new_header).meta, new_metadata);
            // Get a pointer to the new resident.
            let new_data = (new_header as *mut u8).wrapping_add(offset) as *mut R::Data;
            // Try to clone the resident.
            let new_resident = match (*old_data).try_clone() {
                // Clone succeeded.
                Ok(resident) => resident,
                // Clone failed.
                Err(error) => {
                    // Drop the cloned metadata.
                    ptr::drop_in_place(&mut (*new_header).meta);
                    // Free the newly allocated arc.
                    hold.dealloc(new_block);
                    // Return the allocation error.
                    return Err(ArcError::from(error));
                },
            };
            // Move the cloned resident into the new arc.
            ptr::write(new_data, new_resident);
            // Return a new Mut lease with a pointer to the cloned resident.
            Ok(Mut::from_raw(new_data))
        }
    }

    /// Returns a mutable lease to a clone of the shared resident
    ///
    /// # Panics
    ///
    /// Panics on allocation failure.
    pub fn to_unique(this: &Ref<'a, R>) -> Mut<'a, R>
        where R::Data: TryClone,
              R::Meta: TryClone,
    {
        Ref::try_to_unique(this).unwrap()
    }

    /// Converts this immutable lease into a mutable lease to the shared
    /// resident, cloning the resident if there are any outstanding mutable
    /// or immutable leases, and returning an error on allocation failure.
    pub fn try_into_unique(this: Ref<'a, R>) -> Result<Mut<'a, R>, ArcError>
        where R::Data: TryClone,
              R::Meta: TryClone,
    {
        unsafe {
            // Get a pointer to the shared resident.
            let old_data = this.data.as_ptr();
            // Get the alignment of the resident.
            let align = mem::align_of_val(&*old_data);
            // Get the offset of the resident in the arc structure by rounding up
            // the size of the arc header to the alignment of the resident.
            let offset = mem::size_of::<ArcHeader<R::Meta>>()
                .wrapping_add(align).wrapping_sub(1) & !align.wrapping_sub(1);
            // Get a pointer to the shared header by subtracting the resident's
            // offset in the arc structure.
            let old_header = (old_data as *mut u8).wrapping_sub(offset) as *mut ArcHeader<R::Meta>;
            // Load the status field; synchronized by subsequent CAS.
            let mut old_status = (*old_header).status.load(Relaxed);
            // Check if unique, and traverse moves.
            loop {
                // Extract the immutable reference count from the status field.
                let old_ref_count = (old_status & arc::REF_COUNT_MASK) >> arc::REF_COUNT_SHIFT;
                // Check if this is the only immutable reference.
                if old_ref_count == 1 {
                    // Clear the immutable reference count bit field, and set the mut flag.
                    let new_status = old_status & !arc::REF_COUNT_MASK | arc::MUT_FLAG;
                    // Atomically update the status field, synchronizing with reference acquires and releases.
                    match (*old_header).status.compare_exchange_weak(old_status, new_status, SeqCst, Relaxed) {
                        // CAS succeeded.
                        Ok(_) => {
                            // Discard the original lease, whose hard reference we took,
                            // and whose immutable reference we released.
                            mem::forget(this);
                            // Return a new Mut lease.
                            return Ok(Mut::from_raw(old_data));
                        },
                        // CAS failed; update the status field and try again.
                        Err(status) => old_status = status,
                    }
                } else {
                    // Clone the aliased resident.
                    break;
                }
            }
            // Compute the total size of the arc structure.
            let size = offset.wrapping_add(R::resident_size(old_data, &mut (*old_header).meta));
            // Compute the layout of the arc structure.
            let layout = Layout::from_size_align_unchecked(size, cmp::max(align, mem::align_of::<ArcHeader<R::Meta>>()));
            // Get a reference to the hold that allocated the original arc.
            let hold = this.holder();
            // Allocate a block of memory to hold the new arc structure, bailing on failure.
            let new_block = hold.alloc(layout)?;
            // Get a pointer to the header field of the new arc.
            let new_header = new_block.as_ptr() as *mut ArcHeader<R::Meta>;
            // Initialize the new relocation address to zero.
            ptr::write(&mut (*new_header).relocation, AtomicUsize::new(0));
            // Initialize the new lease status field.
            ptr::write(&mut (*new_header).status, AtomicUsize::new(arc::MUT_STATUS_INIT));
            // Try to clone the metadata.
            let new_metadata = match (*old_header).meta.try_clone() {
                // Clone succeeded.
                Ok(metadata) => metadata,
                // Clone failed.
                Err(error) => {
                    // Free the newly allocated arc.
                    hold.dealloc(new_block);
                    // Return the allocation error.
                    return Err(ArcError::from(error));
                },
            };
            // Move the cloned metadata into the new arc header.
            ptr::write(&mut (*new_header).meta, new_metadata);
            // Get a pointer to the new resident.
            let new_data = (new_header as *mut u8).wrapping_add(offset) as *mut R::Data;
            // Try to clone the resident.
            let new_resident = match (*old_data).try_clone() {
                // Clone succeeded.
                Ok(resident) => resident,
                // Clone failed.
                Err(error) => {
                    // Drop the cloned metadata.
                    ptr::drop_in_place(&mut (*new_header).meta);
                    // Free the newly allocated arc.
                    hold.dealloc(new_block);
                    // Return the allocation error.
                    return Err(ArcError::from(error));
                },
            };
            // Move the cloned resident into the new arc.
            ptr::write(new_data, new_resident);
            // Return a new Mut lease with a pointer to the cloned resident.
            Ok(Mut::from_raw(new_data))
        }
    }

    /// Converts this immutable lease into a mutable lease to the shared
    /// resident, cloning the resident if there are any outstanding mutable
    /// or immutable leases.
    ///
    /// # Panics
    ///
    /// Panics on allocation failure.
    pub fn into_unique(this: Ref<'a, R>) -> Mut<'a, R>
        where R::Data: TryClone,
              R::Meta: TryClone,
    {
        Ref::try_into_unique(this).unwrap()
    }

    /// Converts this immutable lease into a mutable lease to the shared resident,
    /// waiting for any outstanding mutable or immutable leases to drop.
    ///
    /// # Safety
    ///
    /// Deadlocks if the current thread already holds a mutable or immutable
    /// lease to the shared resident. Can cause a future deadlock if a thread
    /// holding a mutable lease to a resident attempts to convert another hard
    /// or soft lease to the same resident into a mutable or immutable lease.
    pub unsafe fn into_mut(this: Ref<'a, R>) -> Mut<'a, R> {
        // Get a pointer to the shared resident.
        let data = this.data.as_ptr();
        // Get a pointer to the arc header preceding the resident.
        let header = arc::header::<R>(data);
        // Load the status field; synchronized by subsequent CAS.
        let mut old_status = (*header).status.load(Relaxed);
        // Spin until a mutable reference is acquired.
        loop {
            // Extract the immutable reference count from the status field.
            let old_ref_count = (old_status & arc::REF_COUNT_MASK) >> arc::REF_COUNT_SHIFT;
            // Check if the shared resident has no other immutable references.
            if old_ref_count == 1 {
                // Clear the immutable reference count bit field, and set the mut flag.
                let new_status = old_status & !arc::REF_COUNT_MASK | arc::MUT_FLAG;
                // Atomically update the status field, synchrnozing with reference acquires and releases.
                match (*header).status.compare_exchange_weak(old_status, new_status, SeqCst, Relaxed) {
                    // CAS succeeded.
                    Ok(_) => {
                        // Discard the original lease, whose hard reference we took,
                        // and whose immutable reference we released.
                        mem::forget(this);
                        // Return a new Mut lease.
                        return Mut::from_raw(data);
                    },
                    // CAS failed; update the status field and try again.
                    Err(status) => old_status = status,
                }
            } else {
                // Resident is aliased; reload the status field and spin.
                old_status = (*header).status.load(Relaxed);
            }
        }
    }

    /// Returns a new hard lease to the shared resident, returning an error
    /// if the incremented hard reference count overflows `HARD_COUNT_MAX`.
    pub fn try_to_hard(this: &Ref<'a, R>) -> Result<Hard<'a, R>, ArcError> {
        unsafe {
            // Get a pointer to the shared resident.
            let data = this.data.as_ptr();
            // Get a pointer to the arc header preceding the resident.
            let header = arc::header::<R>(data);
            // Load the status field; synchronized by subsequent CAS.
            let mut old_status = (*header).status.load(Relaxed);
            // Spin until a hard reference is acquired.
            loop {
                // Extract the hard reference count from the status field.
                let old_hard_count = old_status & arc::HARD_COUNT_MASK;
                // Increment the hard reference count.
                let new_hard_count = old_hard_count.wrapping_add(1);
                // Check if the incremented hard reference count overflows its bit field.
                if new_hard_count > arc::HARD_COUNT_MAX {
                    return Err(ArcError::HardCountOverflow);
                }
                // Clear the hard reference count bit field.
                let new_status = old_status & !arc::HARD_COUNT_MASK;
                // Splice the incremented hard reference count into the status field.
                let new_status = new_status | new_hard_count;
                // Atomically update the status field, synchronizing with reference releases.
                match (*header).status.compare_exchange_weak(old_status, new_status, Acquire, Relaxed) {
                    // CAS succeeded; return a new Hard lease.
                    Ok(_) => return Ok(Hard::from_raw(data)),
                    // CAS failed; update the status field and try again.
                    Err(status) => old_status = status,
                }
            }
        }
    }

    /// Returns a new hard lease to the shared resident.
    ///
    /// # Panics
    ///
    /// Panics if the incremented hard reference count overflows `HARD_COUNT_MAX`.
    pub fn to_hard(this: &Ref<'a, R>) -> Hard<'a, R> {
        Ref::try_to_hard(this).unwrap()
    }

    /// Converts this immutable lease into a hard lease.
    pub fn into_hard(this: Ref<'a, R>) -> Hard<'a, R> {
        unsafe {
            // Get a pointer to the shared resident.
            let data = this.data.as_ptr();
            // Get a pointer to the arc header preceding the resident.
            let header = arc::header::<R>(data);
            // Load the status field; synchronized by subsequent CAS.
            let mut old_status = (*header).status.load(Relaxed);
            // Spin until the immutable reference is released.
            loop {
                // Extract the immutable reference count from the status field.
                let old_ref_count = (old_status & arc::REF_COUNT_MASK) >> arc::REF_COUNT_SHIFT;
                // Decrement the immutable reference count, checking for underflow.
                let new_ref_count = match old_ref_count.checked_sub(1) {
                    Some(ref_count) => ref_count,
                    None => panic!("ref count underflow"),
                };
                // Clear the immutable reference count field.
                let new_status = old_status & !arc::REF_COUNT_MASK;
                // Splice the decremented immutable reference count into the status field.
                let new_status = new_status | new_ref_count << arc::REF_COUNT_SHIFT;
                // Atomically update the status field, synchronizing with reference acquires.
                match (*header).status.compare_exchange_weak(old_status, new_status, Release, Relaxed) {
                    // CAS succeeded.
                    Ok(_) => {
                        // Discard the original lease, whose hard reference we took,
                        // and whose immutable reference we released.
                        mem::forget(this);
                        // Return a new Hard lease.
                        return Hard::from_raw(data);
                    },
                    // CAS failed; update the status field and try again.
                    Err(status) => old_status = status,
                }
            }
        }
    }

    /// Returns a new soft reference to the shared resident, returning an error
    /// if the incremented soft reference count overflows `SOFT_COUNT_MAX`.
    pub fn try_to_soft(this: &Ref<'a, R>) -> Result<Soft<'a, R>, ArcError> {
        unsafe {
            // Get a pointer to the shared resident.
            let data = this.data.as_ptr();
            // Get a pointer to the arc header preceding the resident.
            let header = arc::header::<R>(data);
            // Load the status field; synchronized by subsequent CAS.
            let mut old_status = (*header).status.load(Relaxed);
            // Spin until a soft reference is acquired.
            loop {
                // Extract the soft reference count from the status field.
                let old_soft_count = (old_status & arc::SOFT_COUNT_MASK) >> arc::SOFT_COUNT_SHIFT;
                // Increment the soft reference count.
                let new_soft_count = old_soft_count.wrapping_add(1);
                // Check if the incremented soft reference count overflows its bit field.
                if new_soft_count > arc::SOFT_COUNT_MAX {
                    return Err(ArcError::SoftCountOverflow);
                }
                // Clear the soft reference count field.
                let new_status = old_status & !arc::SOFT_COUNT_MASK;
                // Splice the incremented soft reference count into the status field.
                let new_status = new_status | new_soft_count << arc::SOFT_COUNT_SHIFT;
                // Atomically update the status field, synchronizing with reference releases.
                match (*header).status.compare_exchange_weak(old_status, new_status, Acquire, Relaxed) {
                    // CAS succeeded; return a new Soft lease.
                    Ok(_) => return Ok(Soft::from_raw(data)),
                    // CAS failed; update the status field and try again.
                    Err(status) => old_status = status,
                }
            }
        }
    }

    /// Returns a new soft reference to the shared resident.
    ///
    /// # Panics
    ///
    /// Panics if the incremented soft reference count overflows `SOFT_COUNT_MAX`.
    pub fn to_soft(this: &Ref<'a, R>) -> Soft<'a, R> {
        Ref::try_to_soft(this).unwrap()
    }

    /// Converts this immutable lease into a soft lease to the shared resident,
    /// returning an error if the incremented soft reference count overflows
    /// `SOFT_COUNT_MAX`.
    pub fn try_into_soft(this: Ref<'a, R>) -> Result<Soft<'a, R>, ArcError> {
        unsafe {
            // Get a pointer to the shared resident.
            let data = this.data.as_ptr();
            // Get a pointer to the arc header preceding the resident.
            let header = arc::header::<R>(data);
            // Load the status field; synchronized by subsequent CAS.
            let mut old_status = (*header).status.load(Relaxed);
            // Spin until a soft reference is acquired.
            loop {
                // Extract the hard reference count from the status field.
                let old_hard_count = old_status & arc::HARD_COUNT_MASK;
                // Decrement the hard reference count, checking for underflow.
                let new_hard_count = match old_hard_count.checked_sub(1) {
                    Some(hard_count) => hard_count,
                    None => panic!("hard count underflow"),
                };
                // Extract the soft reference count from the status field.
                let old_soft_count = (old_status & arc::SOFT_COUNT_MASK) >> arc::SOFT_COUNT_SHIFT;
                // Increment the soft reference count.
                let new_soft_count = old_soft_count.wrapping_add(1);
                // Check if the incremented soft reference count overflows its bit field.
                if new_soft_count > arc::SOFT_COUNT_MAX {
                    return Err(ArcError::SoftCountOverflow);
                }
                // Extract the immutable reference count from the status field.
                let old_ref_count = (old_status & arc::REF_COUNT_MASK) >> arc::REF_COUNT_SHIFT;
                // Decrement the immutable reference count, checking for underflow.
                let new_ref_count = match old_ref_count.checked_sub(1) {
                    Some(ref_count) => ref_count,
                    None => panic!("ref count underflow"),
                };
                // Clear the hard, soft, and immutable reference count bit fields.
                let new_status = old_status & !(arc::HARD_COUNT_MASK | arc::SOFT_COUNT_MASK | arc::REF_COUNT_MASK);
                // Splice the decremented hard, incremented soft, and decremented immutable reference counts
                // into the status field.
                let new_status = new_status | new_hard_count | new_soft_count << arc::SOFT_COUNT_SHIFT |
                                 new_ref_count << arc::REF_COUNT_SHIFT;
                // Atomically update the status field; synchronizing with reference acquires and releases.
                match (*header).status.compare_exchange_weak(old_status, new_status, SeqCst, Relaxed) {
                    // CAS succeeded.
                    Ok(_) => {
                        // Check if the hard count dropped to zero.
                        if new_hard_count == 0 {
                            // Drop the shared resident.
                            R::resident_drop(data, &mut (*header).meta);
                        }
                        // Discard the original lease, whose hard and immutable references we released.
                        mem::forget(this);
                        // Return a new Soft lease.
                        return Ok(Soft::from_raw(data));
                    },
                    // CAS failed; update the status field and try again.
                    Err(status) => old_status = status,
                }
            }
        }
    }

    /// Converts this immutable lease into a soft lease to the shared resident.
    ///
    /// # Panics
    ///
    /// Panics if the incremented soft reference count overflows `SOFT_COUNT_MAX`.
    pub fn into_soft(this: Ref<'a, R>) -> Soft<'a, R> {
        Ref::try_into_soft(this).unwrap()
    }

    /// Converts this immutable lease into a raw pointer to the shared resident.
    /// Use `Ref::from_raw` to reconstitute the returned pointer back into
    /// an immutable lease.
    ///
    /// # Safety
    ///
    /// A memory leak will occur unless the returned pointer is eventually
    /// converted back into an immutable lease and dropped.
    #[inline]
    pub unsafe fn into_raw(this: Ref<'a, R>) -> *mut R::Data {
        let data = this.data.as_ptr();
        mem::forget(this);
        data
    }

    /// Returns a raw pointer to the shared resident.
    ///
    /// # Safety
    ///
    /// The shared resident may be uninitialized, or mutably aliased.
    #[inline]
    pub unsafe fn as_ptr_unchecked(this: &Ref<'a, R>) -> *mut R::Data {
        this.data.as_ptr()
    }

    /// Consumes this immutable lease, and returns the shared resident;
    /// returns an `Err` containing the original lease if any outstanding
    /// hard or immutable leases prevent the resident from being moved.
    pub fn try_unwrap(this: Ref<'a, R>) -> Result<R::Target, Ref<'a, R>> where R: ResidentUnwrap<Ref<'a, R>> {
        unsafe {
            // Get a pointer to the shared resident.
            let data = this.data.as_ptr();
            // Get the alignment of the resident.
            let align = mem::align_of_val(&*data);
            // Get the offset of the resident in the arc structure by rounding up
            // the size of the arc header to the alignment of the resident.
            let offset = mem::size_of::<ArcHeader<R::Meta>>()
                .wrapping_add(align).wrapping_sub(1) & !align.wrapping_sub(1);
            // Get a pointer to the arc header by subtracting the resident's
            // offset in the arc structure.
            let header = (data as *mut u8).wrapping_sub(offset) as *mut ArcHeader<R::Meta>;
            // Compute the total size of the arc structure.
            let size = offset.wrapping_add(R::resident_size(data, &mut (*header).meta));
            // Load the status field; synchronized by subsequent CAS.
            let mut old_status = (*header).status.load(Relaxed);
            // Spin until the hard and immutable references are released.
            loop {
                // Extract the hard reference count from the status field.
                let old_hard_count = old_status & arc::HARD_COUNT_MASK;
                // Check if the shared resident has multiple hard references.
                if old_hard_count != 1 {
                    // Can't unwrap an aliased resident.
                    return Err(this);
                }
                // The immutable reference count must also be singular.
                debug_assert_eq!((old_status &arc::REF_COUNT_MASK) >> arc::REF_COUNT_SHIFT, 1);
                // Clear the hard and immutable reference count bit fields.
                let new_status = old_status & !(arc::HARD_COUNT_MASK | arc::REF_COUNT_MASK);
                // Extract the soft reference count from the status field.
                let old_soft_count = (old_status & arc::SOFT_COUNT_MASK) >> arc::SOFT_COUNT_SHIFT;
                // Check if all soft references have dropped.
                if old_soft_count == 0 {
                    // Store the new status field; can't fail because we're the last reference of any kind.
                    (*header).status.store(new_status, Relaxed);
                    // Read the resident out of the arc structure.
                    let resident = R::resident_unwrap(&this);
                    // Drop the arc header.
                    (*header).drop::<R>(data);
                    // Get the block of memory containing the arc structure.
                    let block = Block::from_raw_parts(header as *mut u8, size);
                    // Deallocate the block.
                    AllocTag::from_ptr(header as *mut u8).dealloc(block);
                    // Discard the original lease, whose hard and immutable reference we released.
                    mem::forget(this);
                    // Return the unwrapped resident.
                    return Ok(resident);
                } else {
                    // Convert our hard reference into a soft reference to avoid racing with other soft refs.
                    let new_soft_count = old_soft_count.wrapping_add(1);
                    // Check if the incremented soft reference count overflows its bit field.
                    if new_soft_count > arc::SOFT_COUNT_MAX {
                        return Err(this);
                    }
                    // Clear the soft reference count bit field.
                    let new_status = new_status & !arc::SOFT_COUNT_MASK;
                    // Splice the incremented soft reference count into the status field.
                    let new_status = new_status | new_soft_count << arc::SOFT_COUNT_SHIFT;
                    // Atomically update the status field, synchronizing with reference releases.
                    match (*header).status.compare_exchange_weak(old_status, new_status, Acquire, Relaxed) {
                        // CAS succeeded; the last hard reference has been released.
                        Ok(_) => {
                            // Read the resident out of the arc structure.
                            let resident = R::resident_unwrap(&this);
                            // Update the status field.
                            old_status = new_status;
                            // Spin until the soft reference has been released.
                            loop {
                                // Extract the soft reference count from the status field.
                                let old_soft_count = (old_status & arc::SOFT_COUNT_MASK) >> arc::SOFT_COUNT_SHIFT;
                                // Decrement the soft reference count, checking for underflow.
                                let new_soft_count = match old_soft_count.checked_sub(1) {
                                    Some(soft_count) => soft_count,
                                    None => panic!("soft count underflow"),
                                };
                                // Clear the soft reference count bit field.
                                let new_status = old_status & !arc::SOFT_COUNT_MASK;
                                // Splice the decremented soft reference count into the status field.
                                let new_status = new_status | new_soft_count << arc::SOFT_COUNT_SHIFT;
                                // Atomically update the status field, synchronizing with reference acquires.
                                match (*header).status.compare_exchange_weak(old_status, new_status, Release, Relaxed) {
                                    // CAS succeeded.
                                    Ok(_) => {
                                        // Check if all soft references have been released.
                                        if new_soft_count == 0 {
                                            // Drop the arc header.
                                            (*header).drop::<R>(data);
                                            // Get the block of memory containing the arc structure.
                                            let block = Block::from_raw_parts(header as *mut u8, size);
                                            // Deallocate the block.
                                            AllocTag::from_ptr(header as *mut u8).dealloc(block);
                                        }
                                        // Return the unwrapped resident.
                                        return Ok(resident);
                                    },
                                    // CAS failed; update the status field and try again.
                                    Err(status) => old_status = status,
                                }
                            }
                        },
                        // CAS failed; update the status field and try again.
                        Err(status) => old_status = status,
                    }
                }
            }
        }
    }

    /// Consumes this immutable lease, and returns the shared resident.
    ///
    /// # Panics
    ///
    /// Panics if any outstanding hard or immutable leases prevent the resident
    /// from being moved.
    pub fn unwrap(this: Ref<'a, R>) -> R::Target where R: ResidentUnwrap<Ref<'a, R>> {
        match Ref::try_unwrap(this) {
            Ok(resident) => resident,
            Err(_) => panic!("aliased resident"),
        }
    }
}

impl<'a, R: Resident> Holder<'a> for Ref<'a, R> {
    #[inline]
    fn holder(&self) -> &'a dyn Hold<'a> {
        AllocTag::from_ptr(Ref::header(self) as *mut u8).holder()
    }
}

impl<'a, R: Resident> Lease for Ref<'a, R> {
    type Data = R::Data;

    type Meta = R::Meta;

    #[inline]
    fn data(&self) -> *mut R::Data {
        self.data.as_ptr()
    }

    #[inline]
    fn meta(&self) -> *mut R::Meta {
        unsafe { &mut (*Ref::header(self)).meta }
    }
}

impl<'a, R: ResidentDeref<Ref<'a, R>>> Deref for Ref<'a, R> {
    type Target = R::Target;

    #[inline]
    fn deref(&self) -> &R::Target {
        R::resident_deref(self)
    }
}

impl<'a, R: ResidentAsRef<Ref<'a, R>, T>, T: ?Sized> AsRef<T> for Ref<'a, R> {
    #[inline]
    fn as_ref(&self) -> &T {
        R::resident_as_ref(self)
    }
}

impl<'a, R: ResidentIndex<Ref<'a, R>, Idx>, Idx> Index<Idx> for Ref<'a, R> {
    type Output = R::Output;

    #[inline]
    fn index(&self, index: Idx) -> &R::Output {
        R::resident_index(self, index)
    }
}

impl<'a, R: ResidentAdd<Ref<'a, R>, Rhs>, Rhs> Add<Rhs> for Ref<'a, R> {
    type Output = R::Output;

    #[inline]
    fn add(self, rhs: Rhs) -> R::Output {
        R::resident_add(self, rhs)
    }
}

impl<'a, R: ResidentIntoIterator<Ref<'a, R>>> IntoIterator for Ref<'a, R> {
    type Item = R::Item;

    type IntoIter = R::IntoIter;

    #[inline]
    fn into_iter(self) -> R::IntoIter {
        R::resident_into_iter(self)
    }
}

impl<'a, R: ResidentIntoRefIterator<'a, Ref<'a, R>>> IntoIterator for &'a Ref<'a, R> {
    type Item = R::Item;

    type IntoIter = R::IntoIter;

    #[inline]
    fn into_iter(self) -> R::IntoIter {
        R::resident_into_iter(self)
    }
}

impl<'a, R: ResidentPartialEq<Ref<'a, R>, T>, T: ?Sized> PartialEq<T> for Ref<'a, R> {
    #[inline]
    fn eq(&self, other: &T) -> bool {
        R::resident_eq(self, other)
    }

    #[inline]
    fn ne(&self, other: &T) -> bool {
        R::resident_ne(self, other)
    }
}

impl<'a, R: ResidentEq<Ref<'a, R>>> Eq for Ref<'a, R> {
}

impl<'a, R: ResidentPartialOrd<Ref<'a, R>, T>, T: ?Sized> PartialOrd<T> for Ref<'a, R> {
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

impl<'a, R: ResidentOrd<Ref<'a, R>>> Ord for Ref<'a, R> {
    #[inline]
    fn cmp(&self, other: &Ref<'a, R>) -> Ordering {
        R::resident_cmp(self, other)
    }
}

impl<'a, R: ResidentHash<Ref<'a, R>>> Hash for Ref<'a, R> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        R::resident_hash(self, state);
    }
}

impl<'a, R: ResidentDisplay<Ref<'a, R>>> Display for Ref<'a, R> {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        R::resident_fmt(self, f)
    }
}

impl<'a, R: ResidentDebug<Ref<'a, R>>> Debug for Ref<'a, R> {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        R::resident_fmt(self, f)
    }
}

impl<'a, R: Resident> Pointer for Ref<'a, R> {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Pointer::fmt(&self.data.as_ptr(), f)
    }
}

impl<'a, R: Resident> TryClone for Ref<'a, R> {
    fn try_clone(&self) -> Result<Ref<'a, R>, HoldError> {
        unsafe {
            // Get a pointer to the shared resident.
            let data = self.data.as_ptr();
            // Get a pointer to the arc header preceding the resident.
            let header = arc::header::<R>(data);
            // Load the status field; synchronized by subsequent CAS.
            let mut old_status = (*header).status.load(Relaxed);
            // Spin until a hard and immutable reference is acquired.
            loop {
                // Extract the hard reference count from the status field.
                let old_hard_count = old_status & arc::HARD_COUNT_MASK;
                // Increment the hard reference count.
                let new_hard_count = old_hard_count.wrapping_add(1);
                // Check if the incremented hard reference count overflows its bit field.
                if new_hard_count > arc::HARD_COUNT_MAX {
                    return Err(HoldError::Unsupported("hard count overflow"));
                }
                // Extract the immutable reference count from the status field.
                let old_ref_count = (old_status & arc::REF_COUNT_MASK) >> arc::REF_COUNT_SHIFT;
                // Increment the immutable reference count.
                let new_ref_count = old_ref_count.wrapping_add(1);
                // Check if the incremented immutable reference count overflows its field.
                if new_ref_count > arc::REF_COUNT_MAX {
                    return Err(HoldError::Unsupported("ref count overflow"));
                }
                // Clear the hard and immutable reference count bit fields.
                let new_status = old_status & !(arc::HARD_COUNT_MASK | arc::REF_COUNT_MASK);
                // Splice the incremented hard and immutable reference counts into the status field.
                let new_status = new_status | new_hard_count | new_ref_count << arc::REF_COUNT_SHIFT;
                // Atomically update the status field, synchronizing with reference releases.
                match (*header).status.compare_exchange_weak(old_status, new_status, Acquire, Relaxed) {
                    // CAS succeeded; return a new Ref lease.
                    Ok(_) => return Ok(Ref::from_raw(data)),
                    // CAS failed; update the status field and try again.
                    Err(status) => old_status = status,
                }
            }
        }
    }
}

impl<'a, R: Resident> Clone for Ref<'a, R> {
    fn clone(&self) -> Ref<'a, R> {
        self.try_clone().unwrap()
    }
}

unsafe impl<'a, #[may_dangle] R: Resident> Drop for Ref<'a, R> {
    fn drop(&mut self) {
        unsafe {
            // Get a pointer to the shared resident.
            let data = self.data.as_ptr();
            // Get the alignment of the resident.
            let align = mem::align_of_val(&*data);
            // Get the offset of the resident in the arc structure by rounding up
            // the size of the arc header to the alignment of the resident.
            let offset = mem::size_of::<ArcHeader<R::Meta>>()
                .wrapping_add(align).wrapping_sub(1) & !align.wrapping_sub(1);
            // Get a pointer to the arc header by subtracting the resident's
            // offset in the arc structure.
            let header = (data as *mut u8).wrapping_sub(offset) as *mut ArcHeader<R::Meta>;
            // Compute the total size of the arc structure.
            let size = offset.wrapping_add(R::resident_size(data, &mut (*header).meta));
            // Load the status field; synchronized by subsequent CAS.
            let mut old_status = (*header).status.load(Relaxed);
            // Spin until the hard and immutable references have been released.
            loop {
                // Extract the hard reference count from the status field.
                let old_hard_count = old_status & arc::HARD_COUNT_MASK;
                // Decrement the hard reference count, checking for underflow.
                let new_hard_count = match old_hard_count.checked_sub(1) {
                    Some(hard_count) => hard_count,
                    None => panic!("hard count underflow"),
                };
                // Extract the immutable reference count from the status field.
                let old_ref_count = (old_status & arc::REF_COUNT_MASK) >> arc::REF_COUNT_SHIFT;
                // Decrement the immutable reference count, checking for underflow.
                let new_ref_count = match old_ref_count.checked_sub(1) {
                    Some(shared_count) => shared_count,
                    None => panic!("ref count underflow"),
                };
                // Clear the hard and immutable reference count fields.
                let new_status = old_status & !(arc::HARD_COUNT_MASK | arc::REF_COUNT_MASK);
                // Splice the decremented hard and immutable reference counts into the status field.
                let new_status = new_status | new_hard_count | new_ref_count << arc::REF_COUNT_SHIFT;
                // Check if any hard references will remain.
                if new_hard_count != 0 {
                    // Atomically update the status field, synchronizing with reference acquires.
                    match (*header).status.compare_exchange_weak(old_status, new_status, Release, Relaxed) {
                        // CAS succeeded; hard and immutable references released.
                        Ok(_) => return,
                        // CAS failed; update the status field and try again.
                        Err(status) => old_status = status,
                    }
                } else {
                    // Extract the soft reference count from the status field.
                    let old_soft_count = (old_status & arc::SOFT_COUNT_MASK) >> arc::SOFT_COUNT_SHIFT;
                    // Check if all soft references have dropped.
                    if old_soft_count == 0 {
                        // Store the new status field; can't fail because we're the last reference of any kind.
                        (*header).status.store(new_status, Relaxed);
                        // Drop the shared resident.
                        R::resident_drop(data, &mut (*header).meta);
                        // Drop the arc header.
                        (*header).drop::<R>(data);
                        // Get the block of memory containing the arc structure.
                        let block = Block::from_raw_parts(header as *mut u8, size);
                        // Deallocate the block.
                        AllocTag::from_ptr(header as *mut u8).dealloc(block);
                        return;
                    } else {
                        // Convert our hard reference into a soft reference to avoid racing with other soft refs.
                        let new_soft_count = old_soft_count.wrapping_add(1);
                        // Check if the incremented soft reference count overflows its bit field.
                        if new_soft_count > arc::SOFT_COUNT_MAX {
                            panic!("soft count overflow");
                        }
                        // Clear the soft reference count bit field.
                        let new_status = new_status & !arc::SOFT_COUNT_MASK;
                        // Splice the incremented soft reference count into the status field.
                        let new_status = new_status | new_soft_count << arc::SOFT_COUNT_SHIFT;
                        // Atomically update the status field, synchronizing with reference acquires and releases.
                        match (*header).status.compare_exchange_weak(old_status, new_status, SeqCst, Relaxed) {
                            // CAS succeeded; the last hard reference has been released.
                            Ok(_) => {
                                // Drop the shared resident.
                                R::resident_drop(data, &mut (*header).meta);
                                // Update the status field.
                                old_status = new_status;
                                // Spin until the soft reference has been released.
                                loop {
                                    // Extract the soft reference count from the status field.
                                    let old_soft_count = (old_status & arc::SOFT_COUNT_MASK) >> arc::SOFT_COUNT_SHIFT;
                                    // Decrement the soft reference count, checking for underflow.
                                    let new_soft_count = match old_soft_count.checked_sub(1) {
                                        Some(soft_count) => soft_count,
                                        None => panic!("soft count underflow"),
                                    };
                                    // Clear the soft reference count bit field.
                                    let new_status = old_status & !arc::SOFT_COUNT_MASK;
                                    // Splice the decremented soft reference count into the status field.
                                    let new_status = new_status | new_soft_count << arc::SOFT_COUNT_SHIFT;
                                    // Atomically update the status field, synchronizing with reference acquires.
                                    match (*header).status.compare_exchange_weak(old_status, new_status, Release, Relaxed) {
                                        // CAS succeeded.
                                        Ok(_) => {
                                            // Check if all soft references have been released.
                                            if new_soft_count == 0 {
                                                // Shared resident has already been dropped; drop the arc header.
                                                (*header).drop::<R>(data);
                                                // Get the block of memory containing the arc structure.
                                                let block = Block::from_raw_parts(header as *mut u8, size);
                                                // Deallocate the block.
                                                AllocTag::from_ptr(header as *mut u8).dealloc(block);
                                            }
                                            return;
                                        },
                                        // CAS failed; update the status field and try again.
                                        Err(status) => old_status = status,
                                    }
                                }
                            },
                            // CAS failed; update the status field and try again.
                            Err(status) => old_status = status,
                        }
                    }
                }
            }
        }
    }
}
