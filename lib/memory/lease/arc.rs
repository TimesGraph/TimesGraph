use core::mem;
use core::ptr;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering::{Acquire, SeqCst};
use crate::block::{self, Block, Layout, LayoutError};
use crate::alloc::{AllocTag, Hold, HoldError};
use crate::resident::{Resident, ResidentFromValue, ResidentFromClone,
                      ResidentFromCloneUnchecked, ResidentFromCopy,
                      ResidentFromCopyUnchecked, ResidentFromEmpty,
                      ResidentWithCapacity};
use crate::lease::{Lease, Mut, Ref, Hard, Soft};

/// Hard reference count bit field mask.
#[cfg(target_pointer_width = "64")]
pub(crate) const HARD_COUNT_MASK: usize = 0x0000000000FFFFFF;
#[cfg(target_pointer_width = "32")]
pub(crate) const HARD_COUNT_MASK: usize = 0x00000FFF;

/// Maximum number of hard references per lease. A `Mut`, `Ref`, or `Hard`
/// lease each holds its own hard reference to its pointed-to arc.
#[cfg(target_pointer_width = "64")]
pub const HARD_COUNT_MAX: usize = 0xFFFFFF;
#[cfg(target_pointer_width = "32")]
pub const HARD_COUNT_MAX: usize = 0xFFF;

/// Soft reference count bit field mask.
#[cfg(target_pointer_width = "64")]
pub(crate) const SOFT_COUNT_MASK: usize = 0x0000FFFFFF000000;
#[cfg(target_pointer_width = "32")]
pub(crate) const SOFT_COUNT_MASK: usize = 0x00FFF000;

/// Number of trailing bits after the soft reference count bit field.
#[cfg(target_pointer_width = "64")]
pub(crate) const SOFT_COUNT_SHIFT: usize = 24;
#[cfg(target_pointer_width = "32")]
pub(crate) const SOFT_COUNT_SHIFT: usize = 12;

/// Maximum number of soft references per lease. A `Soft` lease holds a soft
/// softreference to its pointed-to arc. A `Mut`, `Ref`, or `Hard` lease
/// temporarily acquire a soft reference when it drops, if it holds the last
/// hard reference to its arc, and outstanding soft references remain.
#[cfg(target_pointer_width = "64")]
pub const SOFT_COUNT_MAX: usize = 0xFFFFFF;
#[cfg(target_pointer_width = "32")]
pub const SOFT_COUNT_MAX: usize = 0xFFF;

/// Immutable reference count bit field mask.
#[cfg(target_pointer_width = "64")]
pub(crate) const REF_COUNT_MASK: usize = 0x3FFF000000000000;
#[cfg(target_pointer_width = "32")]
pub(crate) const REF_COUNT_MASK: usize = 0x3F000000;

/// Number of trailing bits after the soft reference count bit field.
#[cfg(target_pointer_width = "64")]
pub(crate) const REF_COUNT_SHIFT: usize = 48;
#[cfg(target_pointer_width = "32")]
pub(crate) const REF_COUNT_SHIFT: usize = 24;

/// Maximum number of immutable references per lease. A `Ref` leases hold an
/// immutable reference to its pointed-to arc.
#[cfg(target_pointer_width = "64")]
pub const REF_COUNT_MAX: usize = 0x3FFF;
#[cfg(target_pointer_width = "32")]
pub const REF_COUNT_MAX: usize = 0x3F;

/// Bit flag indicating the existence of a mutable reference. A `Mut` lease
/// holds the sole mutable reference to its pointed-to arc.
#[cfg(target_pointer_width = "64")]
pub(crate) const MUT_FLAG: usize = 0x4000000000000000;
#[cfg(target_pointer_width = "32")]
pub(crate) const MUT_FLAG: usize = 0x40000000;

/// Bit flag indicating that the resident has relocated to a new arc.
#[cfg(target_pointer_width = "64")]
pub(crate) const RELOCATED_FLAG: usize = 0x8000000000000000;
#[cfg(target_pointer_width = "32")]
pub(crate) const RELOCATED_FLAG: usize = 0x80000000;

/// Bit mask indicating the existence of mutable or immutable leases.
#[cfg(target_pointer_width = "64")]
pub(crate) const ALIASED_MASK: usize = 0x7FFF000000000000;
#[cfg(target_pointer_width = "32")]
pub(crate) const ALIASED_MASK: usize = 0x7F000000;

