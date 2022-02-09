use core::fmt::{self, Pointer, Formatter};
use core::marker::PhantomData;
use core::mem;
use core::ptr::{self, NonNull};
use core::sync::atomic;
use core::sync::atomic::Ordering::{Acquire, Relaxed, Release, SeqCst};
use crate::block::{self, Block};
use crate::alloc::{AllocTag, Hold, Holder, HoldError, Stow, TryClone};
use crate::lease::{arc, ArcHeader, ArcError, Lease, Mut, Ref, Hard};
use crate::resident::{Resident, ResidentStow};

/// A thread-safe, atomically counted, undereferenceable soft reference to a
/// `Resident` occupying a shared, `Hold`-allocated memory block.
pub struct Soft<'a, R: Resident> {
    /// Pointer to the resident memory block.
    data: NonNull<R::Data>,
    /// Variant over ArcHeader<R::Meta>, with drop check.
    meta_lifetime: PhantomData<ArcHeader<R::Meta>>,
    /// Variant over 'a.
    hold_lifetime: PhantomData<&'a ()>,
}

unsafe impl<'a, R: Resident> Send for Soft<'a, R> where R::Data: Send, R::Meta: Send {
}

unsafe impl<'a, R: Resident> Sync for Soft<'a, R> where R::Data: Sync, R::Meta: Sync {
}

impl<'a, R: Resident> Soft<'a, R> {
    /// Constructs a `Soft` lease from a raw pointer returned by `Soft::into_raw`.
    #[inline]
    pub unsafe fn from_raw(data: *mut R::Data) -> Soft<'a, R> {
        Soft {
            data: NonNull::new_unchecked(data),
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

    /// Returns a new mutable lease to the shared resident, traversing any
    /// completed moves, and returning an error if the reisdent is currently
    /// being relocated, or if there are any outstanding mutable or immutable
    /// leases, or if the shared resident has already been dropped, or if there
    /// is atomic operation contention, or if obtaining the lease would cause
    /// a reference count overflow.
    ///
    /// # Safety
    ///
    /// Mutable leases can coexist with hard and soft leases to the same
    /// resident. This can cause a future deadlock if a thread holding a
    /// mutable lease to a resident attempts to convert another hard or soft
    /// lease to the same resident into a mutable or immutable lease.
    pub unsafe fn poll_mut(&self) -> Result<Mut<'a, R>, ArcError> {
        // Get a pointer to the shared resident.
        let data = self.data.as_ptr();
        // Get a pointer to the arc header preceding the resident.
        let header = arc::header::<R>(data);
        // Load the status field; synchronized by subsequent CAS.
        let old_status = (*header).status.load(Relaxed);
        // Check if the shared resident can be mutably referenced.
        if old_status & arc::READ_LOCKED_MASK == 0 {
            // Extract the hard reference count from the status field.
            let old_hard_count = old_status & arc::HARD_COUNT_MASK;
            // Check if the boxed value is still live.
            if old_hard_count == 0 {
                // The shared resident has already been dropped.
                return Err(ArcError::Cleared);
            }
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
            match (*header).status.compare_exchange(old_status, new_status, Acquire, Relaxed) {
                // CAS succeeded; return a new Mut lease.
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
                // Temporarily reify the arc's relocation lease.
                let relocation = mem::transmute::<*mut R::Data, Hard<'a, R>>(relocation);
                // Recurse into the relocated lease.
                let lease = relocation.poll_mut();
                // Discard the borrowed relocation lease.
                mem::forget(relocation);
                // Return the newly acquired lease.
                return lease;
            }
        }
        // Unable to acquire a mutable lease at this time.
        Err(ArcError::Contended)
    }

