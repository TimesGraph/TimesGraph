use core::cmp::Ordering;
use core::fmt::{self, Debug, Display, Pointer, Formatter};
use core::hash::{Hash, Hasher};
use core::marker::PhantomData;
use core::mem;
use core::ops::{Deref, DerefMut, Index, IndexMut, Add, AddAssign};
use core::ptr::NonNull;
use core::sync::atomic::Ordering::{Acquire, Relaxed, Release, SeqCst};
use crate::block::{Block, Layout};
use crate::alloc::{AllocTag, Hold, Holder, HoldError};
use crate::lease::{arc, ArcHeader, ArcError, Lease, DynamicLease, Ref, Hard, Soft};
use crate::resident::{Resident, ResidentFromValue, ResidentFromClone,
                      ResidentFromCloneUnchecked, ResidentFromCopy,
                      ResidentFromCopyUnchecked, ResidentFromEmpty,
                      ResidentWithCapacity, ResidentUnwrap, ResidentDeref,
                      ResidentDerefMut, ResidentAsRef, ResidentAsMut,
                      ResidentIndex, ResidentIndexMut, ResidentAdd,
                      ResidentAddAssign, ResidentIntoIterator,
                      ResidentIntoRefIterator, ResidentIntoMutIterator,
                      ResidentPartialEq, ResidentEq, ResidentPartialOrd,
                      ResidentOrd, ResidentHash, ResidentDisplay, ResidentDebug};

/// A thread-safe, atomically counted, mutably dereferenceable hard reference
/// to a `Resident` occupying a shared, `Hold`-allocated memory block.
pub struct Mut<'a, R: Resident> {
    /// Pointer to the resident memory block.
    data: NonNull<R::Data>,
    /// Variant over R::Data, with drop check.
    data_lifetime: PhantomData<R::Data>,
    /// Variant over ArcHeader<R::Meta>, with drop check.
    meta_lifetime: PhantomData<ArcHeader<R::Meta>>,
    /// Variant over 'a.
    hold_lifetime: PhantomData<&'a ()>,
}

unsafe impl<'a, R: Resident> Send for Mut<'a, R> where R::Data: Send, R::Meta: Send {
}

unsafe impl<'a, R: Resident> Sync for Mut<'a, R> where R::Data: Sync, R::Meta: Sync {
}