/// Bit mask indicating the existence of references of any kind.
#[cfg(target_pointer_width = "64")]
pub(crate) const REFERENCED_MASK: usize = 0x7FFFFFFFFFFFFFFF;
#[cfg(target_pointer_width = "32")]
pub(crate) const REFERENCED_MASK: usize = 0x7FFFFFFF;

/// Bit mask indicating the existence of mutable or immutable leases,
/// or that the resident has relocated to a new arc.
#[cfg(target_pointer_width = "64")]
pub(crate) const READ_LOCKED_MASK: usize = 0xFFFF000000000000;
#[cfg(target_pointer_width = "32")]
pub(crate) const READ_LOCKED_MASK: usize = 0xFF000000;

/// Bit mask indicating the existence of a mutable lease, or that the
/// resident has relocated to a new arc.
#[cfg(target_pointer_width = "64")]
pub(crate) const WRITE_LOCKED_MASK: usize = 0xC000000000000000;
#[cfg(target_pointer_width = "32")]
pub(crate) const WRITE_LOCKED_MASK: usize = 0xC0000000;

/// Status field representing a unique hard reference.
#[cfg(target_pointer_width = "64")]
pub(crate) const UNIQUE_STATUS: usize = 0x0000000000000001;
#[cfg(target_pointer_width = "32")]
pub(crate) const UNIQUE_STATUS: usize = 0x00000001;

/// Status field representing a single mutable reference.
pub(crate) const MUT_STATUS_INIT: usize = 1 | MUT_FLAG;

/// Status field representing a single immutable reference.
pub(crate) const REF_STATUS_INIT: usize = 1 | 1 << REF_COUNT_SHIFT;

/// Status field representing a single hard reference.
pub(crate) const HARD_STATUS_INIT: usize = 1;

