use core::fmt::{self, Display, Debug, Pointer, Formatter};
use core::hash::{Hash, Hasher};
use core::marker::PhantomData;
use core::mem;
use core::ptr::{self, NonNull};
use core::sync::atomic::{self, AtomicUsize};
use core::sync::atomic::Ordering::{Acquire, Relaxed, Release, SeqCst};
use crate::block::{self, Block, Layout};
use crate::alloc::{AllocTag, Hold, Holder, HoldError, Stow, TryClone};
use crate::lease::{arc, ArcHeader, ArcError, Lease, Mut, Ref, Soft};
use crate::resident::{Resident, ResidentFromValue, ResidentFromClone,
                      ResidentFromCloneUnchecked, ResidentFromCopy,
                      ResidentFromCopyUnchecked, ResidentFromEmpty,
                      ResidentWithCapacity, ResidentUnwrap, ResidentHash,
                      ResidentDisplay, ResidentDebug, ResidentStow};

/// A thread-safe, atomically counted, undereferenceable hard reference to a
/// `Resident` occuping a shared, `Hold`-allocated memory block.
pub struct Hard<'a, R: Resident> {
    /// Pointer to the resident memory block.
    data: NonNull<R::Data>,
    /// Variant over R::Data, with drop check.
    data_lifetime: PhantomData<R::Data>,
    /// Variant over ArcHeader<R::Meta>, with drop check.
    meta_lifetime: PhantomData<ArcHeader<R::Meta>>,
    /// Variant over 'a.
    hold_lifetime: PhantomData<&'a ()>,
}

unsafe impl<'a, R: Resident> Send for Hard<'a, R> where R::Data: Send, R::Meta: Send {
}

unsafe impl<'a, R: Resident> Sync for Hard<'a, R> where R::Data: Sync, R::Meta: Sync {
}