    /// Returns a new mutable lease to the shared resident, traversing any
    /// completed moves, waiting for any concurrent relocation to complete
    /// and for any outstanding mutable or immutable leases to drop, and
    /// returning an error if the shared resident has already been dropped,
    /// or if obtaining the lease would cause a reference count overflow.
    ///
    /// # Safety
    ///
    /// Deadlocks if the current thread already holds a mutable or immutable
    /// lease to the shared resident. Can cause a future deadlock if a thread
    /// holding a mutable lease to a resident attempts to convert another hard
    /// or soft lease to the same resident into a mutable or immutable lease.
    pub unsafe fn try_to_mut(&self) -> Result<Mut<'a, R>, ArcError> {
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
                // Extract the hard reference count from the status field.
                let old_hard_count = old_status & arc::HARD_COUNT_MASK;
                // Check if the shared resident is still alive.
                if old_hard_count == 0 {
                    // The shared resident has already been dropped.
                    return Err(ArcError::Cleared);
                }
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
                    // CAS succeeded; return a new Mut lease.
                    Ok(_) => return Ok(Mut::from_raw(data)),
                    // CAS failed; update the status field and try again.
                    Err(status) => old_status = status,
                }
            } else {
                // Check if the shared resident has relocated.
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
                // Reload the status field and spin.
                old_status = (*header).status.load(Relaxed);
            }
        }
    }

    /// Returns a new mutable lease to the shared resident, traversing any
    /// completed moves, and waiting for any concurrent relocation to complete
    /// and for any outstanding mutable or immutable leases to drop.
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
    /// Panics if the shared resident has already been dropped, or if obtaining
    /// the lease would cause a reference count overflow.
    pub unsafe fn to_mut(&self) -> Mut<'a, R> {
        self.try_to_mut().unwrap()
    }

    /// Converts this soft lease into a mutable lease to the shared resident,
    /// traversing any completed moves, waiting for any concurrnet relocation
    /// to complete and for any outstanding mutable or immutable leases to drop,
    /// and returning an error if the shared resident has already been dropped,
    /// or if obtaining the lease would cause a reference count overflow.
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
                // Extract the hard reference count from the status field.
                let old_hard_count = old_status & arc::HARD_COUNT_MASK;
                // Check if the shared resident is still alive.
                if old_hard_count == 0 {
                    // The shared resident has already been dropped.
                    return Err(ArcError::Cleared);
                }
                // Increment the hard reference count.
                let new_hard_count = old_hard_count.wrapping_add(1);
                // Check if the incremented hard reference count overflows its bit field.
                if new_hard_count > arc::HARD_COUNT_MAX {
                    return Err(ArcError::HardCountOverflow);
                }
                // Extract the soft reference count from the status field.
                let old_soft_count = (old_status & arc::SOFT_COUNT_MASK) >> arc::SOFT_COUNT_SHIFT;
                // Decrement the soft reference count, checking for underflow.
                let new_soft_count = match old_soft_count.checked_sub(1) {
                    Some(soft_count) => soft_count,
                    None => panic!("soft count underflow"),
                };
                // Clear the hard and soft reference count fields.
                let new_status = old_status & !(arc::HARD_COUNT_MASK | arc::SOFT_COUNT_MASK);
                // Splice the incremented hard and decremented soft reference counts into the status field,
                // and set the mut flag.
                let new_status = new_status | new_hard_count |
                                 new_soft_count << arc::SOFT_COUNT_SHIFT | arc::MUT_FLAG;
                // Atomically update the status field, synchronizing with reference acquires and releases.
                match (*header).status.compare_exchange_weak(old_status, new_status, Acquire, Relaxed) {
                    // CAS succeeded.
                    Ok(_) => {
                        // Discard the original lease, whose soft reference we took.
                        mem::forget(self);
                        // Return a new Mut lease.
                        return Ok(Mut::from_raw(data));
                    },
                    // CAS failed; update the status field and try again.
                    Err(status) => old_status = status,
                }
            } else {
                // Check if the shared resident has relocated.
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
                // Reload the status field and spin.
                old_status = (*header).status.load(Relaxed);
            }
        }
    }

    /// Converts this soft lease into a mutable lease to the shared resident,
    /// traversing any completed moves, waiting for any concurrnet relocation
    /// to complete and for any outstanding mutable or immutable leases to drop,
    /// and returning an error if the shared resident has already been dropped,
    /// or if obtaining the lease would cause a reference count overflow.
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
    /// Panics if the shared resident has already been dropped, or if obtaining
    /// the lease would cause a reference count overflow.
    pub unsafe fn into_mut(self) -> Mut<'a, R> {
        self.try_into_mut().unwrap()
    }

    /// Returns a new immutable lease to the shared resident, traversing any
    /// completed moves, and returning an error if the resident is currently
    /// being relocated, or if there is an outstanding mutable lease, or if
    /// the shared resident has already been dropped, or if there is atomic
    /// operation contention, or if obtaining the lease would cause a reference
    /// count overflow.
    pub fn poll_ref(&self) -> Result<Ref<'a, R>, ArcError> {
        unsafe {
            // Get a pointer to the shared resident.
            let data = self.data.as_ptr();
            // Get a pointer to the arc header preceding the resident.
            let header = arc::header::<R>(data);
            // Load the status field; synchronized by subsequent CAS.
            let old_status = (*header).status.load(Relaxed);
            // Check if the shared resident is not mutably referenced, and is not concurrently relocating.
            if old_status & arc::WRITE_LOCKED_MASK == 0 {
                // Extract the hard reference count from the status field.
                let old_hard_count = old_status & arc::HARD_COUNT_MASK;
                // Check if the shared resident is still alive.
                if old_hard_count == 0 {
                    // The shared resident has already been dropped.
                    return Err(ArcError::Cleared);
                }
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
                // Check if the incremented immutable reference count overflows its bit field.
                if new_ref_count > arc::REF_COUNT_MAX {
                    return Err(ArcError::RefCountOverflow);
                }
                // Clear the hard and immutable reference count bit fields.
                let new_status = old_status & !(arc::HARD_COUNT_MASK | arc::REF_COUNT_MASK);
                // Splice the incremented hard and immutable reference counts into the status field.
                let new_status = new_status | new_hard_count | new_ref_count << arc::REF_COUNT_SHIFT;
                // Atomically update the status field, synchronizing with reference releases.
                match (*header).status.compare_exchange(old_status, new_status, Acquire, Relaxed) {
                    // CAS succeeded; return a new Ref lease.
                    Ok(_) => return Ok(Ref::from_raw(data)),
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
                    // Temporarily reify the arc's relocation lease.
                    let relocation = mem::transmute::<*mut R::Data, Hard<'a, R>>(relocation);
                    // Recurse into the relocated lease.
                    let lease = relocation.poll_ref();
                    // Discard the borrowed relocation lease.
                    mem::forget(relocation);
                    // Return the newly acquired lease.
                    return lease;
                }
            }
            // Unable to acquire an immutable lease at this time.
            Err(ArcError::Contended)
        }
    }

    /// Returns a new immutable lease to the shared resident, traversing any
    /// completed moves, waiting for any concurrent relocation to complete
    /// and for any outstanding mutable lease to drop, and returning an error
    /// if the shared resident has already been dropped, or if obtaining the
    /// lease would cause a reference count overflow.
    pub fn try_to_ref(&self) -> Result<Ref<'a, R>, ArcError> {
        unsafe {
            // Get a pointer to the shared resident.
            let data = self.data.as_ptr();
            // Get a pointer to the arc header preceding the resident.
            let header = arc::header::<R>(data);
            // Load the status field; synchronized by subsequent CAS.
            let mut old_status = (*header).status.load(Relaxed);
            // Spin until an immutable reference is acquired.
            loop {
                // Check if the shared resident is not mutably referenced, and is not concurrently relocating.
                if old_status & arc::WRITE_LOCKED_MASK == 0 {
                    // Extract the hard reference count from the status field.
                    let old_hard_count = old_status & arc::HARD_COUNT_MASK;
                    // Check if the shared resident is still alive.
                    if old_hard_count == 0 {
                        // The shared resident has already been dropped.
                        return Err(ArcError::Cleared);
                    }
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
                    // Check if the incremented immutable reference count overflows its bit field.
                    if new_ref_count > arc::REF_COUNT_MAX {
                        return Err(ArcError::RefCountOverflow);
                    }
                    // Clear the hard and immutable reference count bit fields.
                    let new_status = old_status & !(arc::HARD_COUNT_MASK | arc::REF_COUNT_MASK);
                    // Splice the incremented hard and immutable reference counts into the status field.
                    let new_status = new_status | new_hard_count | new_ref_count << arc::REF_COUNT_SHIFT;
                    // Atomically update the status field, synchronizing with reference releases.
                    match (*header).status.compare_exchange_weak(old_status, new_status, Acquire, Relaxed) {
                        // CAS succeeded; return a new Ref lease.
                        Ok(_) => return Ok(Ref::from_raw(data)),
                        // CAS failed; update the status field and trye again.
                        Err(status) => old_status = status,
                    }
                } else {
                    // Check if the shared resident has relocated.
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
                    // Reload the status field and spin.
                    old_status = (*header).status.load(Relaxed);
                }
            }
        }
    }

    /// Returns a new immutable lease to the shared resident, traversing any
    /// completed moves, and waiting for any concurrent relocation to complete
    /// and for any outstanding mutable lease to drop.
    ///
    /// # Panics
    ///
    /// Panics if the shared resident has already been dropped, or if obtaining
    /// the lease would cause a reference count overflow.
    pub fn to_ref(&self) -> Ref<'a, R> {
        self.try_to_ref().unwrap()
    }

    /// Converts this soft lease into an immutable lease to the shared resident,
    /// traversing any completed moves, waiting for any concurrent relocation to
    /// complete and for any outstanding mutable lease to drop, and returning an
    /// error if the shared resident has already been dropped, or if obtaining
    /// the lease would cause a reference count overflow.
    pub fn try_into_ref(self) -> Result<Ref<'a, R>, ArcError> {
        unsafe {
            // Get a pointer to the shared resident.
            let data = self.data.as_ptr();
            // Get a pointer to the arc header preceding the resident.
            let header = arc::header::<R>(data);
            // Load the status field; synchronized by subsequent CAS.
            let mut old_status = (*header).status.load(Relaxed);
            // Spin until an immutable reference is acquired.
            loop {
                // Check if the shared resident is not mutably referenced, and is not concurrently relocating.
                if old_status & arc::WRITE_LOCKED_MASK == 0 {
                    // Extract the hard reference count from the status field.
                    let old_hard_count = old_status & arc::HARD_COUNT_MASK;
                    // Check if the shared resident is still alive.
                    if old_hard_count == 0 {
                        // The shared resident has already been dropped.
                        return Err(ArcError::Cleared);
                    }
                    // Increment the hard reference count.
                    let new_hard_count = old_hard_count.wrapping_add(1);
                    // Check if the incremented hard reference count overflows its bit field.
                    if new_hard_count > arc::HARD_COUNT_MAX {
                        return Err(ArcError::HardCountOverflow);
                    }
                    // Extract the soft reference count from the status field.
                    let old_soft_count = (old_status & arc::SOFT_COUNT_MASK) >> arc::SOFT_COUNT_SHIFT;
                    // Decrement the soft reference count, checking for underflow.
                    let new_soft_count = match old_soft_count.checked_sub(1) {
                        Some(soft_count) => soft_count,
                        None => panic!("soft count underflow"),
                    };
                    // Extract the immutable reference count from the status field.
                    let old_ref_count = (old_status & arc::REF_COUNT_MASK) >> arc::REF_COUNT_SHIFT;
                    // Increment the immutable reference count.
                    let new_ref_count = old_ref_count.wrapping_add(1);
                    // Check if the incremented immutable reference count overflows its bit field.
                    if new_ref_count > arc::REF_COUNT_MAX {
                        return Err(ArcError::RefCountOverflow);
                    }
                    // Clear the hard, soft, and immutable reference count bit fields.
                    let new_status = old_status & !(arc::HARD_COUNT_MASK | arc::SOFT_COUNT_MASK | arc::REF_COUNT_MASK);
                    // Splice the incremented hard, decremented soft, and incremented immutable reference counts
                    // into the status field.
                    let new_status = new_status | new_hard_count | new_soft_count << arc::SOFT_COUNT_SHIFT |
                                     new_ref_count << arc::REF_COUNT_SHIFT;
                    // Atomically update the status field, synchronizing with reference acquires and releases.
                    match (*header).status.compare_exchange_weak(old_status, new_status, SeqCst, Relaxed) {
                        // CAS succeeded.
                        Ok(_) => {
                            // Discard the original lease, whose soft reference we released.
                            mem::forget(self);
                            // Return a new Ref lease.
                            return Ok(Ref::from_raw(data));
                        },
                        // CAS failed; update the status field and try again.
                        Err(status) => old_status = status,
                    }
                } else {
                    // Check if the shared resident has relocated.
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
                    // Reload the status field and spin.
                    old_status = (*header).status.load(Relaxed);
                }
            }
        }
    }

    /// Converts this soft lease into an immutable lease to the shared resident,
    /// traversing any completed moves, and waiting for any concurrent
    /// relocation to complete and for any outstanding mutable lease to drop.
    ///
    /// # Panics
    ///
    /// Panics if the shared resident has already been dropped, or if obtaining
    /// the lease would cause a reference count overflow.
    pub fn into_ref(self) -> Ref<'a, R> {
        self.try_into_ref().unwrap()
    }

    /// Returns a new hard lease to the shared resident, without traversing
    /// any moves, returning an error if the shared resident has already been
    /// dropped or if obtaining the lease would cause a reference count
    /// overflow.
    pub fn try_to_hard(&self) -> Result<Hard<'a, R>, ArcError> {
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
                // Check if the shared resident is still alive.
                if old_hard_count == 0 {
                    // The shared resident has already been dropped.
                    return Err(ArcError::Cleared);
                }
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

    /// Returns a new hard lease to the shared resident, without traversing
    /// any moves.
    ///
    /// # Panics
    ///
    /// Panics if the shared resident has already been dropped or if obtaining
    /// the lease would cause a reference count overflow.
    pub fn to_hard(&self) -> Hard<'a, R> {
        self.try_to_hard().unwrap()
    }

    /// Converts this soft lease into a hard lease, without traversing any
    /// moves, returning an error if the shared resident has already been
    /// dropped, or if obtaining the lease would cause a reference count
    /// overflow.
    pub fn try_into_hard(self) -> Result<Hard<'a, R>, ArcError> {
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
                // Check if the shared resident is still alive.
                if old_hard_count == 0 {
                    // The shared resident has already been dropped.
                    return Err(ArcError::Cleared);
                }
                // Increment the hard reference count.
                let new_hard_count = old_hard_count.wrapping_add(1);
                // Check if the incremented hard reference count overflows its bit field.
                if new_hard_count > arc::HARD_COUNT_MAX {
                    return Err(ArcError::HardCountOverflow);
                }
                // Extract the soft reference count from the status field.
                let old_soft_count = (old_status & arc::SOFT_COUNT_MASK) >> arc::SOFT_COUNT_SHIFT;
                // Decrement the soft reference count, checking for underflow.
                let new_soft_count = match old_soft_count.checked_sub(1) {
                    Some(soft_count) => soft_count,
                    None => panic!("soft count underflow"),
                };
                // Clear the hard and soft reference count bit fields.
                let new_status = old_status & !(arc::HARD_COUNT_MASK | arc::SOFT_COUNT_MASK);
                // Splice the incremented hard and decremented soft reference counts into the status field.
                let new_status = new_status | new_hard_count | new_soft_count << arc::SOFT_COUNT_SHIFT;
                // Atomically update the status field, synchronizing with reference acquires and releases.
                match (*header).status.compare_exchange_weak(old_status, new_status, SeqCst, Relaxed) {
                    // CAS succeeded.
                    Ok(_) => {
                        // Discard the original lease, whose soft reference we released.
                        mem::forget(self);
                        // Return a new Hard lease.
                        return Ok(Hard::from_raw(data));
                    },
                    // CAS failed; update the status field and try again.
                    Err(status) => old_status = status,
                }
            }
        }
    }

    /// Converts this soft lease into a hard lease, without traversing
    /// any moves.
    ///
    /// # Panics
    ///
    /// Panics if the shared resident has already been dropped, or if obtaining
    /// the lease would cause a reference count overflow.
    pub fn into_hard(self) -> Hard<'a, R> {
        self.try_into_hard().unwrap()
    }

    /// Converts this soft lease into a raw pointer to the shared resident.
    /// Use `Soft::from_raw` to reconstitute the returned pointer back into
    /// a soft lease.
    ///
    /// # Safety
    ///
    /// The shared resident is not pinned to the returned memory address, and
    /// may be concurrently relocation at any time. The resident may not be
    /// hard referenced, and could be dropped at any time. A memory leak will
    /// occur unless the returned pointer is eventually converted back into a
    /// soft lease and dropped.
    #[inline]
    pub unsafe fn into_raw(self) -> *mut R::Data {
        let data = self.data.as_ptr();
        mem::forget(self);
        data
    }

    /// Returns a raw pointer to the shared resident.
    ///
    /// # Safety
    ///
    /// The shared resident may be uninitialized, or mutably aliased,
    /// or may have been relocated, or dropped.
    #[inline]
    pub unsafe fn as_ptr_unchecked(&self) -> *mut R::Data {
        self.data.as_ptr()
    }
}