/// Polymorphic, atomically reference counted lease.
pub enum Arc<'a, R: Resident> {
    /// Mutably dereferenceable, unrelocatable, strong reference.
    Mut(Mut<'a, R>),
    /// Immutably dereferenceable, unrelocatable, strong reference.
    Ref(Ref<'a, R>),
    /// Undereferenceable, relocatable, strong reference.
    Hard(Hard<'a, R>),
    /// Undereferenceable, relocatable, weak reference.
    Soft(Soft<'a, R>),
}

/// Atomic reference counting metadata.
pub struct ArcHeader<M = ()> {
    /// Forwarding address for relocated resident, or zero if the resident
    /// hasn't been relocated. For fat resident pointer types, only the address
    /// component is stored.
    pub(crate) relocation: AtomicUsize,
    /// Reference counts, and relocation flag.
    pub(crate) status: AtomicUsize,
    /// User-provided metadata.
    pub(crate) meta: M,
}

/// Atomic reference counting error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArcError {
    /// Resident already dropped.
    Cleared,
    /// Multiple outstanding references.
    Aliased,
    /// Currently being relocated.
    Relocating,
    /// Lock contention encountered.
    Contended,
    /// Too many hard references.
    HardCountOverflow,
    /// Too many soft references.
    SoftCountOverflow,
    /// Too many immutable references.
    RefCountOverflow,
    /// Improper structure alignment.
    Misaligned,
    /// Structure size overflow.
    Oversized,
    /// Insufficient available memory.
    OutOfMemory,
    /// Unsupported operation; will never succeed.
    Unsupported(&'static str),
}

impl From<LayoutError> for ArcError {
    fn from(error: LayoutError) -> ArcError {
        match error {
            LayoutError::Misaligned => ArcError::Misaligned,
            LayoutError::Oversized => ArcError::Oversized,
        }
    }
}

impl From<HoldError> for ArcError {
    fn from(error: HoldError) -> ArcError {
        match error {
            HoldError::Misaligned => ArcError::Misaligned,
            HoldError::Oversized => ArcError::Oversized,
            HoldError::OutOfMemory => ArcError::OutOfMemory,
            HoldError::Unsupported(reason) => ArcError::Unsupported(reason),
        }
    }
}

impl From<ArcError> for HoldError {
    fn from(error: ArcError) -> HoldError {
        match error {
            ArcError::Cleared => HoldError::Unsupported("cleared"),
            ArcError::Aliased => HoldError::Unsupported("aliased"),
            ArcError::Relocating => HoldError::Unsupported("relocating"),
            ArcError::Contended => HoldError::Unsupported("contended"),
            ArcError::HardCountOverflow => HoldError::Unsupported("hard count overflow"),
            ArcError::SoftCountOverflow => HoldError::Unsupported("soft count overflow"),
            ArcError::RefCountOverflow => HoldError::Unsupported("ref count overflow"),
            ArcError::Misaligned => HoldError::Misaligned,
            ArcError::Oversized => HoldError::Oversized,
            ArcError::OutOfMemory => HoldError::OutOfMemory,
            ArcError::Unsupported(reason) => HoldError::Unsupported(reason),
        }
    }
}

impl<M> ArcHeader<M> {
    /// Returns the number of hard references to the arc.
    #[inline]
    pub fn hard_count(&self) -> usize {
        // Synchronously load the status field.
        let status = self.status.load(SeqCst);
        // Extract and return the hard count bit field.
        status & HARD_COUNT_MASK
    }

    /// Returns the number of soft references to the arc.
    #[inline]
    pub fn soft_count(&self) -> usize {
        // Synchronously load the status field.
        let status = self.status.load(SeqCst);
        // Extract and return the soft count bit field.
        (status & SOFT_COUNT_MASK) >> SOFT_COUNT_SHIFT
    }

    /// Returns the number of immutable references to the arc.
    #[inline]
    pub fn ref_count(&self) -> usize {
        // Synchronously load the status field.
        let status = self.status.load(SeqCst);
        // Extract and return the ref count bit field.
        (status & REF_COUNT_MASK) >> REF_COUNT_SHIFT
    }

    /// Returns `true` if the arc is mutably referenced.
    #[inline]
    pub fn is_mut(&self) -> bool {
        // Synchronously load the status field.
        let status = self.status.load(SeqCst);
        // Return whether or not the mut flag is set.
        status & MUT_FLAG != 0
    }

    /// Returns `true` if the resident has relocated to a new arc.
    #[inline]
    pub fn is_relocated(&self) -> bool {
        // Synchronously load the status field.
        let status = self.status.load(SeqCst);
        // Return whether or not the relocated bit is set.
        status & RELOCATED_FLAG != 0
    }

    /// Returns `true` if the arc is immutably or mutably referenced.
    #[inline]
    pub fn is_aliased(&self) -> bool {
        // Synchronously load the status field.
        let status = self.status.load(SeqCst);
        // Return whether or not the ref count bit field is non-zero, or the mut flag is set.
        status & ALIASED_MASK != 0
    }

    /// Drops the arc header. Releases the relocation lease, if relocated;
    /// drops the associated metadata, if not relocated.
    #[inline]
    pub(crate) fn drop<R: Resident>(&mut self, data: *mut R::Data) {
        unsafe {
            // Get a fat pointer to the relocated resident, synchronizing with relocation completion.
            let relocation = block::set_address(data, self.relocation.load(Acquire));
            // Check if the resident has relocated.
            if !relocation.is_null() {
                // Reify and drop the arc's relocation lease.
                mem::drop(mem::transmute::<*mut R::Data, Hard<R>>(relocation));
            } else {
                // Drop the unrelocated metadata.
                ptr::drop_in_place(&mut self.meta);
            }
        }
    }
}

/// Allocate a new arc structure in `hold` for a resident with the given `data`
/// and `meta` data, initialized with the given lease `status`.
#[inline]
pub(crate) unsafe fn alloc_new<'a, R, L, T, M>(hold: &Hold<'a>, data: &T, meta: &M, status: usize)
    -> Result<*mut R::Data, HoldError>
    where R: ResidentFromValue<L, T, M>,
          L: Lease,
{
    // Compute the layout of the arc structure, capturing the offset of its resident field.
    let (layout, offset) = Layout::for_type::<ArcHeader<R::Meta>>()
        .extended(R::new_resident_layout(data, meta))?;
    // Allocate a block of memory to hold the arc structure, bailing on failure.
    let block = hold.alloc(layout)?;
    // Get a pointer to the header field of the new arc.
    let header = block.as_ptr() as *mut ArcHeader<R::Meta>;
    // Initialize the relocation address to zero.
    ptr::write(&mut (*header).relocation, AtomicUsize::new(0));
    // Initialize the lease status field.
    ptr::write(&mut (*header).status, AtomicUsize::new(status));
    // Get a raw pointer to the resident field of the new arc.
    let resident = (header as *mut u8).wrapping_add(offset);
    // Return a fat pointer to the resident field.
    Ok(R::new_resident_ptr(resident, data, meta))
}

/// Allocate a new arc structure in `hold` for a resident with a clone of the
/// given `data` and `meta` data, initialized with the given lease `status`.
#[inline]
pub(crate) unsafe fn alloc_clone<'a, R, L, T, M>(hold: &Hold<'a>, data: &T, meta: &M, status: usize)
    -> Result<*mut R::Data, HoldError>
    where R: ResidentFromClone<L, T, M>,
          L: Lease,
          T: ?Sized,
{
    // Compute the layout of the arc structure, capturing the offset of its resident field.
    let (layout, offset) = Layout::for_type::<ArcHeader<R::Meta>>()
        .extended(R::new_resident_layout(data, meta))?;
    // Allocate a block of memory to hold the arc structure, bailing on failure.
    let block = hold.alloc(layout)?;
    // Get a pointer to the header field of the new arc.
    let header = block.as_ptr() as *mut ArcHeader<R::Meta>;
    // Initialize the relocation address to zero.
    ptr::write(&mut (*header).relocation, AtomicUsize::new(0));
    // Initialize the lease status field.
    ptr::write(&mut (*header).status, AtomicUsize::new(status));
    // Get a raw pointer to the resident field of the new arc.
    let resident = (header as *mut u8).wrapping_add(offset);
    // Return a fat pointer to the resident field.
    Ok(R::new_resident_ptr(resident, data, meta))
}

/// Allocate a new arc structure in `hold` for a resident with an unchecked
/// clone of the given `data` and `meta` data, initialized with the given lease
/// `status`.
#[inline]
pub(crate) unsafe fn alloc_clone_unchecked<'a, R, L, T, M>(hold: &Hold<'a>, data: &T, meta: &M, status: usize)
    -> Result<*mut R::Data, HoldError>
    where R: ResidentFromCloneUnchecked<L, T, M>,
          L: Lease,
          T: ?Sized,
{
    // Compute the layout of the arc structure, capturing the offset of its resident field.
    let (layout, offset) = Layout::for_type::<ArcHeader<R::Meta>>()
        .extended(R::new_resident_layout(data, meta))?;
    // Allocate a block of memory to hold the arc structure, bailing on failure.
    let block = hold.alloc(layout)?;
    // Get a pointer to the header field of the new arc.
    let header = block.as_ptr() as *mut ArcHeader<R::Meta>;
    // Initialize the relocation address to zero.
    ptr::write(&mut (*header).relocation, AtomicUsize::new(0));
    // Initialize the lease status field.
    ptr::write(&mut (*header).status, AtomicUsize::new(status));
    // Get a raw pointer to the resident field of the new arc.
    let resident = (header as *mut u8).wrapping_add(offset);
    // Return a fat pointer to the resident field.
    Ok(R::new_resident_ptr(resident, data, meta))
}