impl<'a, R: Resident> Hard<'a, R> {
    #[inline]
    pub fn try_hold_new_meta<T, M>(hold: &dyn Hold<'a>, data: T, meta: M)
        -> Result<Hard<'a, R>, HoldError>
        where R: ResidentFromValue<Hard<'a, R>, T, M>
    {
        unsafe {
            // Allocate a new arc structure.
            let resident = arc::alloc_new::<R, Hard<'a, R>, T, M>(hold, &data, &meta, arc::HARD_STATUS_INIT)?;
            // Construct a new Hard lease.
            let mut lease = Hard::from_raw(resident);
            // Initialize the new resident.
            R::new_resident(&mut lease, data, meta);
            // Return the new lease.
            Ok(lease)
        }
    }

    #[inline]
    pub fn try_hold_clone_meta<T: ?Sized, M>(hold: &dyn Hold<'a>, data: &T, meta: M)
        -> Result<Hard<'a, R>, HoldError>
        where R: ResidentFromClone<Hard<'a, R>, T, M>
    {
        unsafe {
            // Allocate a new arc structure.
            let resident = arc::alloc_clone::<R, Hard<'a, R>, T, M>(hold, &data, &meta, arc::HARD_STATUS_INIT)?;
            // Construct a new Hard lease.
            let mut lease = Hard::from_raw(resident);
            // Initialize the new resident.
            R::new_resident(&mut lease, data, meta);
            // Return the new lease.
            Ok(lease)
        }
    }

    #[inline]
    pub unsafe fn try_hold_clone_unchecked_meta<T: ?Sized, M>(hold: &dyn Hold<'a>, data: &T, meta: M)
        -> Result<Hard<'a, R>, HoldError>
        where R: ResidentFromCloneUnchecked<Hard<'a, R>, T, M>
    {
        // Allocate a new arc structure.
        let resident = arc::alloc_clone_unchecked::<R, Hard<'a, R>, T, M>(hold, &data, &meta, arc::HARD_STATUS_INIT)?;
        // Construct a new Hard lease.
        let mut lease = Hard::from_raw(resident);
        // Initialize the new resident.
        R::new_resident(&mut lease, data, meta);
        // Return the new lease.
        Ok(lease)
    }

    #[inline]
    pub fn try_hold_copy_meta<T: ?Sized, M>(hold: &dyn Hold<'a>, data: &T, meta: M)
        -> Result<Hard<'a, R>, HoldError>
        where R: ResidentFromCopy<Hard<'a, R>, T, M>
    {
        unsafe {
            // Allocate a new arc structure.
            let resident = arc::alloc_copy::<R, Hard<'a, R>, T, M>(hold, &data, &meta, arc::HARD_STATUS_INIT)?;
            // Construct a new Hard lease.
            let mut lease = Hard::from_raw(resident);
            // Initialize the new resident.
            R::new_resident(&mut lease, data, meta);
            // Return the new lease.
            Ok(lease)
        }
    }

    #[inline]
    pub unsafe fn try_hold_copy_unchecked_meta<T: ?Sized, M>(hold: &dyn Hold<'a>, data: &T, meta: M)
        -> Result<Hard<'a, R>, HoldError>
        where R: ResidentFromCopyUnchecked<Hard<'a, R>, T, M>
    {
        // Allocate a new arc structure.
        let resident = arc::alloc_copy_unchecked::<R, Hard<'a, R>, T, M>(hold, &data, &meta, arc::HARD_STATUS_INIT)?;
        // Construct a new Hard lease.
        let mut lease = Hard::from_raw(resident);
        // Initialize the new resident.
        R::new_resident(&mut lease, data, meta);
        // Return the new lease.
        Ok(lease)
    }

    #[inline]
    pub fn try_hold_empty_meta<M>(hold: &dyn Hold<'a>, meta: M)
        -> Result<Hard<'a, R>, HoldError>
        where R: ResidentFromEmpty<Hard<'a, R>, M>
    {
        unsafe {
            // Allocate a new arc structure.
            let resident = arc::alloc_empty::<R, Hard<'a, R>, M>(hold, &meta, arc::HARD_STATUS_INIT)?;
            // Construct a new Hard lease.
            let mut lease = Hard::from_raw(resident);
            // Initialize the new resident.
            R::new_resident(&mut lease, meta);
            // Return the new lease.
            Ok(lease)
        }
    }

    #[inline]
    pub fn try_hold_cap_meta<M>(hold: &dyn Hold<'a>, cap: usize, meta: M)
        -> Result<Hard<'a, R>, HoldError>
        where R: ResidentWithCapacity<Hard<'a, R>, M>
    {
        unsafe {
            // Allocate a new arc structure.
            let resident = arc::alloc_cap::<R, Hard<'a, R>, M>(hold, cap, &meta, arc::HARD_STATUS_INIT)?;
            // Construct a new Hard lease.
            let mut lease = Hard::from_raw(resident);
            // Initialize the new resident.
            R::new_resident(&mut lease, cap, meta);
            // Return the new lease.
            Ok(lease)
        }
    }

    #[inline]
    pub fn try_hold_new<T>(hold: &dyn Hold<'a>, data: T) -> Result<Hard<'a, R>, HoldError>
        where R: ResidentFromValue<Hard<'a, R>, T>
    {
        Hard::try_hold_new_meta(hold, data, ())
    }

    #[inline]
    pub fn try_hold_clone<T: ?Sized>(hold: &dyn Hold<'a>, data: &T) -> Result<Hard<'a, R>, HoldError>
        where R: ResidentFromClone<Hard<'a, R>, T>
    {
        Hard::try_hold_clone_meta(hold, data, ())
    }

    #[inline]
    pub unsafe fn try_hold_clone_unchecked<T: ?Sized>(hold: &dyn Hold<'a>, data: &T) -> Result<Hard<'a, R>, HoldError>
        where R: ResidentFromCloneUnchecked<Hard<'a, R>, T>
    {
        Hard::try_hold_clone_unchecked_meta(hold, data, ())
    }

    #[inline]
    pub fn try_hold_copy<T: ?Sized>(hold: &dyn Hold<'a>, data: &T) -> Result<Hard<'a, R>, HoldError>
        where R: ResidentFromCopy<Hard<'a, R>, T>
    {
        Hard::try_hold_copy_meta(hold, data, ())
    }

    #[inline]
    pub unsafe fn try_hold_copy_unchecked<T: ?Sized>(hold: &dyn Hold<'a>, data: &T) -> Result<Hard<'a, R>, HoldError>
        where R: ResidentFromCopyUnchecked<Hard<'a, R>, T>
    {
        Hard::try_hold_copy_unchecked_meta(hold, data, ())
    }

    #[inline]
    pub fn try_hold_empty(hold: &dyn Hold<'a>) -> Result<Hard<'a, R>, HoldError>
        where R: ResidentFromEmpty<Hard<'a, R>>
    {
        Hard::try_hold_empty_meta(hold, ())
    }

    #[inline]
    pub fn try_hold_cap(hold: &dyn Hold<'a>, cap: usize) -> Result<Hard<'a, R>, HoldError>
        where R: ResidentWithCapacity<Hard<'a, R>>
    {
        Hard::try_hold_cap_meta(hold, cap, ())
    }

    #[inline]
    pub fn hold_new<T>(hold: &dyn Hold<'a>, data: T) -> Hard<'a, R>
        where R: ResidentFromValue<Hard<'a, R>, T>
    {
        Hard::try_hold_new(hold, data).unwrap()
    }

    #[inline]
    pub fn hold_clone<T: ?Sized>(hold: &dyn Hold<'a>, data: &T) -> Hard<'a, R>
        where R: ResidentFromClone<Hard<'a, R>, T>
    {
        Hard::try_hold_clone(hold, data).unwrap()
    }

    #[inline]
    pub unsafe fn hold_clone_unchecked<T: ?Sized>(hold: &dyn Hold<'a>, data: &T) -> Hard<'a, R>
        where R: ResidentFromCloneUnchecked<Hard<'a, R>, T>
    {
        Hard::try_hold_clone_unchecked(hold, data).unwrap()
    }

    #[inline]
    pub fn hold_copy<T: ?Sized>(hold: &dyn Hold<'a>, data: &T) -> Hard<'a, R>
        where R: ResidentFromCopy<Hard<'a, R>, T>
    {
        Hard::try_hold_copy(hold, data).unwrap()
    }

    #[inline]
    pub unsafe fn hold_copy_unchecked<T: ?Sized>(hold: &dyn Hold<'a>, data: &T) -> Hard<'a, R>
        where R: ResidentFromCopyUnchecked<Hard<'a, R>, T>
    {
        Hard::try_hold_copy_unchecked(hold, data).unwrap()
    }

    #[inline]
    pub fn hold_empty(hold: &dyn Hold<'a>) -> Hard<'a, R>
        where R: ResidentFromEmpty<Hard<'a, R>>
    {
        Hard::try_hold_empty(hold).unwrap()
    }

    #[inline]
    pub fn hold_cap(hold: &dyn Hold<'a>, cap: usize) -> Hard<'a, R>
        where R: ResidentWithCapacity<Hard<'a, R>>
    {
        Hard::try_hold_cap(hold, cap).unwrap()
    }

    #[inline]
    pub fn new<T>(data: T) -> Hard<'a, R>
        where R: ResidentFromValue<Hard<'a, R>, T>
    {
        Hard::hold_new(Hold::global(), data)
    }

    #[inline]
    pub fn from_clone<T: ?Sized>(data: &T) -> Hard<'a, R>
        where R: ResidentFromClone<Hard<'a, R>, T>
    {
        Hard::hold_clone(Hold::global(), data)
    }

    #[inline]
    pub unsafe fn from_clone_unchecked<T: ?Sized>(data: &T) -> Hard<'a, R>
        where R: ResidentFromCloneUnchecked<Hard<'a, R>, T>
    {
        Hard::hold_clone_unchecked(Hold::global(), data)
    }

    #[inline]
    pub fn from_copy<T: ?Sized>(data: &T) -> Hard<'a, R>
        where R: ResidentFromCopy<Hard<'a, R>, T>
    {
        Hard::hold_copy(Hold::global(), data)
    }

    #[inline]
    pub unsafe fn from_copy_unchecked<T: ?Sized>(data: &T) -> Hard<'a, R>
        where R: ResidentFromCopyUnchecked<Hard<'a, R>, T>
    {
        Hard::hold_copy_unchecked(Hold::global(), data)
    }

    #[inline]
    pub fn empty() -> Hard<'a, R>
        where R: ResidentFromEmpty<Hard<'a, R>>
    {
        Hard::hold_empty(Hold::global())
    }

    #[inline]
    pub fn with_cap(cap: usize) -> Hard<'a, R>
        where R: ResidentWithCapacity<Hard<'a, R>>
    {
        Hard::hold_cap(Hold::global(), cap)
    }

    /// Constructs a `Hard` lease from a raw pointer returned by `Hard::into_raw`.
    #[inline]
    pub unsafe fn from_raw(data: *mut R::Data) -> Hard<'a, R> {
        Hard {
            data: NonNull::new_unchecked(data),
            data_lifetime: PhantomData,
            meta_lifetime: PhantomData,
            hold_lifetime: PhantomData,
        }
    }

    /// Returns a pointer to the `ArcHeader` preceding the shared resident.
    #[inline]
    fn header(&self) -> *mut ArcHeader<R::Meta> {
        arc::header::<R>(self.data.as_ptr())
    }

    /// Returns the number of hard references to the shared resident.
    /// Does not traverse relocations.
    #[inline]
    pub fn hard_count(&self) -> usize {
        unsafe { (*self.header()).hard_count() }
    }

    /// Returns the number of soft references to the shared resident.
    /// Does not traverse relocations.
    #[inline]
    pub fn soft_count(&self) -> usize {
        unsafe { (*self.header()).soft_count() }
    }

    /// Returns the number of immutable references to the shared resident.
    /// Does not traverse relocations.
    #[inline]
    pub fn ref_count(&self) -> usize {
        unsafe { (*self.header()).ref_count() }
    }

    /// Returns `true` if the shared resident is mutably referenced.
    /// Does not traverse relocations.
    #[inline]
    pub fn is_mut(&self) -> bool {
        unsafe { (*self.header()).is_mut() }
    }

    /// Returns `true` if the shared resident has relocated to a new arc.
    #[inline]
    pub fn is_relocated(&self) -> bool {
        unsafe { (*self.header()).is_relocated() }
    }

    /// Returns `true` if the shared resident is immutably or mutably referenced.
    /// Does not traverse relocations.
    #[inline]
    pub fn is_aliased(&self) -> bool {
        unsafe { (*self.header()).is_aliased() }
    }

    /// Returns a mutable lease to the resident, traversing any completed
    /// relocations, cloning the resident if there are any outstanding leases,
    /// and returning an error if there is an outstanding mutable lease, if
    /// the resident is currently being relocated, or on allocation failure.
    pub fn poll_unique(&self) -> Result<Mut<'a, R>, ArcError>
        where R::Data: TryClone,
              R::Meta: TryClone,
    {
        unsafe {
            // Get a pointer to the shared resident.
            let mut data = self.data.as_ptr();
            // Get a pointer to the arc header preceding the resident.
            let mut header = arc::header::<R>(data);
            // Load the status field; synchronized by subsequent CAS.
            let mut old_status = (*header).status.load(Relaxed);
            // Check if unique, and traverse relocations.
            loop {
                // Check if the resident is uniquely referenced.
                if old_status == arc::UNIQUE_STATUS {
                    // Set the mut flag in the status field.
                    let new_status = old_status | arc::MUT_FLAG;
                    // Atomically update the status field, synchronizing with reference releases.
                    match (*header).status.compare_exchange(old_status, new_status, Acquire, Relaxed) {
                        // CAS succeeded; return a new mutable lease.
                        Ok(_) => return Ok(Mut::from_raw(data)),
                        // CAS failed.
                        Err(_) => (),
                    }
                } else if old_status & arc::RELOCATED_FLAG != 0 {
                    // Synchronize with relocation initiation.
                    atomic::fence(Release);
                    // Get a fat pointer to the relocated resident, synchronizing with relocation completion.
                    let relocation = block::set_address(data, (*header).relocation.load(Acquire));
                    // Check if the relocation has completed.
                    if !relocation.is_null() {
                        // Recurse into the relocated lease.
                        data = relocation;
                        // Get a pointer to the relocated header.
                        header = arc::header::<R>(data);
                        // Reload the status field.
                        old_status = (*header).status.load(Relaxed);
                        // Try to make the relocated resident unique.
                        continue;
                    }
                } else {
                    // Temporarily reify the arc's relocation lease.
                    let relocation = mem::transmute::<*mut R::Data, Hard<'a, R>>(data);
                    // Get an immutable lease to the relocated resident.
                    let lease = relocation.poll_ref();
                    // Discard the borrowed relocation lease.
                    mem::forget(relocation);
                    // Make the relocated lease unique, and return it.
                    return Ref::try_into_unique(lease?);
                }
                // Unable to acquire a mutable lease at this time.
                return Err(ArcError::Contended);
            }
        }
    }

    /// Returns a mutable lease to the resident, traversing any completed
    /// relocations, waiting for any concurrent relocation to complete,
    /// cloning the resident if there are any outstanding leases, and
    /// returning an error on allocation failure or hard refcount overflow.
    pub fn try_to_unique(&self) -> Result<Mut<'a, R>, ArcError>
        where R::Data: TryClone,
              R::Meta: TryClone,
    {
        unsafe {
            // Get a pointer to the shared resident.
            let mut data = self.data.as_ptr();
            // Get a pointer to the arc header preceding the resident.
            let mut header = arc::header::<R>(data);
            // Load the status field; synchronized by subsequent CAS.
            let mut old_status = (*header).status.load(Relaxed);
            // Check if unique, and traverse relocations.
            loop {
                // Check if the resident is uniquely referenced.
                if old_status == arc::UNIQUE_STATUS {
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
                    // Splice the incremented hard reference count into the status field, and set the mut flag.
                    let new_status = new_status | new_hard_count | arc::MUT_FLAG;
                    // Atomically update the status field, synchronizing with reference releases.
                    match (*header).status.compare_exchange_weak(old_status, new_status, Acquire, Relaxed) {
                        // CAS succeeded; return a new mutable lease.
                        Ok(_) => return Ok(Mut::from_raw(data)),
                        // CAS failed; update the status field and try again.
                        Err(status) => old_status = status,
                    }
                } else if old_status & arc::RELOCATED_FLAG != 0 {
                    // Synchronize with relocation initiation.
                    atomic::fence(Release);
                    // Get a fat pointer to the relocated resident, synchronizing with relocation completion.
                    let relocation = block::set_address(data, (*header).relocation.load(Acquire));
                    // Check if the relocation has completed.
                    if !relocation.is_null() {
                        // Recurse into the relocated lease.
                        data = relocation;
                        // Get a pointer to the relocated header.
                        header = arc::header::<R>(data);
                    }
                    // Reload the status field and spin.
                    old_status = (*header).status.load(Relaxed);
                } else {
                    // Temporarily reify the arc's relocation lease.
                    let relocation = mem::transmute::<*mut R::Data, Hard<'a, R>>(data);
                    // Get an immutable lease to the relocated resident.
                    let lease = relocation.poll_ref();
                    // Discard the borrowed relocation lease.
                    mem::forget(relocation);
                    // Make the relocated lease unique, and return it.
                    return Ref::try_into_unique(lease?);
                }
            }
        }
    }

    /// Returns a mutable lease to the resident, traversing any completed
    /// relocations, waiting for any concurrent relocation to complete,
    /// and cloning the resident if there are any outstanding leases.
    ///
    /// # Panics
    ///
    /// Panics on allocation failure or hard refcount overflow.
    pub fn to_unique(&self) -> Mut<'a, R>
        where R::Data: TryClone,
              R::Meta: TryClone,
    {
        self.try_to_unique().unwrap()
    }

    /// Converts this hard lease into a mutable lease to the resident,
    /// traversing any completed relocations, waiting for any concurrent
    /// relocations to complete, cloning the resident if there are any
    /// outstanding leases, and returning an error on allocation failure.
    ///
    /// # Panics
    ///
    /// Panics if the incremented hard reference count overflows `HARD_COUNT_MAX`,
    /// or if the incremented reference count overflows `REF_COUNT_MAX`.
    pub fn try_into_unique(mut self) -> Result<Mut<'a, R>, ArcError>
        where R::Data: TryClone,
              R::Meta: TryClone,
    {
        unsafe {
            // Get a pointer to the shared resident.
            let mut data = self.data.as_ptr();
            // Get a pointer to the arc header preceding the resident.
            let mut header = arc::header::<R>(data);
            // Load the status field; synchronized by subsequent CAS.
            let mut old_status = (*header).status.load(Relaxed);
            // Check if unique, and traverse relocations.
            loop {
                // Check if the resident is uniquely referenced.
                if old_status == arc::UNIQUE_STATUS {
                    // Set the mut flag in the status field.
                    let new_status = old_status | arc::MUT_FLAG;
                    // Atomically update the status field, synchronizing with reference releases.
                    match (*header).status.compare_exchange_weak(old_status, new_status, Acquire, Relaxed) {
                        // CAS succeeded; return a new mutable lease.
                        Ok(_) => return Ok(Mut::from_raw(data)),
                        // CAS failed; update the status field and try again.
                        Err(status) => old_status = status,
                    }
                } else if old_status & arc::RELOCATED_FLAG != 0 {
                    // Synchronize with relocation initiation.
                    atomic::fence(Release);
                    // Get a fat pointer to the relocated resident, synchronizing with relocation completion.
                    let relocation = block::set_address(data, (*header).relocation.load(Acquire));
                    // Check if the relocation has completed.
                    if !relocation.is_null() {
                        // Temporarily reify the arc's relocation lease.
                        let relocation = mem::transmute::<*mut R::Data, Hard<'a, R>>(data);
                        // Recurse into the relocated lease.
                        self = relocation.clone();
                        // Discard the borrowed relocation lease.
                        mem::forget(relocation);
                        // Get a pointer to the relocated resident.
                        data = self.data.as_ptr();
                        // Get a pointer to the relocated header.
                        header = arc::header::<R>(data);
                    }
                    // Reload the status field and spin.
                    old_status = (*header).status.load(Relaxed);
                } else {
                    // Get an immutable lease to the resident, bailing on failure.
                    let lease = self.into_ref();
                    // Make the immutable lease unique, and return it.
                    return Ref::try_into_unique(lease);
                }
            }
        }
    }

    /// Converts this hard lease into a mutable lease to the resident,
    /// traversing any completed relocations, waiting for any concurrent
    /// relocations to complete, and cloning the resident if there are any
    /// outstanding leases.
    ///
    /// # Panics
    ///
    /// Panics on allocation failure.
    pub fn into_unique(self) -> Mut<'a, R>
        where R::Data: TryClone,
              R::Meta: TryClone,
    {
        self.try_into_unique().unwrap()
    }

    /// Returns a new mutable lease to the shared resident, traversing any
    /// complered relocations, and returning an error if the resident is
    /// currently being relocated, if there is an outstanding mutable lease,
    /// or if there is atomic operation contention.
    ///
    /// # Safety
    ///
    /// Mutable leases can coexist with hard and soft leases to the same
    /// resident. This can cause a future deadlock if a thread holding a
    /// mutable lease to a resident attempts to convert another hard or soft
    /// lease to the same resident into a mutable or immutable lease.
    pub unsafe fn poll_mut(&self) -> Result<Mut<'a, R>, ArcError> {
        // Get a pointer to the shared resident.
        let mut data = self.data.as_ptr();
        // Get a pointer to the arc header preceding the resident.
        let mut header = arc::header::<R>(data);
        // Load the status field; synchronized by subsequent CAS.
        let mut old_status = (*header).status.load(Relaxed);
        // Traverse relocations.
        loop {
            // Check if the shared resident can be mutably referenced.
            if old_status & arc::READ_LOCKED_MASK == 0 {
                // Set the mut flag in the status field.
                let new_status = old_status | arc::MUT_FLAG;
                // Atomically update the status field, synchronizing with reference releases.
                match (*header).status.compare_exchange(old_status, new_status, Acquire, Relaxed) {
                    // CAS succeeded; return a new mutable lease.
                    Ok(_) => return Ok(Mut::from_raw(data)),
                    // CAS failed.
                    Err(_) => return Err(ArcError::Contended),
                }
            } else if old_status & arc::RELOCATED_FLAG != 0 {
                // Synchronize with relocation initiation.
                atomic::fence(Release);
                // Get a fat pointer to the relocated resident, synchronizing with relocation completion.
                let relocation = block::set_address(data, (*header).relocation.load(Acquire));
                // Check if the relocation has completed.
                if !relocation.is_null() {
                    // Recurse into the relocated lease.
                    data = relocation;
                    // Get a pointer to the relocated header.
                    header = arc::header::<R>(data);
                    // Load the relocated status field and repeat.
                    old_status = (*header).status.load(Relaxed);
                } else {
                    return Err(ArcError::Relocating);
                }
            } else {
                return Err(ArcError::Aliased);
            }
        }
    }

    /// Returns a new mutable lease to the shared resident, traversing any
    /// completed relocations, waiting for any concurrent relocation to
    /// complete and for any outstanding mutable or immutable leases to drop,
    /// and returning an error if the incremented hard reference count
    /// overflows `HARD_COUNT_MAX`.
    ///
    /// # Safety
    ///
    /// Deadlocks if the current thread already holds a mutable or immutable
    /// lease to the shared resident. Can cause a future deadlock if a thread
    /// holding a mutable lease to a resident attempts to convert another hard
    /// or soft lease to the same resident into a mutable or immutable lease.
    pub unsafe fn try_to_mut(&self) -> Result<Mut<'a, R>, ArcError> {
        // Get a pointer to the shared resident.
        let mut data = self.data.as_ptr();
        // Get a pointer to the arc header preceding the resident.
        let mut header = arc::header::<R>(data);
        // Load the status field; synchronized by subsequent CAS.
        let mut old_status = (*header).status.load(Relaxed);
        // Traverse relocations, and spin until a mutable reference is acquired.
        loop {
            // Check if the shared resident can be mutably referenced.
            if old_status & arc::READ_LOCKED_MASK == 0 {
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
                // Splice the incremented hard reference count into the status field, and set the mut flag.
                let new_status = new_status | new_hard_count | arc::MUT_FLAG;
                // Atomically update the status field, synchronizing with reference releases.
                match (*header).status.compare_exchange_weak(old_status, new_status, Acquire, Relaxed) {
                    // CAS succeeded; return a new mutable lease.
                    Ok(_) => return Ok(Mut::from_raw(data)),
                    // CAS failed; update the status field and try again.
                    Err(status) => old_status = status,
                }
            } else {
                // Check if the shared resident is concurrently relocating.
                if old_status & arc::RELOCATED_FLAG != 0 {
                    // Synchronize with relocation initiation.
                    atomic::fence(Release);
                    // Get a fat pointer to the relocated resident, synchronizing with relocation completion.
                    let relocation = block::set_address(data, (*header).relocation.load(Acquire));
                    // Check if the relocation has completed.
                    if !relocation.is_null() {
                        // Recurse into the relocated lease.
                        data = relocation;
                        // Get a pointer to the relocated header.
                        header = arc::header::<R>(data);
                    }
                }
                // Concurrently relocating; reload the status field and spin.
                old_status = (*header).status.load(Relaxed);
            }
        }
    }

    /// Returns a new mutable lease to the shared resident, traversing any
    /// completed relocations, and waiting for any concurrent relocation to
    /// complete and for any outstanding mutable or immutable leases to drop.
    ///
    /// # Safety
    ///
    /// Deadlocks if the current thread already holds a mutable or immutable
    /// lease to the shared resident. Can cause a future deadlock if a thread
    /// holding a mutable lease to a resident attempts to convert another hard
    /// or soft lease to the same resident into a mutable or immutable lease.
    ///
    /// # Panics
    ///
    /// Panics if the incremented hard reference count overflows `HARD_COUNT_MAX`.
    pub unsafe fn to_mut(&self) -> Mut<'a, R> {
        self.try_to_mut().unwrap()
    }

    /// Converts this hard lease into a mutable lease to the shared resident,
    /// traversing any completed relocations, waiting for any concurrent
    /// relocations to complete and for any outstanding mutable or immutable
    /// leases to drop, and returning an error if an incremented hard reference
    /// count overflows `HARD_COUNT_MAX`.
    ///
    /// # Safety
    ///
    /// Deadlocks if the current thread already holds a mutable or immutable
    /// lease to the shared resident. Can cause a future deadlock if a thread
    /// holding a mutable lease to a resident attempts to convert another hard
    /// or soft lease to the same resident into a mutable or immutable lease.
    pub unsafe fn try_into_mut(self) -> Result<Mut<'a, R>, ArcError> {
        // Get a pointer to the shared resident.
        let data = self.data.as_ptr();
        // Get a pointer to the arc header preceding the resident.
        let header = arc::header::<R>(data);
        // Load the status field; synchronized by subsequent CAS.
        let mut old_status = (*header).status.load(Relaxed);
        // Spin until a mutable reference is acquired.
        loop {
            // Check if the shared resident can be mutably referenced.
            if old_status & arc::READ_LOCKED_MASK == 0 {
                // Set the mut flag in the status field.
                let new_status = old_status | arc::MUT_FLAG;
                // Atomically update the status field, synchronizing with reference releases.
                match (*header).status.compare_exchange_weak(old_status, new_status, Acquire, Relaxed) {
                    // CAS succeeded.
                    Ok(_) => {
                        // Discard the original lease, whose hard reference we took.
                        mem::forget(self);
                        // Return a new mutable lease.
                        return Ok(Mut::from_raw(data));
                    },
                    // CAS failed; update the status field and try again.
                    Err(status) => old_status = status,
                }
            } else {
                // Check if the shared resident is concurrently relocating.
                if old_status & arc::RELOCATED_FLAG != 0 {
                    // Synchronize with relocation initiation.
                    atomic::fence(Release);
                    // Get a fat pointer to the relocated resident, synchronizing with relocation completion.
                    let relocation = block::set_address(data, (*header).relocation.load(Acquire));
                    // Check if the relocation has completed.
                    if !relocation.is_null() {
                        // Temporarily reify the arc's relocation lease.
                        let relocation = mem::transmute::<*mut R::Data, Hard<'a, R>>(relocation);
                        // Recurse into the relocated lease.
                        let lease = relocation.try_to_mut();
                        // Discard the borrowed relocation lease.
                        mem::forget(relocation);
                        // Return the newly acquired lease.
                        return lease;
                    }
                }
                // Concurrently relocating; reload the status field and spin.
                old_status = (*header).status.load(Relaxed);
            }
        }
    }

    /// Converts this hard lease into a mutable lease to the shared resident,
    /// traversing any completed relocations, and waiting for any concurrent
    /// relocations to complete and for any outstanding mutable or immutable
    /// leases to drop.
    ///
    /// # Safety
    ///
    /// Deadlocks if the current thread already holds a mutable or immutable
    /// lease to the shared resident. Can cause a future deadlock if a thread
    /// holding a mutable lease to a resident attempts to convert another hard
    /// or soft lease to the same resident into a mutable or immutable lease.
    ///
    /// # Panics
    ///
    /// Panics if an incremented hard reference count overflows `HARD_COUNT_MAX`.
    pub unsafe fn into_mut(self) -> Mut<'a, R> {
        self.try_into_mut().unwrap()
    }

    /// Returns a new immutable lease to the shared resident, traversing any
    /// completed relocations, and returning an error if the reisdent is
    /// currently being relocated, or if there is an outstanding mutable lease,
    /// or if there is atomic operation contention, or if obtaining the lease
    /// would cause a reference count overflow.
    pub fn poll_ref(&self) -> Result<Ref<'a, R>, ArcError> {
        unsafe {
            // Get a pointer to the shared resident.
            let mut data = self.data.as_ptr();
            // Get a pointer to the arc header preceding the resident.
            let mut header = arc::header::<R>(data);
            // Load the status field; synchronized by subsequent CAS.
            let mut old_status = (*header).status.load(Relaxed);
            // Traverse relocations.
            loop {
                // Check if the shared resident is not mutably referenced, and is not concurrently relocating.
                if old_status & arc::WRITE_LOCKED_MASK == 0 {
                    // Extract the hard reference count from the status field.
                    let old_hard_count = old_status & arc::HARD_COUNT_MASK;
                    // Increment the hard reference count.
                    let new_hard_count = old_hard_count.wrapping_add(1);
                    // Check if the incremented hard reference count overflows its bit field.
                    if new_hard_count > arc::HARD_COUNT_MAX {
                        return Err(ArcError::HardCountOverflow);
                    }
                    // Extract the immutable reference count from the status field.
                    let old_ref_count = (old_status & arc::REF_COUNT_MASK) >> arc::REF_COUNT_SHIFT;
                    // Increment the immutable reference count.
                    let new_ref_count = old_ref_count.wrapping_add(1);
                    // Check if the incremented shared reference count overflows its bit field.
                    if new_ref_count > arc::REF_COUNT_MAX {
                        return Err(ArcError::RefCountOverflow);
                    }
                    // Clear the hard and immutable reference count bit fields.
                    let new_status = old_status & !(arc::HARD_COUNT_MASK | arc::REF_COUNT_MASK);
                    // Splice the incremented hard and immutable reference counts into the status field.
                    let new_status = new_status | new_hard_count | new_ref_count << arc::REF_COUNT_SHIFT;
                    // Atomically update the status field, synchronizing with reference releases.
                    match (*header).status.compare_exchange(old_status, new_status, Acquire, Relaxed) {
                        // CAS succeeded; return a new immutable lease.
                        Ok(_) => return Ok(Ref::from_raw(data)),
                        // CAS failed; abort.
                        Err(_) => return Err(ArcError::Contended),
                    }
                } else if old_status & arc::RELOCATED_FLAG != 0 {
                    // Synchronize with relocation initiation.
                    atomic::fence(Release);
                    // Get a fat pointer to the relocated resident, synchronizing with relocation completion.
                    let relocation = block::set_address(data, (*header).relocation.load(Acquire));
                    // Check if the relocation has completed.
                    if !relocation.is_null() {
                        // Recurse into the relocated lease.
                        data = relocation;
                        // Get a pointer to the relocated header.
                        header = arc::header::<R>(data);
                        // Load the relocated status field and repeat.
                        old_status = (*header).status.load(Relaxed);
                    } else {
                        return Err(ArcError::Relocating);
                    }
                } else {
                    return Err(ArcError::Aliased);
                }
            }
        }
    }

    /// Returns a new immutable lease to the shared resident, traversing any
    /// completed relocations, waiting for any concurrent relocation to
    /// complete and for any outstanding mutable lease to drop, and returning
    /// an error if the incremented hard reference count overflows `HARD_COUNT_MAX`,
    /// or if the incremented reference count overflows `REF_COUNT_MAX`.
    pub fn try_to_ref(&self) -> Result<Ref<'a, R>, ArcError> {
        unsafe {
            // Get a pointer to the shared resident.
            let mut data = self.data.as_ptr();
            // Get a pointer to the arc header preceding the resident.
            let mut header = arc::header::<R>(data);
            // Load the status field; synchronized by subsequent CAS.
            let mut old_status = (*header).status.load(Relaxed);
            // Traverse relocations, and spin until an immutable reference is acquired.
            loop {
                // Check if the shared resident is not mutably referenced, and is not concurrently relocating.
                if old_status & arc::WRITE_LOCKED_MASK == 0 {
                    // Extract the hard reference count from the status field.
                    let old_hard_count = old_status & arc::HARD_COUNT_MASK;
                    // Increment the hard reference count.
                    let new_hard_count = old_hard_count.wrapping_add(1);
                    // Check if the incremented hard reference count overflows its bit field.
                    if new_hard_count > arc::HARD_COUNT_MAX {
                        return Err(ArcError::HardCountOverflow);
                    }
                    // Extract the immutable reference count from the status field.
                    let old_ref_count = (old_status & arc::REF_COUNT_MASK) >> arc::REF_COUNT_SHIFT;
                    // Increment the immutable reference count.
                    let new_ref_count = old_ref_count.wrapping_add(1);
                    // Check if the incremented shared reference count overflows its bit field.
                    if new_ref_count > arc::REF_COUNT_MAX {
                        return Err(ArcError::RefCountOverflow);
                    }
                    // Clear the hard and immutable reference count bit fields.
                    let new_status = old_status & !(arc::HARD_COUNT_MASK | arc::REF_COUNT_MASK);
                    // Splice the incremented hard and immutable reference counts into the status field.
                    let new_status = new_status | new_hard_count | new_ref_count << arc::REF_COUNT_SHIFT;
                    // Atomically update the status field, synchronizing with reference releases.
                    match (*header).status.compare_exchange_weak(old_status, new_status, Acquire, Relaxed) {
                        // CAS succeeded; return a new immutable lease.
                        Ok(_) => return Ok(Ref::from_raw(data)),
                        // CAS failed; update the status field and try again.
                        Err(status) => old_status = status,
                    }
                } else {
                    // Check if the shared resident is concurrently relocating.
                    if old_status & arc::RELOCATED_FLAG != 0 {
                        // Synchronize with relocation initiation.
                        atomic::fence(Release);
                        // Get a fat pointer to the relocated resident, synchronizing with relocation completion.
                        let relocation = block::set_address(data, (*header).relocation.load(Acquire));
                        // Check if the relocation has completed.
                        if !relocation.is_null() {
                            // Recurse into the relocated lease.
                            data = relocation;
                            // Get a pointer to the relocated header.
                            header = arc::header::<R>(data);
                        }
                    }
                    // Concurrently relocating; reload the status field and spin.
                    old_status = (*header).status.load(Relaxed);
                }
            }
        }
    }

    /// Returns a new immutable lease to the shared resident, traversing any
    /// completed relocations, waiting for any concurrent relocation to
    /// complete, and for any outstanding mutable lease to drop.
    ///
    /// # Panics
    ///
    /// Panics if the incremented hard reference count overflows `HARD_COUNT_MAX`,
    /// or if the incremented reference count overflows `REF_COUNT_MAX`.
    pub fn to_ref(&self) -> Ref<'a, R> {
        self.try_to_ref().unwrap()
    }

    /// Converts this hard lease into an immutable lease to the shared
    /// resident, traversing any completed relocations, waiting for any
    /// concurrent relocation to complete and for any outstanding mutable
    /// leases to drop, and returning an error if the incremented hard
    /// reference count overflows `HARD_COUNT_MAX`, or if the incremented
    /// immutable reference count overflows `REF_COUNT_MAX`.
    pub fn try_into_ref(self) -> Result<Ref<'a, R>, ArcError> {
        unsafe {
            // Get a pointer to the shared resident.
            let data = self.data.as_ptr();
            // Get a pointer to the arc header preceding the resident.
            let header = arc::header::<R>(data);
            // Load the status field; synchronized by subsequent CAS.
            let mut old_status = (*header).status.load(Relaxed);
            // Traverse relocations, and spin until an immutable reference is acquired.
            loop {
                // Check if the shared resident is not mutably referenced, and is not concurrently relocating.
                if old_status & arc::WRITE_LOCKED_MASK == 0 {
                    // Extract the immutable reference count from the status field.
                    let old_ref_count = (old_status & arc::REF_COUNT_MASK) >> arc::REF_COUNT_SHIFT;
                    // Increment the immutable reference count.
                    let new_ref_count = old_ref_count.wrapping_add(1);
                    // Check if the incremented shared reference count overflows its bit field.
                    if new_ref_count > arc::REF_COUNT_MAX {
                        return Err(ArcError::RefCountOverflow);
                    }
                    // Clear the immutable reference count bit field.
                    let new_status = old_status & !arc::REF_COUNT_MASK;
                    // Splice the incremented immutable reference count into the status field.
                    let new_status = new_status | new_ref_count << arc::REF_COUNT_SHIFT;
                    // Atomically update the status field, synchronizing with reference releases.
                    match (*header).status.compare_exchange_weak(old_status, new_status, Acquire, Relaxed) {
                        // CAS succeeded.
                        Ok(_) => {
                            // Discard the original lease, whose hard reference we took.
                            mem::forget(self);
                            // Return a new immutable lease.
                            return Ok(Ref::from_raw(data));
                        },
                        // CAS failed; update the status field and try again.
                        Err(status) => old_status = status,
                    }
                } else {
                    // Check if the shared resident is concurrently relocating.
                    if old_status & arc::RELOCATED_FLAG != 0 {
                        // Synchronize with relocation initiation.
                        atomic::fence(Release);
                        // Get a fat pointer to the relocated resident, synchronizing with relocation completion.
                        let relocation = block::set_address(data, (*header).relocation.load(Acquire));
                        // Check if the relocation has completed.
                        if !relocation.is_null() {
                            // Temporarily reify the arc's relocation lease.
                            let relocation = mem::transmute::<*mut R::Data, Hard<'a, R>>(relocation);
                            // Recurse into the relocated lease.
                            let lease = relocation.try_to_ref();
                            // Discard the borrowed relocation lease.
                            mem::forget(relocation);
                            // Return the newly acquired lease.
                            return lease;
                        }
                    }
                    // Concurrently relocating; reload the status field and spin.
                    old_status = (*header).status.load(Relaxed);
                }
            }
        }
    }

    /// Converts this hard lease into an immutable lease to the shared
    /// resident, traversing any completed relocations, and waiting for any
    /// concurrent relocation to complete and for any outstanding mutable
    /// leases to drop.
    ///
    /// # Panics
    ///
    /// Panics if the incremented hard reference count overflows `HARD_COUNT_MAX`,
    /// or if the incremented reference count overflows `REF_COUNT_MAX`.
    pub fn into_ref(self) -> Ref<'a, R> {
        self.try_into_ref().unwrap()
    }

    /// Returns a new soft lease to the shared resident, without traversing any
    /// relocations, returning an error if the incremented soft reference count
    /// overflows `SOFT_COUNT_MAX`.
    pub fn try_to_soft(&self) -> Result<Soft<'a, R>, ArcError> {
        unsafe {
            // Get a pointer to the shared resident.
            let data = self.data.as_ptr();
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
    }

    /// Returns a new soft lease to the shared resident, without traversing any
    /// relocations.
    ///
    /// # Panics
    ///
    /// Panics if the incremented soft reference count overflows `SOFT_COUNT_MAX`.
    pub fn to_soft(&self) -> Soft<'a, R> {
        self.try_to_soft().unwrap()
    }

    /// Converts this hard lease into a soft lease to the shared resident,
    /// without traversing any relocations, returning an error if the
    /// incremented soft reference count overflows `SOFT_COUNT_MAX`.
    pub fn try_into_soft(self) -> Result<Soft<'a, R>, ArcError> {
        unsafe {
            // Get a pointer to the shared resident.
            let data = self.data.as_ptr();
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
                // Clear the hard and soft reference count bit fields.
                let new_status = old_status & !(arc::HARD_COUNT_MASK | arc::SOFT_COUNT_MASK);
                // Splice the decremented hard and incremented soft reference counts into the status field.
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
                        // Discard the original lease, whose hard reference we released.
                        mem::forget(self);
                        // Return a new Soft lease.
                        return Ok(Soft::from_raw(data));
                    },
                    // CAS failed; update the status field and try again.
                    Err(status) => old_status = status,
                }
            }
        }
    }

    /// Converts this hard lease into a soft lease to the shared resident,
    /// without traversing any relocations.
    ///
    /// # Panics
    ///
    /// Panics if the incremented soft reference count overflows `SOFT_COUNT_MAX`.
    pub fn into_soft(self) -> Soft<'a, R> {
        self.try_into_soft().unwrap()
    }

    /// Converts this hard lease into a raw pointer to the shared resident.
    /// Use `Hard::from_raw` to reconstitute the returned pointer back into
    /// a hard lease.
    ///
    /// # Safety
    ///
    /// The shared resident is not pinned to the returned memory address, and
    /// may be concurrently relocated at any time. A memory leak will occur
    /// unless the returned pointer is eventually converted back into a hard
    /// lease and dropped.
    #[inline]
    pub unsafe fn into_raw(self) -> *mut R::Data {
        let data = self.data.as_ptr();
        mem::forget(self);
        data
    }

    /// Returns an immutable lease to the shared resident, traversing any
    /// completed moves, without waiting.
    ///
    /// # Panics
    ///
    /// Panics if the the shared resident is mutably aliased.
    pub fn borrow(&self) -> Ref<'a, R> {
        loop {
            // Try to acquire an immutable reference.
            match self.poll_ref() {
                // Immutable lease acquired.
                Ok(lease) => return lease,
                // Concurrently relocating, try again.
                Err(ArcError::Relocating) => (),
                // Lock contention encountered, try again.
                Err(ArcError::Contended) => (),
                // Immutable reference unavailable.
                Err(error) => panic!("{:?}", error),
            }
        }
    }

    /// Returns a raw pointer to the shared resident.
    ///
    /// # Safety
    ///
    /// The shared resident may be uninitialized, or mutably aliased,
    /// or may have been have relocated.
    #[inline]
    pub unsafe fn as_ptr_unchecked(&self) -> *mut R::Data {
        self.data.as_ptr()
    }

    /// Consumes this hard lease, traversing any completed relocations,
    /// and returns the shared resident; returns an error if there are
    /// any outstanding hard, mutable, or immutable leases.
    pub fn try_unwrap(mut self) -> Result<R::Target, Hard<'a, R>> where R: ResidentUnwrap<Hard<'a, R>> {
        unsafe {
            // Get a pointer to the shared resident.
            let mut data = self.data.as_ptr();
            // Get the alignment of the resident.
            let align = mem::align_of_val(&*data);
            // Get the offset of the resident in the arc structure by rounding up
            // the size of the arc header to the alignment of the resident.
            let offset = mem::size_of::<ArcHeader<R::Meta>>()
                .wrapping_add(align).wrapping_sub(1) & !align.wrapping_sub(1);
            // Get a pointer to the arc header by subtracting the resident's
            // offset in the arc structure.
            let mut header = (data as *mut u8).wrapping_sub(offset) as *mut ArcHeader<R::Meta>;
            // Compute the total size of the arc structure.
            let size = offset.wrapping_add(R::resident_size(data, &mut (*header).meta));
            // Load the status field; synchronized by subsequent CAS.
            let mut old_status = (*header).status.load(Relaxed);
            // Spin until the hard reference is released.
            loop {
                // Check if the shared resident hasn't relocated.
                if old_status & arc::RELOCATED_FLAG == 0 {
                    // Extract the hard reference count from the status field.
                    let old_hard_count = old_status & arc::HARD_COUNT_MASK;
                    // Check if the resident has multiple hard references.
                    if old_hard_count != 1 {
                        // Can't unwrap an aliased resident.
                        return Err(self);
                    }
                    // Clear the hard reference count bit field.
                    let new_status = old_status & !arc::HARD_COUNT_MASK;
                    // Extract the soft reference count from the status field.
                    let old_soft_count = (old_status & arc::SOFT_COUNT_MASK) >> arc::SOFT_COUNT_SHIFT;
                    // Check if all soft references have dropped.
                    if old_soft_count == 0 {
                        // Store the new status field; can't fail because we're the last reference of any kind.
                        (*header).status.store(new_status, Relaxed);
                        // Read the resident out of the arc structure.
                        let resident = R::resident_unwrap(&self);
                        // Drop the arc header.
                        (*header).drop::<R>(data);
                        // Get the block of memory containing the arc structure.
                        let block = Block::from_raw_parts(header as *mut u8, size);
                        // Deallocate the block.
                        AllocTag::from_ptr(header as *mut u8).dealloc(block);
                        // Discard the original lease, whose hard reference we released.
                        mem::forget(self);
                        // Return the unwrapped resident.
                        return Ok(resident);
                    } else {
                        // Convert our hard reference into a soft reference to avoid racing with other soft refs.
                        let new_soft_count = old_soft_count.wrapping_add(1);
                        // Check if the incremented soft reference count overflows its bit field.
                        if new_soft_count > arc::SOFT_COUNT_MAX {
                            return Err(self);
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
                                let resident = R::resident_unwrap(&self);
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
                } else {
                    // Synchronize with relocation initiation.
                    atomic::fence(Release);
                    // Get a fat pointer to the relocated resident, synchronizing with relocation completion.
                    let relocation = block::set_address(data, (*header).relocation.load(Acquire));
                    // Check if the relocation has completed.
                    if !relocation.is_null() {
                        // Temporarily reify the arc's relocation lease.
                        let relocation = mem::transmute::<*mut R::Data, Hard<'a, R>>(relocation);
                        // Recurse into the relocated lease.
                        self = relocation.clone();
                        // Discard the borrowed relocation lease.
                        mem::forget(relocation);
                        // Get a pointer to the relocated resident.
                        data = self.data.as_ptr();
                        // Get a pointer to the relocated header.
                        header = (data as *mut u8).wrapping_sub(offset) as *mut ArcHeader<R::Meta>;
                        // Load the relocated status field and try again.
                        old_status = (*header).status.load(Relaxed);
                    } else {
                        // Can't unwrap a relocating resident.
                        return Err(self);
                    }
                }
            }
        }
    }

    /// Consumes this hard lease, traversing any completed relocations,
    /// and returns the shared resident.
    ///
    /// # Panics
    ///
    /// Panics if there are any outstanding hard, mutable, or immutable leases.
    pub fn unwrap(self) -> R::Target where R: ResidentUnwrap<Hard<'a, R>> {
        match self.try_unwrap() {
            Ok(resident) => resident,
            Err(_) => panic!("aliased resident"),
        }
    }
}

impl<'a, R: Resident> Holder<'a> for Hard<'a, R> {
    #[inline]
    fn holder(&self) -> &'a dyn Hold<'a> {
        AllocTag::from_ptr(self.header() as *mut u8).holder()
    }
}

impl<'a, R: Resident> Lease for Hard<'a, R> {
    type Data = R::Data;

    type Meta = R::Meta;

    #[inline]
    fn data(&self) -> *mut R::Data {
        self.data.as_ptr()
    }

    #[inline]
    fn meta(&self) -> *mut R::Meta {
        unsafe { &mut (*self.header()).meta }
    }
}

impl<'a, R: ResidentHash<Ref<'a, R>>> Hash for Hard<'a, R> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash(&self.borrow(), state);
    }
}

impl<'a, R: ResidentDisplay<Ref<'a, R>>> Display for Hard<'a, R> {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.borrow(), f)
    }
}