impl<'a, R: Resident> Mut<'a, R> {
    #[inline]
    pub fn try_hold_new_meta<T, M>(hold: &dyn Hold<'a>, data: T, meta: M)
        -> Result<Mut<'a, R>, HoldError>
        where R: ResidentFromValue<Mut<'a, R>, T, M>
    {
        unsafe {
            // Allocate a new arc structure.
            let resident = arc::alloc_new::<R, Mut<'a, R>, T, M>(hold, &data, &meta, arc::MUT_STATUS_INIT)?;
            // Construct a new Mut lease.
            let mut lease = Mut::from_raw(resident);
            // Initialize the new resident.
            R::new_resident(&mut lease, data, meta);
            // Return the new lease.
            Ok(lease)
        }
    }

    #[inline]
    pub fn try_hold_clone_meta<T: ?Sized, M>(hold: &dyn Hold<'a>, data: &T, meta: M)
        -> Result<Mut<'a, R>, HoldError>
        where R: ResidentFromClone<Mut<'a, R>, T, M>
    {
        unsafe {
            // Allocate a new arc structure.
            let resident = arc::alloc_clone::<R, Mut<'a, R>, T, M>(hold, &data, &meta, arc::MUT_STATUS_INIT)?;
            // Construct a new Mut lease.
            let mut lease = Mut::from_raw(resident);
            // Initialize the new resident.
            R::new_resident(&mut lease, data, meta);
            // Return the new lease.
            Ok(lease)
        }
    }

    #[inline]
    pub unsafe fn try_hold_clone_unchecked_meta<T: ?Sized, M>(hold: &dyn Hold<'a>, data: &T, meta: M)
        -> Result<Mut<'a, R>, HoldError>
        where R: ResidentFromCloneUnchecked<Mut<'a, R>, T, M>
    {
        // Allocate a new arc structure.
        let resident = arc::alloc_clone_unchecked::<R, Mut<'a, R>, T, M>(hold, &data, &meta, arc::MUT_STATUS_INIT)?;
        // Construct a new Mut lease.
        let mut lease = Mut::from_raw(resident);
        // Initialize the new resident.
        R::new_resident(&mut lease, data, meta);
        // Return the new lease.
        Ok(lease)
    }

    #[inline]
    pub fn try_hold_copy_meta<T: ?Sized, M>(hold: &dyn Hold<'a>, data: &T, meta: M)
        -> Result<Mut<'a, R>, HoldError>
        where R: ResidentFromCopy<Mut<'a, R>, T, M>
    {
        unsafe {
            // Allocate a new arc structure.
            let resident = arc::alloc_copy::<R, Mut<'a, R>, T, M>(hold, &data, &meta, arc::MUT_STATUS_INIT)?;
            // Construct a new Mut lease.
            let mut lease = Mut::from_raw(resident);
            // Initialize the new resident.
            R::new_resident(&mut lease, data, meta);
            // Return the new lease.
            Ok(lease)
        }
    }

    #[inline]
    pub unsafe fn try_hold_copy_unchecked_meta<T: ?Sized, M>(hold: &dyn Hold<'a>, data: &T, meta: M)
        -> Result<Mut<'a, R>, HoldError>
        where R: ResidentFromCopyUnchecked<Mut<'a, R>, T, M>
    {
        // Allocate a new arc structure.
        let resident = arc::alloc_copy_unchecked::<R, Mut<'a, R>, T, M>(hold, &data, &meta, arc::MUT_STATUS_INIT)?;
        // Construct a new Mut lease.
        let mut lease = Mut::from_raw(resident);
        // Initialize the new resident.
        R::new_resident(&mut lease, data, meta);
        // Return the new lease.
        Ok(lease)
    }

    #[inline]
    pub fn try_hold_empty_meta<M>(hold: &dyn Hold<'a>, meta: M)
        -> Result<Mut<'a, R>, HoldError>
        where R: ResidentFromEmpty<Mut<'a, R>, M>
    {
        unsafe {
            // Allocate a new arc structure.
            let resident = arc::alloc_empty::<R, Mut<'a, R>, M>(hold, &meta, arc::MUT_STATUS_INIT)?;
            // Construct a new Mut lease.
            let mut lease = Mut::from_raw(resident);
            // Initialize the new resident.
            R::new_resident(&mut lease, meta);
            // Return the new lease.
            Ok(lease)
        }
    }

    #[inline]
    pub fn try_hold_cap_meta<M>(hold: &dyn Hold<'a>, cap: usize, meta: M)
        -> Result<Mut<'a, R>, HoldError>
        where R: ResidentWithCapacity<Mut<'a, R>, M>
    {
        unsafe {
            // Allocate a new arc structure.
            let resident = arc::alloc_cap::<R, Mut<'a, R>, M>(hold, cap, &meta, arc::MUT_STATUS_INIT)?;
            // Construct a new Mut lease.
            let mut lease = Mut::from_raw(resident);
            // Initialize the new resident.
            R::new_resident(&mut lease, cap, meta);
            // Return the new lease.
            Ok(lease)
        }
    }

    #[inline]
    pub fn try_hold_new<T>(hold: &dyn Hold<'a>, data: T) -> Result<Mut<'a, R>, HoldError>
        where R: ResidentFromValue<Mut<'a, R>, T>
    {
        Mut::try_hold_new_meta(hold, data, ())
    }

    #[inline]
    pub fn try_hold_clone<T: ?Sized>(hold: &dyn Hold<'a>, data: &T) -> Result<Mut<'a, R>, HoldError>
        where R: ResidentFromClone<Mut<'a, R>, T>
    {
        Mut::try_hold_clone_meta(hold, data, ())
    }

    #[inline]
    pub unsafe fn try_hold_clone_unchecked<T: ?Sized>(hold: &dyn Hold<'a>, data: &T) -> Result<Mut<'a, R>, HoldError>
        where R: ResidentFromCloneUnchecked<Mut<'a, R>, T>
    {
        Mut::try_hold_clone_unchecked_meta(hold, data, ())
    }

    #[inline]
    pub fn try_hold_copy<T: ?Sized>(hold: &dyn Hold<'a>, data: &T) -> Result<Mut<'a, R>, HoldError>
        where R: ResidentFromCopy<Mut<'a, R>, T>
    {
        Mut::try_hold_copy_meta(hold, data, ())
    }

    #[inline]
    pub unsafe fn try_hold_copy_unchecked<T: ?Sized>(hold: &dyn Hold<'a>, data: &T) -> Result<Mut<'a, R>, HoldError>
        where R: ResidentFromCopyUnchecked<Mut<'a, R>, T>
    {
        Mut::try_hold_copy_unchecked_meta(hold, data, ())
    }

    #[inline]
    pub fn try_hold_empty(hold: &dyn Hold<'a>) -> Result<Mut<'a, R>, HoldError>
        where R: ResidentFromEmpty<Mut<'a, R>>
    {
        Mut::try_hold_empty_meta(hold, ())
    }

    #[inline]
    pub fn try_hold_cap(hold: &dyn Hold<'a>, cap: usize) -> Result<Mut<'a, R>, HoldError>
        where R: ResidentWithCapacity<Mut<'a, R>>
    {
        Mut::try_hold_cap_meta(hold, cap, ())
    }

    #[inline]
    pub fn hold_new<T>(hold: &dyn Hold<'a>, data: T) -> Mut<'a, R>
        where R: ResidentFromValue<Mut<'a, R>, T>
    {
        Mut::try_hold_new(hold, data).unwrap()
    }

    #[inline]
    pub fn hold_clone<T: ?Sized>(hold: &Hold<'a>, data: &T) -> Mut<'a, R>
        where R: ResidentFromClone<Mut<'a, R>, T>
    {
        Mut::try_hold_clone(hold, data).unwrap()
    }

    #[inline]
    pub unsafe fn hold_clone_unchecked<T: ?Sized>(hold: &dyn Hold<'a>, data: &T) -> Mut<'a, R>
        where R: ResidentFromCloneUnchecked<Mut<'a, R>, T>
    {
        Mut::try_hold_clone_unchecked(hold, data).unwrap()
    }

    #[inline]
    pub fn hold_copy<T: ?Sized>(hold: &Hold<'a>, data: &T) -> Mut<'a, R>
        where R: ResidentFromCopy<Mut<'a, R>, T>
    {
        Mut::try_hold_copy(hold, data).unwrap()
    }

    #[inline]
    pub unsafe fn hold_copy_unchecked<T: ?Sized>(hold: &dyn Hold<'a>, data: &T) -> Mut<'a, R>
        where R: ResidentFromCopyUnchecked<Mut<'a, R>, T>
    {
        Mut::try_hold_copy_unchecked(hold, data).unwrap()
    }

    #[inline]
    pub fn hold_empty(hold: &Hold<'a>) -> Mut<'a, R>
        where R: ResidentFromEmpty<Mut<'a, R>>
    {
        Mut::try_hold_empty(hold).unwrap()
    }

    #[inline]
    pub fn hold_cap(hold: &Hold<'a>, cap: usize) -> Mut<'a, R>
        where R: ResidentWithCapacity<Mut<'a, R>>
    {
        Mut::try_hold_cap(hold, cap).unwrap()
    }

    #[inline]
    pub fn new<T>(data: T) -> Mut<'a, R>
        where R: ResidentFromValue<Mut<'a, R>, T>
    {
        Mut::hold_new(Hold::global(), data)
    }

    #[inline]
    pub fn from_clone<T: ?Sized>(data: &T) -> Mut<'a, R>
        where R: ResidentFromClone<Mut<'a, R>, T>
    {
        Mut::hold_clone(Hold::global(), data)
    }

    #[inline]
    pub unsafe fn from_clone_unchecked<T: ?Sized>(data: &T) -> Mut<'a, R>
        where R: ResidentFromCloneUnchecked<Mut<'a, R>, T>
    {
        Mut::hold_clone_unchecked(Hold::global(), data)
    }

    #[inline]
    pub fn from_copy<T: ?Sized>(data: &T) -> Mut<'a, R>
        where R: ResidentFromCopy<Mut<'a, R>, T>
    {
        Mut::hold_copy(Hold::global(), data)
    }

    #[inline]
    pub unsafe fn from_copy_unchecked<T: ?Sized>(data: &T) -> Mut<'a, R>
        where R: ResidentFromCopyUnchecked<Mut<'a, R>, T>
    {
        Mut::hold_copy_unchecked(Hold::global(), data)
    }

    #[inline]
    pub fn empty() -> Mut<'a, R>
        where R: ResidentFromEmpty<Mut<'a, R>>
    {
        Mut::hold_empty(Hold::global())
    }

    #[inline]
    pub fn with_cap(cap: usize) -> Mut<'a, R>
        where R: ResidentWithCapacity<Mut<'a, R>>
    {
        Mut::hold_cap(Hold::global(), cap)
    }

    /// Constructs a `Mut` lease from a raw pointer returned by `Mut::into_raw`.
    #[inline]
    pub unsafe fn from_raw(data: *mut R::Data) -> Mut<'a, R> {
        Mut {
            data: NonNull::new_unchecked(data),
            data_lifetime: PhantomData,
            meta_lifetime: PhantomData,
            hold_lifetime: PhantomData,
        }
    }

    /// Returns a pointer to the `ArcHeader` preceding the shared resident.
    #[inline]
    fn header(this: &Mut<'a, R>) -> *mut ArcHeader<R::Meta> {
        arc::header::<R>(this.data.as_ptr())
    }

    /// Returns the number of hard references to the shared resident.
    #[inline]
    pub fn hard_count(this: &Mut<'a, R>) -> usize {
        unsafe { (*Mut::header(this)).hard_count() }
    }

    /// Returns the number of soft references to the shared resident.
    #[inline]
    pub fn soft_count(this: &Mut<'a, R>) -> usize {
        unsafe { (*Mut::header(this)).soft_count() }
    }

    /// Returns the number of immutable references to the shared resident;
    /// always returns zero.
    #[inline]
    pub fn ref_count(this: &Mut<'a, R>) -> usize {
        unsafe { (*Mut::header(this)).ref_count() }
    }

    /// Returns a reference to the user-provided metadata associated with the
    /// shared resident.
    #[inline]
    pub fn metadata<'b>(this: &'b Mut<'a, R>) -> &'b R::Meta {
        unsafe { &(*Mut::header(this)).meta }
    }

    /// Returns a mutable reference to the user-provided metadata associated
    /// with the shared resident.
    #[inline]
    pub fn metadata_mut<'b>(this: &'b mut Mut<'a, R>) -> &'b mut R::Meta {
        unsafe { &mut (*Mut::header(this)).meta }
    }

    /// Converts this mutable lease into an immutable lease to the shared resident,
    /// returning an error if the incremented reference count overflows `REF_COUNT_MAX`.
    pub fn try_into_ref(this: Mut<'a, R>) -> Result<Ref<'a, R>, ArcError> {
        unsafe {
            // Get a pointer to the shared resident.
            let data = this.data.as_ptr();
            // Get a pointer to the arc header preceding the resident.
            let header = arc::header::<R>(data);
            // Load the status field; synchronized by subsequent CAS.
            let mut old_status = (*header).status.load(Relaxed);
            // Spin until the mutable reference is released, and an immutable reference is acquired.
            loop {
                // Extract the immutable reference count from the status field.
                let old_ref_count = (old_status & arc::REF_COUNT_MASK) >> arc::REF_COUNT_SHIFT;
                // Increment the immutable reference count.
                let new_ref_count = old_ref_count.wrapping_add(1);
                // Check if the incremented immutable reference count overflows its bit field.
                if new_ref_count > arc::REF_COUNT_MAX {
                    return Err(ArcError::RefCountOverflow);
                }
                // Clear the immutable reference count bit field, and unset the mut flag.
                let new_status = old_status & !(arc::REF_COUNT_MASK | arc::MUT_FLAG);
                // Splice the incremented immutable reference count into the status field.
                let new_status = new_status | new_ref_count << arc::REF_COUNT_SHIFT;
                // Atomically update the status field, synchronizing with reference acquires and releases.
                match (*header).status.compare_exchange_weak(old_status, new_status, SeqCst, Relaxed) {
                    // CAS succeeded.
                    Ok(_) => {
                        // Discard the original lease, whose hard reference we took,
                        // and whose mutable reference we released.
                        mem::forget(this);
                        // Return a new Ref lease.
                        return Ok(Ref::from_raw(data));
                    },
                    // CAS failed; update the status field and try again.
                    Err(status) => old_status = status,
                }
            }
        }
    }

    /// Converts this mutable lease into an immutable lease to the shared resident.
    ///
    /// # Panics
    ///
    /// Panics if the incremented reference count overflows `REF_COUNT_MAX`.
    pub fn into_ref(this: Mut<'a, R>) -> Ref<'a, R> {
        Mut::try_into_ref(this).unwrap()
    }

    /// Returns a new hard reference to the shared resident, returning an error
    /// if the incremented hard reference count overflows `HARD_COUNT_MAX`.
    ///
    /// # Safety
    ///
    /// Mutable leases can coexist with hard and soft leases to the same
    /// resident. This can cause a future deadlock if a thread holding a
    /// mutable lease to a resident attempts to convert another hard or soft
    /// lease to the same resident into a mutable or immutable lease.
    pub unsafe fn try_to_hard(this: &Mut<'a, R>) -> Result<Hard<'a, R>, ArcError> {
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

    /// Returns a new hard reference to the shared resident.
    ///
    /// # Safety
    ///
    /// Mutable leases can coexist with hard and soft leases to the same
    /// resident. This can cause a future deadlock if a thread holding a
    /// mutable lease to a resident attempts to convert another hard or soft
    /// lease to the same resident into a mutable or immutable lease.
    ///
    /// # Panics
    ///
    /// Panics if the incremented hard reference count overflows `HARD_COUNT_MAX`.
    pub unsafe fn to_hard(this: &Mut<'a, R>) -> Hard<'a, R> {
        Mut::try_to_hard(this).unwrap()
    }

    /// Converts this mutable lease into a hard lease.
    pub fn into_hard(this: Mut<'a, R>) -> Hard<'a, R> {
        unsafe {
            // Get a pointer to the shared resident.
            let data = this.data.as_ptr();
            // Get a pointer to the arc header preceding the resident.
            let header = arc::header::<R>(data);
            // Load the status field; synchronized by subsequent CAS.
            let mut old_status = (*header).status.load(Relaxed);
            // Spin until the mutable reference is released.
            loop {
                // Clear the mut flag in the status field.
                let new_status = old_status & !arc::MUT_FLAG;
                // Atomically update the status field, synchronizing with reference acquires.
                match (*header).status.compare_exchange_weak(old_status, new_status, Release, Relaxed) {
                    // CAS succeeded.
                    Ok(_) => {
                        // Discard the original lease, whose hard reference we took,
                        // and whose mutable reference we released.
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
    ///
    /// # Safety
    ///
    /// Mutable leases can coexist with hard and soft leases to the same
    /// resident. This can cause a future deadlock if a thread holding a
    /// mutable lease to a resident attempts to convert another hard or soft
    /// lease to the same resident into a mutable or immutable lease.
    pub unsafe fn try_to_soft(this: &Mut<'a, R>) -> Result<Soft<'a, R>, ArcError> {
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
            // Clear the soft reference count bit field.
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

    /// Returns a new soft reference to the shared resident.
    ///
    /// # Safety
    ///
    /// Mutable leases can coexist with hard and soft leases to the same
    /// resident. This can cause a future deadlock if a thread holding a
    /// mutable lease to a resident attempts to convert another hard or soft
    /// lease to the same resident into a mutable or immutable lease.
    ///
    /// # Panics
    ///
    /// Panics if the incremented soft reference count overflows `SOFT_COUNT_MAX`.
    pub unsafe fn to_soft(this: &Mut<'a, R>) -> Soft<'a, R> {
        Mut::try_to_soft(this).unwrap()
    }

    /// Converts this mutable lease into a soft lease to the shared resident,
    /// returning an error if the incremented soft reference count overflows
    /// `SOFT_COUNT_MAX`.
    pub fn try_into_soft(this: Mut<'a, R>) -> Result<Soft<'a, R>, ArcError> {
        unsafe {
            // Get a pointer to the shared resident.
            let data = this.data.as_ptr();
            // Get a pointer to the arc header preceding the resident.
            let header = arc::header::<R>(data);
            // Load the status field; synchronized by subsequent CAS.
            let mut old_status = (*header).status.load(Relaxed);
            // Spin until a soft reference is acquired, and the hard and mutable references are released.
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
                // Clear the hard and soft reference count bit fields, and unset the mut flag.
                let new_status = old_status & !(arc::HARD_COUNT_MASK | arc::SOFT_COUNT_MASK | arc::MUT_FLAG);
                // Splice the decremented hard, and incremented soft reference counts into the status field.
                let new_status = new_status | new_hard_count | new_soft_count << arc::SOFT_COUNT_SHIFT;
                // Atomically update the status field, synchronizing with reference acquires and releases.
                match (*header).status.compare_exchange_weak(old_status, new_status, SeqCst, Relaxed) {
                    // CAS succeeded.
                    Ok(_) => {
                        // Check if the hard count dropped to zero.
                        if new_hard_count == 0 {
                            // Drop the shared resident.
                            R::resident_drop(data, &mut (*header).meta);
                        }
                        // Discard the original lease, whose hard and mutable references we released.
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

    /// Converts this mutable lease into a soft lease to the shared resident.
    ///
    /// # Panics
    ///
    /// Panics if the incremented soft reference count overflows `SOFT_COUNT_MAX`.
    pub fn into_soft(this: Mut<'a, R>) -> Soft<'a, R> {
        Mut::try_into_soft(this).unwrap()
    }

    /// Converts this mutable lease into a raw pointer to the shared resident.
    /// Use `Mut::from_raw` to reconstitute the returned pointer back into
    /// a mutable lease.
    ///
    /// # Safety
    ///
    /// A memory leak will occur unless the returned pointer is eventually
    /// converted back into a mutable lease and dropped.
    #[inline]
    pub unsafe fn into_raw(this: Mut<'a, R>) -> *mut R::Data {
        let data = this.data.as_ptr();
        mem::forget(this);
        data
    }

    /// Returns a raw pointer to the shared resident.
    ///
    /// # Safety
    ///
    /// The shared resident may be uninitialized.
    #[inline]
    pub unsafe fn as_ptr_unchecked(this: &Mut<'a, R>) -> *mut R::Data {
        this.data.as_ptr()
    }

    /// Consumes this mutable lease, and returns the shared resident;
    /// returns an `Err` containing the original lease if any outstanding hard
    /// leases prevent the resident from being moved.
    pub fn try_unwrap(this: Mut<'a, R>) -> Result<R::Target, Mut<'a, R>> where R: ResidentUnwrap<Mut<'a, R>> {
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
            // Spin until the hard and mutable references are released.
            loop {
                // Extract the hard reference count from the status field.
                let old_hard_count = old_status & arc::HARD_COUNT_MASK;
                // Check if the shared resident has multiple hard references.
                if old_hard_count != 1 {
                    // Can't unwrap an aliased resident.
                    return Err(this);
                }
                // Clear the hard reference count bit field, and unset the mut flag.
                let new_status = old_status & !(arc::HARD_COUNT_MASK | arc::MUT_FLAG);
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
                    // Discard the original lease, whose hard and mutable reference we released.
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

    /// Consumes this mutable lease, and returns the shared resident.
    ///
    /// # Panics
    ///
    /// Panics if any outstanding hard leases prevent the resident from being moved.
    pub fn unwrap(this: Mut<'a, R>) -> R::Target where R: ResidentUnwrap<Mut<'a, R>> {
        match Mut::try_unwrap(this) {
            Ok(resident) => resident,
            Err(_) => panic!("aliased resident"),
        }
    }
}

impl<'a, R: Resident> Holder<'a> for Mut<'a, R> {
    #[inline]
    fn holder(&self) -> &'a dyn Hold<'a> {
        AllocTag::from_ptr(Mut::header(self) as *mut u8).holder()
    }
}

impl<'a, R: Resident> Lease for Mut<'a, R> {
    type Data = R::Data;

    type Meta = R::Meta;

    #[inline]
    fn data(&self) -> *mut R::Data {
        self.data.as_ptr()
    }

    #[inline]
    fn meta(&self) -> *mut R::Meta {
        unsafe { &mut (*Mut::header(self)).meta }
    }
}

impl<'a, R: Resident> DynamicLease<'a> for Mut<'a, R> {
    unsafe fn realloc(&mut self, new_layout: Layout) -> Result<(), HoldError> {
        // Reallocate the leased memory block to fit the new layout.
        let new_data = arc::realloc::<R>(self.data.as_ptr(), new_layout)?;
        // Update the lease to point to the reallocated resident.
        self.data = NonNull::new_unchecked(new_data);
        // Return successfully.
        Ok(())
    }

    unsafe fn resize(&mut self, new_layout: Layout) -> Result<(), HoldError> {
        // Resize the leased memory block to fit the new layout.
        let new_data = arc::resize::<R>(self.data.as_ptr(), new_layout)?;
        // Update the lease to point to the resized resident.
        self.data = NonNull::new_unchecked(new_data);
        // Return successfully.
        Ok(())
    }
}

impl<'a, R: ResidentDeref<Mut<'a, R>>> Deref for Mut<'a, R> {
    type Target = R::Target;

    #[inline]
    fn deref(&self) -> &R::Target {
        R::resident_deref(self)
    }
}

impl<'a, R: ResidentDerefMut<Mut<'a, R>>> DerefMut for Mut<'a, R> {
    #[inline]
    fn deref_mut(&mut self) -> &mut R::Target {
        R::resident_deref_mut(self)
    }
}

impl<'a, R: ResidentAsRef<Mut<'a, R>, T>, T: ?Sized> AsRef<T> for Mut<'a, R> {
    #[inline]
    fn as_ref(&self) -> &T {
        R::resident_as_ref(self)
    }
}

impl<'a, R: ResidentAsMut<Mut<'a, R>, T>, T: ?Sized> AsMut<T> for Mut<'a, R> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        R::resident_as_mut(self)
    }
}

impl<'a, R: ResidentIndex<Mut<'a, R>, Idx>, Idx> Index<Idx> for Mut<'a, R> {
    type Output = R::Output;

    #[inline]
    fn index(&self, index: Idx) -> &R::Output {
        R::resident_index(self, index)
    }
}

impl<'a, R: ResidentIndexMut<Mut<'a, R>, Idx>, Idx> IndexMut<Idx> for Mut<'a, R> {
    #[inline]
    fn index_mut(&mut self, index: Idx) -> &mut R::Output {
        R::resident_index_mut(self, index)
    }
}

impl<'a, R: ResidentAdd<Mut<'a, R>, Rhs>, Rhs> Add<Rhs> for Mut<'a, R> {
    type Output = R::Output;

    #[inline]
    fn add(self, rhs: Rhs) -> R::Output {
        R::resident_add(self, rhs)
    }
}

impl<'a, R: ResidentAddAssign<Mut<'a, R>, Rhs>, Rhs> AddAssign<Rhs> for Mut<'a, R> {
    #[inline]
    fn add_assign(&mut self, rhs: Rhs) {
        R::resident_add_assign(self, rhs);
    }
}

impl<'a, R: ResidentIntoIterator<Mut<'a, R>>> IntoIterator for Mut<'a, R> {
    type Item = R::Item;

    type IntoIter = R::IntoIter;

    #[inline]
    fn into_iter(self) -> R::IntoIter {
        R::resident_into_iter(self)
    }
}

impl<'a, R: ResidentIntoRefIterator<'a, Mut<'a, R>>> IntoIterator for &'a Mut<'a, R> {
    type Item = R::Item;

    type IntoIter = R::IntoIter;

    #[inline]
    fn into_iter(self) -> R::IntoIter {
        R::resident_into_iter(self)
    }
}

impl<'a, R: ResidentIntoMutIterator<'a, Mut<'a, R>>> IntoIterator for &'a mut Mut<'a, R> {
    type Item = R::Item;

    type IntoIter = R::IntoIter;

    #[inline]
    fn into_iter(self) -> R::IntoIter {
        R::resident_into_iter(self)
    }
}

impl<'a, R: ResidentPartialEq<Mut<'a, R>, T>, T: ?Sized> PartialEq<T> for Mut<'a, R> {
    #[inline]
    fn eq(&self, other: &T) -> bool {
        R::resident_eq(self, other)
    }

    #[inline]
    fn ne(&self, other: &T) -> bool {
        R::resident_ne(self, other)
    }
}

impl<'a, R: ResidentEq<Mut<'a, R>>> Eq for Mut<'a, R> {
}

impl<'a, R: ResidentPartialOrd<Mut<'a, R>, T>, T: ?Sized> PartialOrd<T> for Mut<'a, R> {
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

impl<'a, R: ResidentOrd<Mut<'a, R>>> Ord for Mut<'a, R> {
    #[inline]
    fn cmp(&self, other: &Mut<'a, R>) -> Ordering {
        R::resident_cmp(self, other)
    }
}

impl<'a, R: ResidentHash<Mut<'a, R>>> Hash for Mut<'a, R> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        R::resident_hash(self, state);
    }
}

impl<'a, R: ResidentDisplay<Mut<'a, R>>> Display for Mut<'a, R> {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        R::resident_fmt(self, f)
    }
}

impl<'a, R: ResidentDebug<Mut<'a, R>>> Debug for Mut<'a, R> {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        R::resident_fmt(self, f)
    }
}

impl<'a, R: Resident> Pointer for Mut<'a, R> {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Pointer::fmt(&self.data.as_ptr(), f)
    }
}

unsafe impl<'a, #[may_dangle] R: Resident> Drop for Mut<'a, R> {
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
            // Spin until the hard and mutable references have been released.
            loop {
                // Extract the hard reference count from the status field.
                let old_hard_count = old_status & arc::HARD_COUNT_MASK;
                // Decrement the hard reference count, checking for underflow.
                let new_hard_count = match old_hard_count.checked_sub(1) {
                    Some(hard_count) => hard_count,
                    None => panic!("hard count underflow"),
                };
                // Clear the hard reference count bit field, and unset the mut flag.
                let new_status = old_status & !(arc::HARD_COUNT_MASK | arc::MUT_FLAG);
                // Splice the decremented hard reference count into the status field.
                let new_status = new_status | new_hard_count;
                // Check if any hard references will remain.
                if new_hard_count != 0 {
                    // Atomically update the status field, synchronizing with reference acquires.
                    match (*header).status.compare_exchange_weak(old_status, new_status, Release, Relaxed) {
                        // CAS succeeded; hard reference released.
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