/// Allocate a new arc structure in `hold` for a resident with a copy of the
/// given `data` and `meta` data, initialized with the given lease `status`.
#[inline]
pub(crate) unsafe fn alloc_copy<'a, R, L, T, M>(hold: &Hold<'a>, data: &T, meta: &M, status: usize)
    -> Result<*mut R::Data, HoldError>
    where R: ResidentFromCopy<L, T, M>,
          L: Lease,
          T: ?Sized,
{
    // Compute the layout of the arc structure, capturing the offset of its resident field.
    let (layout, offset) = Layout::for_type::<ArcHeader<R::Meta>>()
        .extended(R::new_resident_layout(data, meta))?;
    // Allocate a block of memory to hold the arc structure, bailing on failure.
    let block = hold.alloc(layout)?;
    // Get a pointer to the header field of the new arc.
    let header = block.as_ptr() as *mut ArcHeader<R::Meta>;
    // Initialize the relocation address to zero.
    ptr::write(&mut (*header).relocation, AtomicUsize::new(0));
    // Initialize the lease status field.
    ptr::write(&mut (*header).status, AtomicUsize::new(status));
    // Get a raw pointer to the resident field of the new arc.
    let resident = (header as *mut u8).wrapping_add(offset);
    // Return a fat pointer to the resident field.
    Ok(R::new_resident_ptr(resident, data, meta))
}