impl<'a, R: ResidentDebug<Ref<'a, R>>> Debug for Hard<'a, R> {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Debug::fmt(&self.borrow(), f)
    }
}

impl<'a, R: Resident> Pointer for Hard<'a, R> {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Pointer::fmt(&self.data.as_ptr(), f)
    }
}

impl<'a, R: Resident> TryClone for Hard<'a, R> {
    fn try_clone(&self) -> Result<Hard<'a, R>, HoldError> {
        unsafe {
            // Get a pointer to the shared resident.
            let data = self.data.as_ptr();
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
                    return Err(HoldError::Unsupported("hard count overflow"));
                }
                // Clear the hard reference count bit field.
                let new_status = old_status & !arc::HARD_COUNT_MASK;
                // Splice the incremented hard reference count into the status field.
                let new_status = new_status | new_hard_count;
                // Atomically update the status field, synchronizing with reference releases.
                match (*header).status.compare_exchange_weak(old_status, new_status, Acquire, Relaxed) {
                    // CAS succeeded; return a new hard lease.
                    Ok(_) => return Ok(Hard::from_raw(data)),
                    // CAS failed; update the status field and try again.
                    Err(status) => old_status = status,
                }
            }
        }
    }
}

impl<'a, R: Resident> Clone for Hard<'a, R> {
    fn clone(&self) -> Hard<'a, R> {
        self.try_clone().unwrap()
    }
}