impl<'a, R: Resident> Holder<'a> for Soft<'a, R> {
    #[inline]
    fn holder(&self) -> &'a dyn Hold<'a> {
        AllocTag::from_ptr(self.header() as *mut u8).holder()
    }
}

impl<'a, R: Resident> Lease for Soft<'a, R> {
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

impl<'a, R: Resident> TryClone for Soft<'a, R> {
    fn try_clone(&self) -> Result<Soft<'a, R>, HoldError> {
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
                    return Err(HoldError::Unsupported("soft count overflow"));
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
}

impl<'a, R: Resident> Clone for Soft<'a, R> {
    fn clone(&self) -> Soft<'a, R> {
        self.try_clone().unwrap()
    }
}

impl<'a, 'b, R: ResidentStow<'b, Hard<'a, R>, Hard<'b, R>>> Stow<'b, Soft<'b, R>> for Soft<'a, R> {
    unsafe fn stow(src: *mut Soft<'a, R>, dst: *mut Soft<'b, R>, hold: &Hold<'b>) -> Result<(), HoldError> {
        // Try to acquire a hard lease to the shared resident.
        match (*src).try_to_hard() {
            // Hard lease acquired.
            Ok(mut hard_src) => {
                let mut hard_dst = mem::uninitialized::<Hard<'b, R>>();
                // Try to stow the hard lease.
                match R::resident_stow(&mut hard_src, &mut hard_dst, hold) {
                    // Stow succeeded.
                    Ok(_) => {
                        // Convert the stowed destination into a soft lease.
                        match hard_dst.try_into_soft() {
                            Ok(soft_dst) => {
                                // Write the stowed soft least to the destination address.
                                ptr::write(dst, soft_dst);
                                Ok(())
                            },
                            Err(error) => Err(HoldError::from(error)),
                        }
                    },
                    // Stow failed.
                    Err(error) => {
                        mem::forget(hard_dst);
                        Err(error)
                    },
                }
            },
            // Failed to acquire a hard reference.
            Err(error) => Err(HoldError::from(error)),
        }
    }

    unsafe fn unstow(src: *mut Soft<'a, R>, dst: *mut Soft<'b, R>) {
        R::resident_unstow(&mut *(src as *mut Hard<'a, R>), &mut *(dst as *mut Hard<'b, R>));
    }
}

unsafe impl<'a, #[may_dangle] R: Resident> Drop for Soft<'a, R> {
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
                        // Check if all references to the shared resident have been released.
                        if new_status & arc::REFERENCED_MASK == 0 {
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
        }
    }
}

impl<'a, R: Resident> Pointer for Soft<'a, R> {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Pointer::fmt(&self.data.as_ptr(), f)
    }
}