/// Allocate a new arc structure in `hold` for a resident with an unchecked
/// copy of the given `data` and `meta` data, initialized with the given lease
/// `status`.
#[inline]
pub(crate) unsafe fn alloc_copy_unchecked<'a, R, L, T, M>(hold: &Hold<'a>, data: &T, meta: &M, status: usize)
    -> Result<*mut R::Data, HoldError>
    where R: ResidentFromCopyUnchecked<L, T, M>,
          L: Lease,
          T: ?Sized,
{
    // Compute the layout of the arc structure, capturing the offset of its resident field.
    let (layout, offset) = Layout::for_type::<ArcHeader<R::Meta>>()
        .extended(R::new_resident_layout(data, meta))?;
    // Allocate a block of memory to hold the arc structure, bailing on failure.
    let block = hold.alloc(layout)?;
    // Get a pointer to the header field of the new arc.
    let header = block.as_ptr() as *mut ArcHeader<R::Meta>;
    // Initialize the relocation address to zero.
    ptr::write(&mut (*header).relocation, AtomicUsize::new(0));
    // Initialize the lease status field.
    ptr::write(&mut (*header).status, AtomicUsize::new(status));
    // Get a raw pointer to the resident field of the new arc.
    let resident = (header as *mut u8).wrapping_add(offset);
    // Return a fat pointer to the resident field.
    Ok(R::new_resident_ptr(resident, data, meta))
}

/// Allocate a new arc structure in `hold` for an empty resident with the given
/// `meta` data, initialized with the given lease `status`.
#[inline]
pub(crate) unsafe fn alloc_empty<'a, R, L, M>(hold: &Hold<'a>, meta: &M, status: usize)
    -> Result<*mut R::Data, HoldError>
    where R: ResidentFromEmpty<L, M>,
          L: Lease,
{
    // Compute the layout of the arc structure, capturing the offset of its resident field.
    let (layout, offset) = Layout::for_type::<ArcHeader<R::Meta>>()
        .extended(R::new_resident_layout(meta))?;
    // Allocate a block of memory to hold the arc structure, bailing on failure.
    let block = hold.alloc(layout)?;
    // Get a pointer to the header field of the new arc.
    let header = block.as_ptr() as *mut ArcHeader<R::Meta>;
    // Initialize the relocation address to zero.
    ptr::write(&mut (*header).relocation, AtomicUsize::new(0));
    // Initialize the lease status field.
    ptr::write(&mut (*header).status, AtomicUsize::new(status));
    // Get a raw pointer to the resident field of the new arc.
    let resident = (header as *mut u8).wrapping_add(offset);
    // Return a fat pointer to the resident field.
    Ok(R::new_resident_ptr(resident, meta))
}

/// Allocate a new arc structure in `hold` for a resident with `cap` slots,
/// with the given `meta` data, initialized with the given lease `status`.
#[inline]
pub(crate) unsafe fn alloc_cap<'a, R, L, M>(hold: &Hold<'a>, cap: usize, meta: &M, status: usize)
    -> Result<*mut R::Data, HoldError>
    where R: ResidentWithCapacity<L, M>,
          L: Lease,
{
    // Compute the layout of the arc structure, capturing the offset of its resident field.
    let (layout, offset) = Layout::for_type::<ArcHeader<R::Meta>>()
        .extended(R::new_resident_layout(cap, meta)?)?;
    // Allocate a block of memory to hold the arc structure, bailing on failure.
    let block = hold.alloc(layout)?;
    // Get a pointer to the header field of the new arc.
    let header = block.as_ptr() as *mut ArcHeader<R::Meta>;
    // Initialize the relocation address to zero.
    ptr::write(&mut (*header).relocation, AtomicUsize::new(0));
    // Initialize the lease status field.
    ptr::write(&mut (*header).status, AtomicUsize::new(status));
    // Get a raw pointer to the resident field of the new arc.
    let resident = (header as *mut u8).wrapping_add(offset);
    // Return a fat pointer to the resident field.
    Ok(R::new_resident_ptr(resident, cap, meta))
}