impl<'a, 'b, R: ResidentStow<'b, Hard<'a, R>, Hard<'b, R>>> Stow<'b, Hard<'b, R>> for Hard<'a, R> {
    unsafe fn stow(src: *mut Hard<'a, R>, dst: *mut Hard<'b, R>, hold: &Hold<'b>) -> Result<(), HoldError> {
        // Get a pointer to the source resident.
        let src_data = (*src).data.as_ptr();
        // Get the alignment of the resident.
        let src_align = mem::align_of_val(&*src_data);
        // Get the offset of the resident in the arc structure by rounding up
        // the size of the arc header to the alignment of the resident.
        let src_offset = mem::size_of::<ArcHeader<R::Meta>>()
            .wrapping_add(src_align).wrapping_sub(1) & !src_align.wrapping_sub(1);
        // Get a pointer to the source header by subtracting the resident's
        // offset in the arc structure.
        let src_header = (src_data as *mut u8).wrapping_sub(src_offset) as *mut ArcHeader<R::Meta>;
        // Load the status field; synchronized by subsequent CAS.
        let mut old_status = (*src_header).status.load(Relaxed);
        // Spin until the relocated flag is set.
        loop {
            // Check if the source resident can be mutably referenced.
            if old_status & arc::READ_LOCKED_MASK == 0 {
                // Set the relocated flag in the status field.
                let new_status = old_status | arc::RELOCATED_FLAG;
                // Atomically update the status field, synchronizing with relocation initiation and traversal.
                match (*src_header).status.compare_exchange_weak(old_status, new_status, SeqCst, Relaxed) {
                    // CAS succeeded.
                    Ok(_) => {
                        // Update the status field.
                        old_status = new_status;
                        // Move initiated.
                        break;
                    },
                    // CAS failed; update the status field and try again.
                    Err(status) => old_status = status,
                }
            } else {
                // Check if the source resident has already relocated.
                if old_status & arc::RELOCATED_FLAG != 0 {
                    // Synchronize with relocation initiation.
                    atomic::fence(Release);
                    // Get a fat pointer to the source resident, synchronizing with relocation completion.
                    let relocation = block::set_address(src_data, (*src_header).relocation.load(Acquire));
                    // Check if the relocation has completed.
                    if !relocation.is_null() {
                        // Temporarily reify the arc's relocation lease.
                        let relocation = mem::transmute::<*mut R::Data, Hard<'b, R>>(relocation);
                        // Set the destination lease to a clone of the relocation lease.
                        ptr::write(dst, relocation.clone());
                        // Discard the borrowed relocation lease.
                        mem::forget(relocation);
                        // Return successfully.
                        return Ok(());
                    }
                }
                // Concurrently relocating; reload the status field and spin.
                old_status = (*src_header).status.load(Relaxed);
            }
        }
        // Compute the layout of the destination arc structure, capturing the offset of its resident field.
        let (dst_layout, dst_offset) = Layout::for_type::<ArcHeader<R::Meta>>()
            .extended(R::new_resident_layout(&*src))?;
        // Allocate a block of memory to hold the new arc structure, bailing on failure.
        let dst_block = match hold.alloc(dst_layout) {
            // Allocation succeeded.
            Ok(block) => block,
            // Allocation failed.
            Err(error) => {
                // Spin until the relocated flag is unset.
                loop {
                    // Unset the relocated flag in the status field.
                    let new_status = old_status & !arc::RELOCATED_FLAG;
                    // Atomically update the status field, synchronizing with reference acquires and releases.
                    match (*src_header).status.compare_exchange_weak(old_status, new_status, SeqCst, Relaxed) {
                        // CAS succeeded; return the allocation error.
                        Ok(_) => return Err(error),
                        // CAS failed; update the status field and try again.
                        Err(status) => old_status = status,
                    }
                }
            },
        };
        // Get a pointer to the header of the new arc.
        let dst_header = dst_block.as_ptr() as *mut ArcHeader<R::Meta>;
        // Initialize the new arc's relocation address to zero.
        ptr::write(&mut (*dst_header).relocation, AtomicUsize::new(0));
        // Initialize the new arc's status field with two hard references,
        // owned by the relocation reference of the source lease, and the other
        // owned by the destination lease.
        ptr::write(&mut (*dst_header).status, AtomicUsize::new(2));
        // Get a fat pointer to the destination resident.
        let dst_data = block::set_address(src_data, (dst_header as usize).wrapping_add(dst_offset));
        // Initialize the destination lease.
        ptr::write(dst, Hard::from_raw(dst_data));
        // Try to stow the resident.
        if let err @ Err(_) = R::resident_stow(&mut *src, &mut *dst, hold) {
            // Free the newly allocated arc.
            hold.dealloc(dst_block);
            // Spin until the relocated flag is unset.
            loop {
                // Unset the relocated flag in the status field.
                let new_status = old_status & !arc::RELOCATED_FLAG;
                // Atomically update the status field, synchronizing with relocation initiation and traversal.
                match (*src_header).status.compare_exchange_weak(old_status, new_status, SeqCst, Relaxed) {
                    // CAS succeeded; return the stow error.
                    Ok(_) => return err,
                    // CAS failed; update the status field and try again.
                    Err(status) => old_status = status,
                }
            }
        }
        // Write the relocation address of the new resident into the old arc header,
        // synchronizing with relocation traversals, completing the relocation.
        (*src_header).relocation.store(dst_data as *mut u8 as usize, Release);
        // Return successfully.
        Ok(())
    }

    unsafe fn unstow(src: *mut Hard<'a, R>, dst: *mut Hard<'b, R>) {
        // Get a pointer to the source resident.
        let src_data = (*src).data.as_ptr();
        // Get the alignment of the resident.
        let align = mem::align_of_val(&*src_data);
        // Get the offset of the resident in the arc structure by rounding up
        // the size of the arc header to the alignment of the resident.
        let offset = mem::size_of::<ArcHeader<R::Meta>>()
            .wrapping_add(align).wrapping_sub(1) & !align.wrapping_sub(1);
        // Get a pointer to the source header.
        let src_header = (src_data as *mut u8).wrapping_sub(offset) as *mut ArcHeader<R::Meta>;
        // Get a pointer to the destination resident.
        let dst_data = (*dst).data.as_ptr();
        // Get a pointer to the destination header.
        let dst_header = (dst_data as *mut u8).wrapping_sub(offset) as *mut ArcHeader<R::Meta>;
        // Load the destination status field; synchronized by subsequent CAS.
        let mut old_dst_status = (*dst_header).status.load(Relaxed);
        // Spin until the source arc's relocation lease has been dropped.
        loop {
            // Check if the source arc's relocation lease is unique.
            if old_dst_status == arc::UNIQUE_STATUS {
                // Forward lease is unique, clear the source arc's relocation address.
                // Can't race with relocation traversal because the src and dst
                // pointers can't be aliased outside the call stack.
                (*src_header).relocation.store(0, Release);
                // Load the source status field; synchronized by subsequent CAS.
                let mut old_src_status = (*src_header).status.load(Relaxed);
                // Spin until the source relocated flag is unset.
                loop {
                    // Unset the relocated flag in the status field.
                    let new_src_status = old_src_status & !arc::RELOCATED_FLAG;
                    // Atomically update the status field, synchronizing with relocation initiation and traversal.
                    match (*src_header).status.compare_exchange_weak(old_src_status, new_src_status, SeqCst, Relaxed) {
                        // CAS succeeded; proceed.
                        Ok(_) => break,
                        // CAS failed; update the status field and try again.
                        Err(status) => old_src_status = status,
                    }
                }
                // Unstow the resident.
                R::resident_unstow(&mut *src, &mut *dst);
                // Compute the total size of the arc structure.
                let size = offset.wrapping_add(R::resident_size(dst_data, &mut (*dst_header).meta));
                // Get the memory block containing the destination arc.
                let dst_block = Block::from_raw_parts(dst as *mut u8, size);
                // Deallocate the destination arc.
                AllocTag::from_ptr(dst_header as *mut u8).dealloc(dst_block);
                // Move has been fully reverted.
                return;
            } else {
                // Forward lease is aliased (within the current call stack);
                // release the source arc's hard reference.
                // Extract the hard reference count from the status field.
                let old_hard_count = old_dst_status & arc::HARD_COUNT_MASK;
                // Decrement the hard reference count, checking for underflow.
                let new_hard_count = match old_hard_count.checked_sub(1) {
                    Some(hard_count) => hard_count,
                    None => panic!("hard count underflow"),
                };
                // Clear the hard reference count field.
                let new_dst_status = old_dst_status & !arc::HARD_COUNT_MASK;
                // Splice the decremented hard reference count into the status field.
                let new_dst_status = new_dst_status | new_hard_count;
                // Atomically update the destination status field, synchronizing with reference acquires.
                match (*dst_header).status.compare_exchange_weak(old_dst_status, new_dst_status, Release, Relaxed) {
                    // CAS succeeded; hard reference released; relocation has been fully reverted.
                    Ok(_) => return,
                    // CAS failed; update the status field and try again.
                    Err(status) => old_dst_status = status,
                }
            }
        }
    }
}

unsafe impl<'a, #[may_dangle] R: Resident> Drop for Hard<'a, R> {
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
            // Spin until the hard reference has been released.
            loop {
                // Extract the hard reference count from the status field.
                let old_hard_count = old_status & arc::HARD_COUNT_MASK;
                // Decrement the hard reference count, checking for underflow.
                let new_hard_count = match old_hard_count.checked_sub(1) {
                    Some(hard_count) => hard_count,
                    None => panic!("hard count underflow"),
                };
                // Clear the hard reference count field.
                let new_status = old_status & !arc::HARD_COUNT_MASK;
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
                        // Check if the resident hasn't relocated.
                        if new_status & arc::RELOCATED_FLAG == 0 {
                            // Drop the shared resident.
                            R::resident_drop(data, &mut (*header).meta);
                        }
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
                                // Check if the resident hasn't relocated.
                                if new_status & arc::RELOCATED_FLAG == 0 {
                                    // Drop the shared resident.
                                    R::resident_drop(data, &mut (*header).meta);
                                }
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