/// Reallocates the arc containing the `old_data` resident to fit the
/// `new_layout`; uses the same `Hold` that allocated the old arc.
pub(crate) unsafe fn realloc<R>(old_data: *mut R::Data, new_layout: Layout) -> Result<*mut R::Data, HoldError>
    where R: Resident
{
    // Get the alignment of the resident.
    let align = mem::align_of_val(&*old_data);
    // Compute the layout of the arc header.
    let header_layout = Layout::for_type::<ArcHeader<R::Meta>>();
    // Get the offset of the resident in the arc structure by rounding up
    // the size of the arc header to the alignment of the resident.
    let offset = header_layout.size().wrapping_add(align).wrapping_sub(1) & !align.wrapping_sub(1);
    // Get a pointer to the current metadata by subtracting the resident's
    // offset in the arc structure.
    let old_meta = (old_data as *mut u8).wrapping_sub(offset) as *mut R::Meta;
    // Compute the total size of the arc structure.
    let size = offset.wrapping_add(R::resident_size(old_data, old_meta));
    // Extend the arc header to include the new layout of the resident.
    let new_layout = header_layout.extended(new_layout)?.0;
    // Get the currently leased memory block.
    let old_block = Block::from_raw_parts(old_meta as *mut u8, size);
    // Get a pointer to the hold that allocated the current memory block.
    let hold = AllocTag::from_ptr(old_meta as *mut u8).holder();
    // Reallocate the leased memory block.
    match hold.realloc(old_block, new_layout) {
        // Reallocation succeeded.
        Ok(new_block) => {
            // Get a pointer to the reallocated arc header.
            let new_meta = new_block.as_ptr() as *mut ArcHeader<R::Meta>;
            // Get a fat pointer to the reallocated resident.
            let new_data = block::set_address(old_data, (new_meta as usize).wrapping_add(offset));
            // Return a pointer to the new resident.
            Ok(new_data)
       },
       // Reallocation failed.
       Err(error) => Err(error),
    }
}

/// Resizes in place the arc containing the `old_data` resident to fit the `new_layout`.
pub(crate) unsafe fn resize<R>(old_data: *mut R::Data, new_layout: Layout) -> Result<*mut R::Data, HoldError>
    where R: Resident
{
    // Get the alignment of the resident.
    let align = mem::align_of_val(&*old_data);
    // Compute the layout of the arc header.
    let header_layout = Layout::for_type::<ArcHeader<R::Meta>>();
    // Get the offset of the resident in the arc structure by rounding up
    // the size of the arc header to the alignment of the resident.
    let offset = header_layout.size().wrapping_add(align).wrapping_sub(1) & !align.wrapping_sub(1);
    // Get a pointer to the current metadata by subtracting the resident's
    // offset in the arc structure.
    let old_meta = (old_data as *mut u8).wrapping_sub(offset) as *mut R::Meta;
    // Compute the total size of the arc structure.
    let size = offset.wrapping_add(R::resident_size(old_data, old_meta));
    // Extend the arc header to include the new layout of the resident.
    let new_layout = header_layout.extended(new_layout)?.0;
    // Get the currently leased memory block.
    let old_block = Block::from_raw_parts(old_meta as *mut u8, size);
    // Get a pointer to the hold that allocated the current memory block.
    let hold = AllocTag::from_ptr(old_meta as *mut u8).holder();
    // Reallocate the leased memory block.
    match hold.resize(old_block, new_layout) {
        // Reallocation succeeded.
        Ok(new_block) => {
            // Get a pointer to the reallocated arc header.
            let new_meta = new_block.as_ptr() as *mut ArcHeader<R::Meta>;
            // Get a fat pointer to the reallocated resident.
            let new_data = block::set_address(old_data, (new_meta as usize).wrapping_add(offset));
            // Return a pointer to the new resident.
            Ok(new_data)
       },
       // Reallocation failed.
       Err(error) => Err(error),
    }
}

/// Returns a pointer to the `ArcHeader` preceding the resident `data` pointer.
#[inline]
pub(crate) fn header<R: Resident>(data: *mut R::Data) -> *mut ArcHeader<R::Meta> {
    // Get the alignment of the resident.
    let align = mem::align_of_val(unsafe { &*data });
    // Get the offset of the resident in the arc structure by rounding up
    // the size of the arc header to the alignment of the resident.
    let offset = mem::size_of::<ArcHeader<R::Meta>>()
        .wrapping_add(align).wrapping_sub(1) & !align.wrapping_sub(1);
    // Return a pointer to the arc header by subtracting the resident's
    // offset in the arc structure.
    (data as *mut u8).wrapping_sub(offset) as *mut ArcHeader<R::Meta>
}
