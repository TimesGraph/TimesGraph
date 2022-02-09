use core::cmp;
use core::intrinsics::assume;
use core::marker::{PhantomData, PhantomPinned};
use core::mem;
use core::ptr;
use core::sync::atomic::{self, AtomicPtr, AtomicUsize};
use core::sync::atomic::Ordering::{Acquire, Relaxed, Release, SeqCst};
use core::usize;
use crate::block::{Block, Layout};
use crate::alloc::{Heap, HeapError};

/// Pointer bit flag indicating a temporarily frozen list node.
const FREEZE_FLAG: usize = 0x1;
/// Pointer bit flag indicating a logically deleted list node.
const REMOVE_FLAG: usize = 0x2;
/// Bit mask that clears the freeze and remove flags from a pointer.
const ADDR_MASK: usize = !0x3;

/// Bit flag indicating a list node that is logically merged with its successor.
const MERGE_FLAG: usize = 0x4;
/// Bit flag indicating a logically split list node; the high order bits of
/// the status field hold the extent aligned split offset.
const SPLIT_FLAG: usize = 0x8;
/// Bit mask that clears the freeze, remove, merge, and split flags from a status value.
const SIZE_MASK: usize = !0xF;

/// Power-of-two base address alignment of an extent.
const EXTENT_ALIGN: usize = mem::align_of::<ExtentNode>();
/// Bit mask that rounds an address within a free extent header down to the
/// base address of the extent.
const EXTENT_ADDR_MASK: usize = !(EXTENT_ALIGN - 1);

/// Maximum number of levels in the free extent skip list.
const MAX_LEVEL: usize = 32;

/// Reference counted smart pointer to an `ExtentList`.
pub struct AddrSpace<'a> {
    ptr: *mut ExtentList<'a>,
    size: usize,
}

/// Lock-free allocator of page-aligned memory extents from some address range.
#[repr(C, align(4096))]
pub struct ExtentList<'a> {
    /// Size in bytes of this extent; always zero for the head extent.
    zero: usize,
    /// Insertion status of this slip list.
    status: AtomicUsize,
    /// Number of references to this extent.
    refcount: AtomicUsize,
    /// Skip list of free extents, ordered by ascending address.
    addr_list: AddrList,
    /// Skip list of free extents, ordered by ascending size,
    /// then ascending address.
    size_list: SizeList,
    /// Number of bytes in the address space, including the head extent.
    size: AtomicUsize,
    /// Number of live allocations in the address space.
    live: AtomicUsize,
    /// Number of currently allocated bytes in the address space.
    used: AtomicUsize,
    /// Lifetime of the address space in which this extent list resides.
    lifetime: PhantomData<&'a ()>,
    /// Pin to the base address of the head extent.
    pinned: PhantomPinned,
}

/// Header embedded within a free memory extent, with skip list nodes for the
/// address-ordered free extent list, and the size-ordered free extent list.
/// Transmutable from an `ExtentList`.
#[repr(C, align(4096))]
struct ExtentNode {
    /// Size in bytes of the extent, including this header.
    size: usize,
    /// Insertion status of this slip list node.
    status: AtomicUsize,
    /// Number of references to this extent.
    refcount: AtomicUsize,
    /// Skip list node linking this extent to subsequent free extents,
    /// ordered by ascending address.
    addr_node: AddrNode,
    /// Skip list node linking this extent to subsequent free extents,
    /// ordered by ascending size, then ascending address.
    size_node: SizeNode,
    /// Pin to the base address of the free extent.
    pinned: PhantomPinned,
}

/// Base address of a free extent, used to order the address skip list.
type AddrKey = usize;

/// Lock-free sorted skip list of free extents, ordered by ascending address.
#[repr(C)]
struct AddrList {
    /// Pseudo skip list node whose address is less than all valid extent addresses.
    head: AddrNode,
}

/// Lock-free sorted skip list node for a skip list of free extents, ordered by
/// ascending address. The base address of the extent in which a node resides
/// is obtained by rounding down the address of the node to the extent alignment.
#[repr(C)]
struct AddrNode {
    /// State of the pseudo-random number generator used to select the heights
    /// of inserted skip list nodes; used only by `AddrList` head nodes.
    seed: AtomicUsize,
    /// Tower of forward-pointing list links, with `height` valid levels.
    levels: [AddrLink; MAX_LEVEL],
}

/// Reference counted smart pointer to an `AddrNode`; uses the `refcount` field
/// of the extent in which the pointed-to node resides.
struct AddrNodeRef {
    ptr: *mut AddrNode,
}

/// Lock-free sorted skip list link for given level of a skip list of free
/// extents, ordered by ascending address. The root level of the node in which
/// a link resides is obtained by decrementing the link pointer by its level in
/// the skip list. The base address of the extent in which a link resides is
/// obtained by rounding down the address of the link to the extent alignment.
#[repr(C)]
struct AddrLink {
    /// Pointer to the next link in the list, possibly tagged with a freeze or
    /// remove flag--but never both. Zero indicates the end of the list.
    succ: AtomicUsize,
    /// Back pointer to the predecessor link in the level list, whose successor
    /// points to this link, with its freeze bit set so that it cannot be marked
    /// while `back` points to it. Non-zero only during removal.
    back: AtomicPtr<AddrLink>,
    /// Pin to level in `AddrNode` tower.
    pinned: PhantomPinned,
}

/// Reference counted smart pointer to an `AddrLink`; uses the `refcount` field
/// of the extent in which the pointed-to link resides.
struct AddrLinkRef {
    ptr: *mut AddrLink,
}

/// Size and base address of a free extent, used to order the size skip list.
type SizeKey = (usize, usize);

/// Lock-free sorted skip list of free extents, ordered by ascending size, then
/// ascending address.
#[repr(C)]
struct SizeList {
    /// Pseudo skip list node whose size is less than all valid extent sizes.
    head: SizeNode,
}

/// Lock-free sorted skip list node for a skip list of free extents, ordered by
/// ascending size, then ascending address. The base address of the extent in
/// which a node resides is obtained by rounding down the address of the node
/// to the extent alignment.
#[repr(C)]
struct SizeNode {
    /// State of the pseudo-random number generator used to select the heights
    /// of inserted skip list nodes; used only by `SizeList` head nodes.
    seed: AtomicUsize,
    /// Tower of forward-pointing list links, with `height` valid levels.
    levels: [SizeLink; MAX_LEVEL],
}

/// Reference counted smart pointer to a `SizeNode`; uses the `refcount` field
/// of the extent in which the pointed-to node resides.
struct SizeNodeRef {
    ptr: *mut SizeNode,
}

/// Lock-free sorted skip list link for given level of a skip list of free
/// extents, ordered by ascending size, then ascending address. The root level
/// of the node in which a link resides is obtained by decrementing the link
/// pointer by its level in the skip list. The base address of the extent in
/// which a link resides is obtained by rounding down the address of the link
/// to the extent alignment.
#[repr(C)]
struct SizeLink {
    /// Pointer to the next link in the list, possibly tagged with a freeze or
    /// remove flag--but never both. Zero indicates the end of the list.
    succ: AtomicUsize,
    /// Back pointer to the predecessor link in the level list, whose successor
    /// points to this link, with its freeze bit set so that it cannot be marked
    /// while `back` points to it. Non-zero only during removal.
    back: AtomicPtr<SizeLink>,
    /// Pin to level in `AddrNode` tower.
    pinned: PhantomPinned,
}

/// Reference counted smart pointer to a `SizeLink`; uses the `refcount` field
/// of the extent in which the pointed-to link resides.
struct SizeLinkRef {
    ptr: *mut SizeLink,
}

#[macro_export]
macro_rules! addr_space {
    // pub heap $name = [$size];
    ($(#[$attr:meta])* pub heap $name:ident = [$size:expr];) => (
        #[repr(align(4096))]
        struct __Heap([u8; $size]);
        static mut __HEAP: __Heap = __Heap([0; $size]);
        $(#[$attr])* pub static $name: $crate::alloc::AddrSpace<'static> =
            unsafe { $crate::alloc::AddrSpace::from_raw(&__HEAP as *const __Heap as *mut u8, $size) };
    );
}

unsafe impl<'a> Send for AddrSpace<'a> {
}

unsafe impl<'a> Sync for AddrSpace<'a> {
}

impl<'a> AddrSpace<'a> {
    #[inline]
    pub const unsafe fn from_raw(ptr: *mut u8, size: usize) -> AddrSpace<'a> {
        AddrSpace {
            ptr: ptr as *mut ExtentList,
            size: size,
        }
    }

    #[inline]
    unsafe fn extent(&self) -> *mut ExtentNode {
        self.ptr as *mut ExtentNode
    }

    /// Returns the total number of bytes in this address space.
    #[inline]
    pub fn size(&self) -> usize {
        self.size
    }

    /// Returns the number of live allocations from this address space.
    #[inline]
    pub fn live(&self) -> usize {
        unsafe { (*self.ptr).live() }
    }

    /// Returns the number of bytes currently allocated from this address space.
    #[inline]
    pub fn used(&self) -> usize {
        unsafe { (*self.ptr).used() }
    }
}

impl<'a> Heap<'a> for AddrSpace<'a> {
    unsafe fn alloc(&self, layout: Layout) -> Result<Block<'a>, HeapError> {
        (*self.ptr).grow(self.size);
        (*self.ptr).alloc(layout)
    }

    unsafe fn dealloc(&self, block: Block<'a>) -> usize {
        (*self.ptr).grow(self.size);
        (*self.ptr).dealloc(block)
    }
}

impl<'a> Clone for AddrSpace<'a> {
    fn clone(&self) -> AddrSpace<'a> {
        unsafe {
            self.extent().retain();
            AddrSpace::from_raw(self.ptr as *mut u8, self.size)
        }
    }
}

impl<'a> Drop for AddrSpace<'a> {
    fn drop(&mut self) {
        unsafe {
            self.extent().release();
        }
    }
}

impl<'a> ExtentList<'a> {
    /// Returns the total number of bytes in this address space.
    #[inline]
    pub fn size(&self) -> usize {
        self.size.load(Relaxed)
    }

    /// Returns the number of live allocations from this address space.
    #[inline]
    pub fn live(&self) -> usize {
        self.live.load(Relaxed)
    }

    /// Returns the number of bytes currently allocated from this address space.
    #[inline]
    pub fn used(&self) -> usize {
        self.used.load(Relaxed)
    }

    /// Extends this address space to `new_size` bytes in length by inserting a
    /// free extent for the delta between the current size and the new size.
    pub unsafe fn grow(&self, new_size: usize) {
        // Get the current size of the address space; synchronized by subsequent CAS.
        let mut old_size = self.size.load(Relaxed);
        // The address space must be large enough to hold the head extent.
        let mut min_size;
        // Loop until the address space has been resized.
        loop {
            // Check if the address space is already the desired size.
            if old_size == new_size {
                return;
            }
            // Exclude the head extent from the growable range.
            min_size = cmp::max(EXTENT_ALIGN, old_size);
            // Ensure that the new address space size is extent aligned.
            assert!(new_size % EXTENT_ALIGN == 0);
            // Make sure there's room for a complete extent.
            assert!(min_size + EXTENT_ALIGN <= new_size);
            // Try to grow the address space, synchronizing with concurrent
            // grow operations.
            match self.size.compare_exchange(old_size, new_size, SeqCst, Relaxed) {
                // CAS succeeded; incorporate the new extent.
                Ok(_) => break,
                // CAS failed; update the current address space size and try again.
                Err(size) => old_size = size,
            }
        }
        // Get a pointer to the new extent.
        let extent = (self as *const ExtentList<'a> as usize).wrapping_add(min_size) as *mut ExtentNode;
        // Compute the size of the new extent as the delta between the old and
        // new address space sizes.
        let size = new_size.wrapping_sub(min_size);
        // Insert the extent into the free extent skip lists.
        self.insert(extent, size);
    }

    /// Inserts an extent into the free extent skip lists, without updating
    /// allocation accounting information.
    #[inline]
    unsafe fn insert(&self, extent: *mut ExtentNode, size: usize) {
        // Initialize the size of the free extent.
        ptr::write(&mut (*extent).size, size);
        // Initialize two references to the free extent, one for the address
        // skip list, and one for the size skip list.
        ptr::write(&mut (*extent).refcount, AtomicUsize::new(2));
        // Get a reference to the free extent's address skip list node.
        let addr_node = AddrNodeRef::from_raw(&mut (*extent).addr_node);
        // Get a reference to the free extent's size skip list node.
        let size_node = SizeNodeRef::from_raw(&mut (*extent).size_node);
        // Insert the free extent into the size-ordered skip list.
        self.size_list.insert(size_node);
        // Insert the free extent into the address-ordered skip list;
        // linearization point for extent insertion.
        self.addr_list.insert(addr_node);
    }
}

impl<'a> Heap<'a> for ExtentList<'a> {
    unsafe fn alloc(&self, layout: Layout) -> Result<Block<'a>, HeapError> {
        // Get the requested allocation size.
        let size = layout.size();
        // Check if this is a zero-sized allocation.
        if size == 0 {
            // Return the zero-sized block.
            return Ok(Block::empty());
        }
        // Align the allocation size to the extent alignment.
        let size = size.wrapping_add(EXTENT_ALIGN).wrapping_sub(1) & !EXTENT_ALIGN.wrapping_sub(1);
        // Compute the greaters lower bound of the desired allocation size.
        let size_key = (size.wrapping_sub(1), usize::MAX);
        // Loop until a suitable extent is allocated.
        loop {
            // Remove the first suitable extent from the size skip list;
            // linearization point for allocation attempt.
            let size_link = self.size_list.take_next(size_key);
            // Check if no suitable extent was found.
            if size_link.is_nil() {
                // Out of memory.
                return Err(HeapError::OutOfMemory);
            }
            // Get the address of the removed extent.
            let addr = size_link.addr();
            // Drop the reference to the removed link in the size list to
            // reduce latency with concurrent allocations.
            mem::drop(size_link);
            // Remove the extent from the address skip list; linearization
            // point for allocation completion.
            let addr_link = self.addr_list.remove(addr, addr.wrapping_sub(1));
            // Check if removal from the address skip list failed.
            if addr_link.is_nil() {
                // Extent concurrently merged with its predecessor; try again.
                continue;
            }
            // Drop the reference to the address skip link, converting it to an
            // uncounted pointer to the removed extent.
            let extent = addr_link.into_extent();
            // Wait for all remaining references to the extent drop.
            extent.await_release();

            // Compute the difference between the allocated extent size and the
            // requested allocation size.
            let excess_size = (*extent).size.wrapping_sub(size);
            // Check if the allocated extent can be split in two, and an extent
            // containing the excess bytes re-inserted into the free lists.
            if excess_size != 0 {
                // Ensure that the excess extent is properly aligned.
                assert!(excess_size % EXTENT_ALIGN == 0);
                // Get a pointer to the excess extent.
                let excess_extent = (extent as usize).wrapping_add(size) as *mut ExtentNode;
                // Insert the excess extent back into the free skip lists.
                self.insert(excess_extent, excess_size);
            }

            // Increment the number of live allocations in the address space.
            self.live.fetch_add(1, Relaxed);
            // Increment the number of allocated bytes in the address space.
            self.used.fetch_add(size, Relaxed);
            // Return the raw memory block of the newly allocated extent.
            return Ok(Block::from_raw_parts(extent as *mut u8, size));
        }
    }

    unsafe fn dealloc(&self, block: Block<'a>) -> usize {
        // Get the size of the memory block to deallocate.
        let size = block.size();
        // Check if this is a zero-sized deallocation.
        if size == 0 {
            // Nothing to deallocate.
            return 0;
        }
        // Transmute the memory block into a free extent.
        let extent = block.into_raw() as *mut ExtentNode;
        // Insert the extent into the free extent skip lists.
        self.insert(extent, size);
        // Return the size of the freed extent.
        size
    }
}

impl ExtentNode {
    /// Returns `true` if this is the `nil` extent.
    #[inline]
    fn is_nil(self: *mut ExtentNode) -> bool {
        self.is_null()
    }

    /// Increments the reference count of this extent.
    #[inline]
    unsafe fn retain(self: *mut ExtentNode) {
        // Check if this is the nil extent.
        if self.is_nil() {
            // The nil extent has no reference count.
            return;
        }
        // Increment the reference count of this extent, synchronizing with
        // reference releases.
        let old_refcount = (*self).refcount.fetch_add(1, Acquire);
        if old_refcount == 0 {
            panic!();
        }
        // Check for reference count overflow.
        if old_refcount == usize::MAX {
            panic!("refcount overflow");
        }
    }

    /// Decremements the reference count of this extent, returning `true` if
    /// this causes the reference count to drop to zero.
    #[inline]
    unsafe fn release(self: *mut ExtentNode) -> bool {
        // Check if this is the nil extent.
        if self.is_nil() {
            // The nil extent has no reference count.
            return false;
        }
        // Decrement the reference count of this extent, synchronizing with
        // reference count acquires.
        let old_refcount = (*self).refcount.fetch_sub(1, Release);
        if old_refcount == 0 {
            panic!();
        }
        // Return true if the reference count dropped to zero.
        old_refcount == 1
    }

    /// Busy waits until all references to this extent have been released.
    #[inline]
    unsafe fn await_release(self: *mut ExtentNode) {
        // Loop until all references to this extent have been released.
        loop {
            // Load this extent's reference count, synchronizing with reference
            // count releases.
            let refcount = (*self).refcount.load(Acquire);
            // Check if the reference count has dropped to zero.
            if refcount == 0 {
                // The extent is no longer aliased.
                break;
            }
            // Busy wait before trying again.
            atomic::spin_loop_hint();
        }
    }

    ///// Merges this extent with its successor.
    //#[inline]
    //unsafe fn merge(self: *mut ExtentNode) {
    //    // Load the status field of the extent in which this link resides;
    //    // synchronized by subsequent CAS.
    //    let mut old_status = (*self).status.load(Relaxed);
    //    // Loop until the merge flag is set.
    //    loop {
    //        // Check if the extent is flagged for removal, merging, or splitting.
    //        if old_status & REMOVE_FLAG != 0 {
    //            // Can't merge logically removed extent.
    //            return;
    //        } else if old_status & MERGE_FLAG != 0 {
    //            // Help merge the extent with its successor.
    //            self.help_merge();
    //        } else if old_status & SPLIT_FLAG != 0 {
    //            // Help split the extent at the size boundary encoded in the
    //            // high order bits of the status field.
    //            self.help_split(old_status & SIZE_MASK);
    //        } else {
    //            // Set the merge flag on the status field.
    //            let new_status = old_status | MERGE_FLAG;
    //            // Try to update the status field, synchronizing with other
    //            // list mutations; linearization point for extent merging.
    //            match (*self).status.compare_exchange(old_status, new_status, SeqCst, Relaxed) {
    //                // CAS succeeded.
    //                Ok(_) => break,
    //                // CAS failed; try again with the latest status.
    //                Err(status) => old_status = status,
    //            }
    //        }
    //    }
    //}

    ///// Splits this extent at the `size` byte boundary.
    //#[inline]
    //unsafe fn split(self: *mut ExtentNode, size: usize) {
    //    // TODO
    //}

    /// Helps merge this extent with its successor.
    #[inline]
    unsafe fn help_merge(self: *mut ExtentNode) {
        // TODO
    }

    /// Helps split this extent in two at the `size` byte boundary.
    #[inline]
    unsafe fn help_split(self: *mut ExtentNode, size: usize) {
        // Get a pointer to the split point of the extent.
        let _next = (self as usize).wrapping_add(size) as *mut ExtentNode;
        // Insert the excess extent back into the free skip lists.
        //(*self).size_list.insert(next, size);
    }
}

impl AddrList {
    /// Returns a pointer to the extent in which this list resides.
    #[inline]
    unsafe fn extent(&self) -> *mut ExtentNode {
        (self as *const AddrList as usize & EXTENT_ADDR_MASK) as *mut ExtentNode
    }

    /// Inserts a `node` into this skip list, returning a reference to the base
    /// link of the inserted node, or `nil` if the node was already found in
    /// the list.
    unsafe fn insert(&self, node: AddrNodeRef) -> AddrLinkRef {
        // Find a pair of references to consecutive base links whose extent
        // keys bound the key of the extent of the inserted node. The returned
        // next reference will be nil if the key of the extent of the inserted
        // node is greater than the keys of the extents of all links in the
        // base level list.
        let (mut prev, mut next) = self.search_to_level(node.key(), 0);
        // Check if this node is already in the list.
        if prev.key() == node.key() {
            panic!("duplicate node");
        }
        // Generate a random height for the inserted node.
        let height = self.random_height();
        // Get a pointer to the link at the base level of the inserted node,
        // stealing the given node's reference.
        let base = AddrLinkRef::from_raw((*node.ptr).levels.as_ptr() as *mut AddrLink);
        // Discard the node, whose reference we stole.
        mem::forget(node);
        // Start with the base link.
        let mut link = base.clone();
        // Of the base level list.
        let mut level = 0;
        // Loop until `height` link levels have been inserted into the skip list.
        loop {
            // Insert this node's link for the current level into the level list,
            // between the links that bound the address of the extent of the
            // inserted node..
            let (new_prev, result) = link.clone().insert(prev, next, level);
            // Update to the latest predecessor link.
            prev = new_prev;
            // Check if the insert failed due to a duplicate base link.
            if result.is_nil() && level == 0 {
                // Propagate the insert failure.
                return AddrLinkRef::nil();
            }
            // Check if the inserted base link has already become superfluous.
            if (*base.ptr).succ.load(SeqCst) & REMOVE_FLAG != 0 {
                // Check if the current link was inserted, and isn't the base link.
                if result.ptr == link.ptr && link.ptr != base.ptr {
                    // If so, remove the superfluous link.
                    link.remove(prev, level);
                }
                // Return the superfluous base link.
                return base;
            }
            // Increment the link level.
            level = level.wrapping_add(1);
            // Check if the new link level exceeds the chosen height for the
            // inserted node.
            if level > height {
                // All desired levels have been inserted; return the base link.
                return base;
            }
            // Ascend to the next higher level link.
            link.ascend();
            // Find a pair of pointers to consecutive links in the current
            // level whose extent addresses bound the address of the extent of
            // the inserted node. The returned next pointer will be null if the
            // address of the extent of the inserted node is greater than the
            // addresses of the extents of all links in the current level list.
            let (new_prev, new_next) = self.search_to_level(link.key(), level);
            // Recurse into the lower bound link of the current level list.
            prev = new_prev;
            // And the upper bound link of the current level list.
            next = new_next;
        }
    }

    /// Removes the node with the given `key`, and greatest lower bound, from
    /// this skip list, returning a reference to the removed node's base link,
    /// or `nil` if no node with the given `key` was found in the list.
    unsafe fn remove(&self, key: AddrKey, glb: AddrKey) -> AddrLinkRef {
        // Find a pair consecutive base link references whose extent keys bound
        // the `key` of the node to remove, and its greatest lower bound key.
        let (prev, del) = self.search_to_level(glb, 0);
        // Check if the upper bound link doesn't match the node to remove,
        // indicating that the node is not present in the list.
        if del.key() != key {
            // Return nil to indicate that the node was not found.
            return AddrLinkRef::nil();
        }
        // Load the status field of the extent in which this link resides;
        // synchronized by subsequent CAS.
        let mut old_status = (*self.extent()).status.load(Relaxed);
        // Loop until the remove flag is set.
        loop {
            // Set the remove flag on the status field.
            let new_status = old_status | REMOVE_FLAG;
            // Try to update the status field of the extent in which this link resides,
            // synchronizing with other list mutations; linearization point for skip
            // list removal.
            match (*self.extent()).status.compare_exchange(old_status, new_status, SeqCst, Relaxed) {
                // CAS succeeded.
                Ok(_) => break,
                // CAS failed; try again with the latest status.
                Err(status) => old_status = status,
            }
        }
        // Remove the node from the base level list.
        let result = del.remove(prev, 0);
        // Check if the removal of the link from the base level list failed.
        if result.is_nil() {
            // Return nil to indicate that the node was already removed.
            return AddrLinkRef::nil();
        }
        // Delete the links at the higher levels of the node.
        self.search_to_level(key, 1);
        // Return a pointer to the base link of the successfully removed node.
        return result;
    }

    /// Returns two consecutive links in the `target_level` list, with the
    /// extent of the first link having a key less than or equal to the search
    /// `key`, and the extent of the second link having a key strictly greater
    /// than the search `key`. Returns `nil` for the second link if the search
    /// `key` is greater than or equal to the keys of all nodes in the list.
    unsafe fn search_to_level(&self, key: AddrKey, target_level: usize)
        -> (AddrLinkRef, AddrLinkRef)
    {
        // Get the head link and level of the highest level non-empty list
        // whose level is greater than or equal to the target level.
        let (mut link, mut level) = self.find_start(target_level);
        // Search the skip list down to the target level.
        loop {
            // Search for bounding links on the current level.
            let (prev, next) = link.search_right(key, level);
            // Step into the predecessor link on the current level.
            link = prev;
            // Check if we're still above the target level.
            if level > target_level {
                // Descend to the next lower level link.
                link.descend();
                // Decrement the level index.
                level = level.wrapping_sub(1);
            } else {
                // Return bounding links on the target level.
                return (link, next);
            }
        }
    }

    /// Returns the head link and level of the highest level non-empty list
    /// whose level is greater than or equal to the minimum start `level`.
    #[inline]
    unsafe fn find_start(&self, mut level: usize) -> (AddrLinkRef, usize) {
        // Get a pointer to the head link for the minimum start level.
        let mut link = self.head.levels.as_ptr().wrapping_add(level) as *mut AddrLink;
        // Loop until the next higher level list is empty, or the top level is reached.
        loop {
            // Get a pointer to the head link for the next highest level list.
            let up_link = link.wrapping_add(1);
            // Load the address of the next link in the next highest level list,
            // synchronizing with level list mutations.
            let up_next = ((*up_link).succ.load(SeqCst) & ADDR_MASK) as *mut AddrLink;
            // Check if the next highest level list is empty.
            if up_next.is_null() {
                // The current level is the highest non-empty level.
                break;
            }
            // Step to the head link of the next highest level list.
            link = up_link;
            // Increment the level index.
            level = level.wrapping_add(1);
            // Check if the new level is as high as the skip list goes.
            if level == MAX_LEVEL {
                // Can't go any higher.
                break;
            }
        }
        // Acquire a reference the head extent in which this list resides.
        (*self.extent()).refcount.fetch_add(1, Acquire);
        // Return the head link and level of the highest level non-empty list.
        (AddrLinkRef::from_raw(link), level)
    }

    /// Returns a pseudo-random skip node height, ranging from 0 to 31, inclusive.
    unsafe fn random_height(&self) -> usize {
        // Load the current PRNG state.
        let mut old_seed = self.head.seed.load(Relaxed);
        // Loop until a new pseudo-random number is generated.
        loop {
            // Compute the next pseudo-random number, using a simple xorshit PRNG.
            let mut new_seed = old_seed ^ old_seed << 13;
            new_seed = new_seed ^ new_seed >> 17;
            new_seed = new_seed ^ new_seed << 5;
            // Try to update the PRNG state.
            match self.head.seed.compare_exchange_weak(old_seed, new_seed, Relaxed, Relaxed) {
                // CAS succeeded.
                Ok(_) => {
                    // Truncate the pseudo-random number to 32 bits.
                    let seed = new_seed as u32;
                    // Check if the highest and/or lowest bits are unset.
                    if seed & 0x80000001 != 0x80000001 {
                        // Return the highest probability skip node height.
                        return 0;
                    }
                    // Return the number of consecutive trailing zeros, between 1 and 31.
                    return (seed & 0xFFFFFFFE).trailing_zeros() as usize
                },
                // CAS failed; update the PRNG state and try again.
                Err(seed) => old_seed = seed,
            }
        }
    }
}

impl AddrNodeRef {
    /// Returns a new reference to a skip list node from a raw pointer.
    #[inline]
    unsafe fn from_raw(ptr: *mut AddrNode) -> AddrNodeRef {
        AddrNodeRef { ptr: ptr }
    }

    /// Returns `true` if this node is the `nil` end of list sentinel.
    #[inline]
    fn is_nil(&self) -> bool {
        self.ptr.is_null()
    }

    /// Returns a pointer to the extent in which this node resides.
    #[inline]
    fn extent(&self) -> *mut ExtentNode {
        (self.ptr as usize & EXTENT_ADDR_MASK) as *mut ExtentNode
    }

    /// Returns the address of the extent in which this node resides.
    #[inline]
    fn addr(&self) -> usize {
        if !self.is_nil() {
            self.extent() as usize
        } else {
            usize::MAX
        }
    }

    /// Returns the sort key used to order this node.
    #[inline]
    fn key(&self) -> AddrKey {
        self.addr()
    }
}

impl Clone for AddrNodeRef {
    #[inline]
    fn clone(&self) -> AddrNodeRef {
        unsafe {
            self.extent().retain();
            AddrNodeRef::from_raw(self.ptr)
        }
    }
}

impl Drop for AddrNodeRef {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            self.extent().release();
        }
    }
}

impl AddrLinkRef {
    /// Returns the end of list sentinel.
    #[inline]
    fn nil() -> AddrLinkRef {
        AddrLinkRef { ptr: ptr::null_mut() }
    }

    /// Returns a new reference to a skip list link from a raw pointer.
    #[inline]
    unsafe fn from_raw(ptr: *mut AddrLink) -> AddrLinkRef {
        AddrLinkRef { ptr: ptr }
    }

    /// Returns `true` if this is the `nil` end of list sentinel.
    #[inline]
    fn is_nil(&self) -> bool {
        self.ptr.is_null()
    }

    /// Returns a pointer to the extent in which this link resides.
    #[inline]
    fn extent(&self) -> *mut ExtentNode {
        (self.ptr as usize & EXTENT_ADDR_MASK) as *mut ExtentNode
    }

    /// Returns the address of the extent in which this link resides.
    #[inline]
    fn addr(&self) -> usize {
        if !self.is_nil() {
            self.extent() as usize
        } else {
            usize::MAX
        }
    }

    /// Returns the sort key used to order this link.
    #[inline]
    fn key(&self) -> AddrKey {
        self.addr()
    }

    /// Returns the greatest key that precedes this link's key.
    #[inline]
    fn glb(&self) -> AddrKey {
        self.addr().wrapping_sub(1)
    }

    /// Updates this reference to point to the node's next higher level link.
    #[inline]
    unsafe fn ascend(&mut self) {
        self.ptr = self.ptr.wrapping_add(1);
    }

    /// Updates this reference to point to the node's next lower level link.
    #[inline]
    unsafe fn descend(&mut self) {
        self.ptr = self.ptr.wrapping_sub(1);
    }

    /// Returns `true` if the node in which this link resides is marked for removal.
    #[inline]
    unsafe fn is_superfluous(&self, level: usize) -> bool {
        !self.is_nil() && (*self.ptr.wrapping_sub(level)).succ.load(SeqCst) & REMOVE_FLAG != 0
    }

    /// Returns the insertion status of the node in which this link resides.
    #[inline]
    unsafe fn status(&self) -> usize {
        if !self.is_nil() {
            (*self.extent()).status.load(Acquire)
        } else {
            0
        }
    }

    /// Returns a reference to the successor of this link, incrementing its
    /// reference count.
    #[inline]
    unsafe fn acquire_next(&self) -> AddrLinkRef {
        // Loop until a reference is acquired to the successor of this link.
        loop {
            // Load the address of the successor to this link in the level list,
            // synchronizing with level list mutations.
            let next = AddrLinkRef::from_raw(((*self.ptr).succ.load(SeqCst) & ADDR_MASK) as *mut AddrLink);
            // Check if the next link is the end of list sentinel.
            if next.is_nil() {
                // Return a reference to the end of list sentinel.
                return next;
            }
            // Increment the reference count of the extent of the next link,
            // synchronizing with reference releases.
            (*next.extent()).refcount.fetch_add(1, Acquire);
            // Check if the successor to this link remains unchanged,
            // synchronizing with level list mutations.
            if next.ptr == ((*self.ptr).succ.load(SeqCst) & ADDR_MASK) as *mut AddrLink {
                // Return the acquired reference to the next link.
                return next;
            } else {
                // Successor changed; release the reference we acquired to the
                // previous successor, and try again.
                mem::drop(next);
            }
        }
    }

    /// Returns a reference to the predecessor of this marked link,
    /// incrementing its reference count.
    #[inline]
    unsafe fn acquire_back(&self) -> AddrLinkRef {
        // Loop until a reference is acquired to the predecessor of this link.
        loop {
            // Load the address of the predecessor to this link in the level list,
            // synchronizing with back reference releases.
            let prev = AddrLinkRef::from_raw((*self.ptr).back.load(Acquire));
            // The back pointer must not be null.
            debug_assert!(!prev.ptr.is_null());
            // The back pointer cannot be null.
            assume(!prev.ptr.is_null());
            // Increment the reference count of the extent of the prev link,
            // synchronizing with reference releases.
            (*prev.extent()).refcount.fetch_add(1, Acquire);
            // Check if the predecessor to this link remains unchanged,
            // synchronizing with back reference releases.
            if prev.ptr == (*self.ptr).back.load(Acquire) {
                // Return the acquired reference to the prev link.
                return prev;
            } else {
                // Successor changed; release the reference we acquired to the
                // previous predecessor, and try again.
                mem::drop(prev);
            }
        }
    }

    /// Returns two consecutive links in the list at `level`, with the extent
    /// of the first link having a key less than or equal to the search key,
    /// and the extent of the second link having a key strictly greater than
    /// the search `key`. Assumes that the address of the extent of this link
    /// precedes the search address.
    unsafe fn search_right(mut self, key: AddrKey, level: usize) -> (AddrLinkRef, AddrLinkRef) {
        // Acquire a reference to the next link in the level list.
        let mut next = self.acquire_next();
        // Loop while the key of the extent of the next link does not exceed the search key.
        while !next.is_nil() && next.key() <= key {
            // Load the insertion status of the extent in which the next link resides.
            let status = next.status();
            // Check if the extent is flagged for merging or splitting.
            if status & MERGE_FLAG != 0 {
                // Help merge the extent in which this link resides with its successor.
                next.extent().help_merge();
            } else if status & SPLIT_FLAG != 0 {
                // Help split the extent in which this link resides at the size boundary
                // enocded in the high order bits of the status field.
                next.extent().help_split(status & SIZE_MASK);
            }
            // Loop while the next node is marked for removal.
            while next.is_superfluous(level) {
                // The next node is marked for removal; try to freeze its predecessor.
                let (prev, result) = next.try_freeze(self, level);
                // Point the current link at the latest predecrssor of the next link.
                self = prev;
                // Check if the current link is frozen.
                if result & FREEZE_FLAG != 0 {
                    // If so, help remove the next link.
                    next.help_freeze(&self);
                }
                // Acquire a reference to the new next link in the level list.
                next = self.acquire_next();
            }
            // Check if the key of the extent of the next link still precedes
            // the search key.
            if next.key() <= key {
                // If so, step into the next link.
                self = next;
                // And acquire a reference to the new current link's successor.
                next = self.acquire_next();
            }
        }
        // The key of the extent of the next link now exceeds the search key;
        // return the consecutive links, which bound the search key.
        (self, next)
    }

    /// Inserts this link into the list at `level`, between the `prev` link and
    /// the `next` link; returns the predecessor to the inserted link, and the
    /// inserted link itself.
    unsafe fn insert(self, mut prev: AddrLinkRef, mut next: AddrLinkRef, level: usize)
        -> (AddrLinkRef, AddrLinkRef)
    {
        // Check if this link is already inserted into the list.
        if prev.key() == self.key() {
            // Duplicate link.
            return (prev, AddrLinkRef::nil());
        }
        // Loop until the link is inserted into the list.
        loop {
            // Load the successor to the predecessor link, synchronizing with
            // list mutations.
            let prev_succ = (*prev.ptr).succ.load(SeqCst);
            // Check if the successor to the predecessor link is frozen.
            if prev_succ & FREEZE_FLAG != 0 {
                // If so, borrow the predecessor's reference to its successor;
                // safe because the predecessor is frozen.
                let succ = AddrLinkRef::from_raw((prev_succ & ADDR_MASK) as *mut AddrLink);
                // Help remove the predecessor's successor.
                succ.help_freeze(&prev);
                // Discard the borrowed successor reference.
                mem::forget(succ);
            } else {
                // Set the successor of this link to the next link in this list;
                // no need to synchronize because this new link is not aliased.
                (*self.ptr).succ.store(next.ptr as usize, Relaxed);
                // Try to set the successor of the predecessor link to this link,
                // synchronizing with list mutations; linearization point for
                // link insertion.
                match (*prev.ptr).succ.compare_exchange(next.ptr as usize,
                                                        self.ptr as usize,
                                                        SeqCst, SeqCst) {
                    // Link successfully inserted; return the new link bounds.
                    Ok(_) => return (prev, self),
                    // Failed to insert link.
                    Err(result) => {
                        // Check if the predecessor link was frozen.
                        if result & FREEZE_FLAG != 0 {
                            // If so, borrow the predecessor's reference to its successor;
                            // safe because the predecessor is frozen.
                            let succ = AddrLinkRef::from_raw((result & ADDR_MASK) as *mut AddrLink);
                            // Help remove the predecessor's successor.
                            succ.help_freeze(&prev);
                            // Discard the borrowed successor reference.
                            mem::forget(succ);
                        }
                        // Loop while the sucessor to the predecessor link is
                        // marked for removal, synchronizing with list mutations.
                        while (*prev.ptr).succ.load(SeqCst) & REMOVE_FLAG != 0 {
                            // Acquire a reference to the predecessor of the predecessor link.
                            prev = prev.acquire_back();
                        }
                    },
                }
            }
            // Get the links that bound the search address in the level list.
            let (new_prev, new_next) = prev.search_right(self.key(), level);
            // Update the predecessor link.
            prev = new_prev;
            // Update the successor link.
            next = new_next;
            // Check if this link is already inserted into the list.
            if prev.key() == self.key() {
                // Duplicate link.
                return (prev, AddrLinkRef::nil());
            }
        }
    }

    /// Removes this link from the `level` list, following the `prev` link.
    unsafe fn remove(self, mut prev: AddrLinkRef, level: usize) -> AddrLinkRef {
        // Try to freeze the predecessor of this link.
        let (new_prev, result) = self.try_freeze(prev, level);
        // Update to the latest predecessor of this link.
        prev = new_prev;
        // Check if the predecessor link is frozen.
        if result & FREEZE_FLAG != 0 {
            // If so, help remove this link.
            self.help_freeze(&prev);
        }
        // Check if this link was not in the list.
        if result & REMOVE_FLAG == 0 {
            // Return nil to indicate that no link was removed.
            return AddrLinkRef::nil();
        }
        // Return a pointer to the removed link.
        return self;
    }

    /// Attempts to set the freeze bit on the successor pointer of the predecessor
    /// of this link, freezing the predecessor from concurrent mutation while
    /// the node in which this link resides is removed. Returns the latest
    /// predecessor to this link, and a bit mask with the freeze bit set if the
    /// predecessor is frozen, and with the remove flag set if this operation
    /// is the one that transitioned the predecessor into the frozen state.
    #[inline]
    unsafe fn try_freeze(&self, mut prev: AddrLinkRef, level: usize) -> (AddrLinkRef, usize) {
        loop {
            // Check if the predecessor is already frozen.
            if (*prev.ptr).succ.load(SeqCst) == self.ptr as usize & FREEZE_FLAG {
                // Already frozen.
                return (prev, FREEZE_FLAG);
            }
            // Try to set the freeze bit on the successor pointer of the predecessor link;
            // linearization point for successor removal.
            match (*prev.ptr).succ.compare_exchange(self.ptr as usize,
                                                    self.ptr as usize | FREEZE_FLAG,
                                                    SeqCst, SeqCst) {
                // Successfully frozen.
                Ok(_) => return (prev, FREEZE_FLAG | REMOVE_FLAG),
                // Failed to set freeze bit.
                Err(result) => {
                    // Check if this link is still the successor to the precedessor,
                    // and if the successor pointer to the predecessor link has its
                    // freeze bit set.
                    if result == self.ptr as usize | FREEZE_FLAG {
                        // Predecessor was concurrently frozen; return the address
                        // of the predecessor link, with the freeze bit set.
                        return (prev, FREEZE_FLAG);
                    }
                    // Loop while the sucessor to the predecessor link is
                    // marked for removal, synchronizing with list mutations.
                    while (*prev.ptr).succ.load(SeqCst) & REMOVE_FLAG != 0 {
                        // Acquire a reference to the predecessor of the predecessor link.
                        prev = prev.acquire_back();
                    }
                    // Successor to the predecessor concurrently changed;
                    // search for the links that bound the key
                    // "infinitessimally" smaller than this key.
                    let (new_prev, new_next) = prev.search_right(self.glb(), level);
                    // Update to the latest predecessor link.
                    prev = new_prev;
                    // Check if this link is no longer the successor to the
                    // predecessor of this key.
                    if new_next.ptr != self.ptr {
                        // This link was concurrently removed; return the address
                        // of the predecessor link.
                        return (prev, 0);
                    }
                },
            }
        }
    }

    /// Attempts to mark and remove this link, which is a successor of the `prev` link.
    #[inline]
    unsafe fn help_freeze(&self, prev: &AddrLinkRef) {
        // Store a weak reference to the predecessor link in the back pointer
        // of this link, synchronizing with back pointer reads.
        (*self.ptr).back.store(prev.ptr, Release);
        // Check if this link hasn't yet been marked for removal.
        if (*self.ptr).succ.load(SeqCst) & REMOVE_FLAG == 0 {
            // Try to mark this link for removal.
            self.try_remove();
        }
        // Help physically remove this link.
        self.help_remove(prev);
    }

    /// Attempts to mark this link for removal.
    #[inline]
    unsafe fn try_remove(&self) {
        // Loops until this link is marked for removal.
        loop {
            // Load the address of the successor to this link,
            // synchronized by subsequent CAS.
            let next = (*self.ptr).succ.load(Relaxed) & ADDR_MASK;
            // Try to set the remove flag of the successor pointer;
            // linearization point for logical link removal.
            match (*self.ptr).succ.compare_exchange(next, next | REMOVE_FLAG, SeqCst, SeqCst) {
                // Successfully marked.
                Ok(_) => break,
                // Failed to set remove flag.
                Err(result) => {
                    // Check if this link is frozen.
                    if result & FREEZE_FLAG != 0 {
                        // Borrow this link's reference to its successor;
                        // safe because this link is frozen.
                        let succ = AddrLinkRef::from_raw((result & ADDR_MASK) as *mut AddrLink);
                        // Help remove the successor of the successor of this link.
                        succ.acquire_next().help_freeze(self);
                        // Discard the borrowed successor reference.
                        mem::forget(succ);
                    }
                    // Check if this link is marked.
                    if result & REMOVE_FLAG != 0 {
                        // Link already marked for removal.
                        break;
                    }
                },
            }
        }
    }

    /// Attempts to physically remove this link from the list.
    #[inline]
    unsafe fn help_remove(&self, prev: &AddrLinkRef) {
        // Load the successor of this link, synchronizing with list mutation.
        let next = (*self.ptr).succ.load(SeqCst) & ADDR_MASK;
        // Try to set the successor of the predecessor link to the successor of
        // this link; linearization point for physical link removal.
        let _ = (*prev.ptr).succ.compare_exchange(self.ptr as usize | FREEZE_FLAG, next, SeqCst, Relaxed);
    }

    /// Consumes this link reference, returning a raw, uncounted pointer to the
    /// extent in which this link resides.
    unsafe fn into_extent(self) -> *mut ExtentNode {
        self.extent()
    }
}

impl Clone for AddrLinkRef {
    #[inline]
    fn clone(&self) -> AddrLinkRef {
        unsafe {
            self.extent().retain();
            AddrLinkRef::from_raw(self.ptr)
        }
    }
}

impl Drop for AddrLinkRef {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            self.extent().release();
        }
    }
}

impl SizeList {
    /// Returns a pointer to the extent in which this list resides.
    #[inline]
    unsafe fn extent(&self) -> *mut ExtentNode {
        (self as *const SizeList as usize & EXTENT_ADDR_MASK) as *mut ExtentNode
    }

    /// Inserts a `node` into this skip list, returning a reference to the base
    /// link of the inserted node, or `nil` if the node was already found in
    /// the list.
    unsafe fn insert(&self, node: SizeNodeRef) -> SizeLinkRef {
        // Find a pair of references to consecutive base links whose extent
        // keys bound the key of the extent of the inserted node. The returned
        // next reference will be nil if the key of the extent of the inserted
        // node is greater than the keys of the extents of all links in the
        // base level list.
        let (mut prev, mut next) = self.search_to_level(node.key(), 0);
        // Check if this node is already in the list.
        if prev.key() == node.key() {
            panic!("duplicate node");
        }
        // Generate a random height for the inserted node.
        let height = self.random_height();
        // Get a pointer to the link at the base level of the inserted node,
        // stealing the given node's reference.
        let base = SizeLinkRef::from_raw((*node.ptr).levels.as_ptr() as *mut SizeLink);
        // Discard the node, whose reference we stole.
        mem::forget(node);
        // Start with the base link.
        let mut link = base.clone();
        // Of the base level list.
        let mut level = 0;
        // Loop until `height` link levels have been inserted into the skip list.
        loop {
            // Insert this node's link for the current level into the level list,
            // between the links that bound the address of the extent of the
            // inserted node..
            let (new_prev, result) = link.clone().insert(prev, next, level);
            // Update to the latest predecessor link.
            prev = new_prev;
            // Check if the insert failed due to a duplicate base link.
            if result.is_nil() && level == 0 {
                // Propagate the insert failure.
                return SizeLinkRef::nil();
            }
            // Check if the inserted base link has already become superfluous.
            if (*base.ptr).succ.load(SeqCst) & REMOVE_FLAG != 0 {
                // Check if the current link was inserted, and isn't the base link.
                if result.ptr == link.ptr && link.ptr != base.ptr {
                    // If so, remove the superfluous link.
                    link.remove(prev, level);
                }
                // Return the superfluous base link.
                return base;
            }
            // Increment the link level.
            level = level.wrapping_add(1);
            // Check if the new link level exceeds the chosen height for the
            // inserted node.
            if level > height {
                // All desired levels have been inserted; return the base link.
                return base;
            }
            // Ascend to the next higher level link.
            link.ascend();
            // Find a pair of pointers to consecutive links in the current
            // level whose extent addresses bound the address of the extent of
            // the inserted node. The returned next pointer will be null if the
            // address of the extent of the inserted node is greater than the
            // addresses of the extents of all links in the current level list.
            let (new_prev, new_next) = self.search_to_level(link.key(), level);
            // Recurse into the lower bound link of the current level list.
            prev = new_prev;
            // And the upper bound link of the current level list.
            next = new_next;
        }
    }

    /// Removes the first node from the least with a key greater than a given
    /// greatest lower bound (`glb`) key, returning a reference to the removed
    /// node's base link, or `nil` of no node with a key greater than `glb` was
    /// found in the list.
    unsafe fn take_next(&self, glb: SizeKey) -> SizeLinkRef {
        loop {
            // Find a pair consecutive base link references whose upper bound key
            // is greater than the given greatest lower bound key.
            let (prev, link) = self.search_to_level(glb, 0);
            // Check if no node was found whose key is greater than the given
            // greatest lower bound key.
            if link.is_nil() {
                // Return nil to indicate that no node was found.
                return link;
            }
            // Load the status field of the extent in which this link resides;
            // synchronized by subsequent CAS.
            let mut old_status = (*self.extent()).status.load(Relaxed);
            // Loop until the remove flag is set.
            loop {
                // Set the remove flag on the status field.
                let new_status = old_status | REMOVE_FLAG;
                // Try to update the status field of the extent in which this link resides,
                // synchronizing with other list mutations; linearization point for skip
                // list removal.
                match (*self.extent()).status.compare_exchange(old_status, new_status, SeqCst, Relaxed) {
                    // CAS succeeded.
                    Ok(_) => break,
                    // CAS failed; try again with the latest status.
                    Err(status) => old_status = status,
                }
            }
            // Remove the node from the base level list.
            let result = link.remove(prev, 0);
            // Check if the removal of the link from the base level list failed.
            if result.is_nil() {
                // The node was concurrently removed; try again.
                continue;
            }
            // Delete the links at the higher levels of the node.
            self.search_to_level(result.key(), 1);
            // Return a pointer to the base link of the successfully removed node.
            return result;
        }
    }

    /// Returns two consecutive links in the `target_level` list, with the
    /// extent of the first link having a key less than or equal to the search
    /// `key`, and the extent of the second link having a key strictly greater
    /// than the search `key`. Returns `nil` for the second link if the search
    /// `key` is greater than or equal to the keys of all nodes in the list.
    unsafe fn search_to_level(&self, key: SizeKey, target_level: usize)
        -> (SizeLinkRef, SizeLinkRef)
    {
        // Get the head link and level of the highest level non-empty list
        // whose level is greater than or equal to the target level.
        let (mut link, mut level) = self.find_start(target_level);
        // Search the skip list down to the target level.
        loop {
            // Search for bounding links on the current level.
            let (prev, next) = link.search_right(key, level);
            // Step into the predecessor link on the current level.
            link = prev;
            // Check if we're still above the target level.
            if level > target_level {
                // Descend to the next lower level link.
                link.descend();
                // Decrement the level index.
                level = level.wrapping_sub(1);
            } else {
                // Return bounding links on the target level.
                return (link, next);
            }
        }
    }

    /// Returns the head link and level of the highest level non-empty list
    /// whose level is greater than or equal to the minimum start `level`.
    #[inline]
    unsafe fn find_start(&self, mut level: usize) -> (SizeLinkRef, usize) {
        // Get a pointer to the head link for the minimum start level.
        let mut link = self.head.levels.as_ptr().wrapping_add(level) as *mut SizeLink;
        // Loop until the next higher level list is empty, or the top level is reached.
        loop {
            // Get a pointer to the head link for the next highest level list.
            let up_link = link.wrapping_add(1);
            // Load the address of the next link in the next highest level list,
            // synchronizing with level list mutations.
            let up_next = ((*up_link).succ.load(SeqCst) & ADDR_MASK) as *mut SizeLink;
            // Check if the next highest level list is empty.
            if up_next.is_null() {
                // The current level is the highest non-empty level.
                break;
            }
            // Step to the head link of the next highest level list.
            link = up_link;
            // Increment the level index.
            level = level.wrapping_add(1);
            // Check if the new level is as high as the skip list goes.
            if level == MAX_LEVEL {
                // Can't go any higher.
                break;
            }
        }
        // Acquire a reference the head extent in which this list resides.
        (*self.extent()).refcount.fetch_add(1, Acquire);
        // Return the head link and level of the highest level non-empty list.
        (SizeLinkRef::from_raw(link), level)
    }

    /// Returns a pseudo-random skip node height, ranging from 0 to 31, inclusive.
    unsafe fn random_height(&self) -> usize {
        // Load the current PRNG state.
        let mut old_seed = self.head.seed.load(Relaxed);
        // Loop until a new pseudo-random number is generated.
        loop {
            // Compute the next pseudo-random number, using a simple xorshit PRNG.
            let mut new_seed = old_seed ^ old_seed << 13;
            new_seed = new_seed ^ new_seed >> 17;
            new_seed = new_seed ^ new_seed << 5;
            // Try to update the PRNG state.
            match self.head.seed.compare_exchange_weak(old_seed, new_seed, Relaxed, Relaxed) {
                // CAS succeeded.
                Ok(_) => {
                    // Truncate the pseudo-random number to 32 bits.
                    let seed = new_seed as u32;
                    // Check if the highest and/or lowest bits are unset.
                    if seed & 0x80000001 != 0x80000001 {
                        // Return the highest probability skip node height.
                        return 0;
                    }
                    // Return the number of consecutive trailing zeros, between 1 and 31.
                    return (seed & 0xFFFFFFFE).trailing_zeros() as usize
                },
                // CAS failed; update the PRNG state and try again.
                Err(seed) => old_seed = seed,
            }
        }
    }
}

impl SizeNodeRef {
    /// Returns a new reference to a skip list node from a raw pointer.
    #[inline]
    unsafe fn from_raw(ptr: *mut SizeNode) -> SizeNodeRef {
        SizeNodeRef { ptr: ptr }
    }

    /// Returns a pointer to the extent in which this node resides.
    #[inline]
    fn extent(&self) -> *mut ExtentNode {
        (self.ptr as usize & EXTENT_ADDR_MASK) as *mut ExtentNode
    }

    /// Returns the sort key used to order this node.
    #[inline]
    fn key(&self) -> SizeKey {
        unsafe {
            let extent = self.extent();
            if !extent.is_null() {
                ((*extent).size, extent as usize)
            } else {
                (usize::MAX, usize::MAX)
            }
        }
    }
}

impl Clone for SizeNodeRef {
    #[inline]
    fn clone(&self) -> SizeNodeRef {
        unsafe {
            self.extent().retain();
            SizeNodeRef::from_raw(self.ptr)
        }
    }
}

impl Drop for SizeNodeRef {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            self.extent().release();
        }
    }
}

impl SizeLinkRef {
    /// Returns the end of list sentinel.
    #[inline]
    fn nil() -> SizeLinkRef {
        SizeLinkRef { ptr: ptr::null_mut() }
    }

    /// Returns a new reference to a skip list link from a raw pointer.
    #[inline]
    unsafe fn from_raw(ptr: *mut SizeLink) -> SizeLinkRef {
        SizeLinkRef { ptr: ptr }
    }

    /// Updates this reference to point to the node's next higher level link.
    #[inline]
    unsafe fn ascend(&mut self) {
        self.ptr = self.ptr.wrapping_add(1);
    }

    /// Updates this reference to point to the node's next lower level link.
    #[inline]
    unsafe fn descend(&mut self) {
        self.ptr = self.ptr.wrapping_sub(1);
    }

    /// Returns `true` if this is the `nil` end of list sentinel.
    #[inline]
    fn is_nil(&self) -> bool {
        self.ptr.is_null()
    }

    /// Returns a pointer to the extent in which this link resides.
    #[inline]
    fn extent(&self) -> *mut ExtentNode {
        (self.ptr as usize & EXTENT_ADDR_MASK) as *mut ExtentNode
    }

    /// Returns the address of the extent in which this link resides.
    #[inline]
    fn addr(&self) -> usize {
        if !self.is_nil() {
            self.extent() as usize
        } else {
            usize::MAX
        }
    }

    /// Returns the sort key used to order this link.
    #[inline]
    fn key(&self) -> (usize, usize) {
        unsafe {
            let extent = self.extent();
            if !extent.is_null() {
                ((*extent).size, extent as usize)
            } else {
                (usize::MAX, usize::MAX)
            }
        }
    }

    /// Returns the greatest key that precedes this link's key.
    #[inline]
    fn glb(&self) -> (usize, usize) {
        unsafe {
            let extent = self.extent();
            if !extent.is_null() {
                ((*extent).size.wrapping_sub(1), extent as usize)
            } else {
                (usize::MAX, usize::MAX)
            }
        }
    }

    /// Returns `true` if the node in which this link resides is marked for removal.
    #[inline]
    unsafe fn is_superfluous(&self, level: usize) -> bool {
        !self.is_nil() && (*self.ptr.wrapping_sub(level)).succ.load(SeqCst) & REMOVE_FLAG != 0
    }

    /// Returns the insertion status of the node in which this link resides.
    #[inline]
    unsafe fn status(&self) -> usize {
        if !self.is_nil() {
            (*self.extent()).status.load(Acquire)
        } else {
            0
        }
    }

    /// Returns a reference to the successor of this link, incrementing its
    /// reference count.
    #[inline]
    unsafe fn acquire_next(&self) -> SizeLinkRef {
        // Loop until a reference is acquired to the successor of this link.
        loop {
            // Load the address of the successor to this link in the level list,
            // synchronizing with level list mutations.
            let next = SizeLinkRef::from_raw(((*self.ptr).succ.load(SeqCst) & ADDR_MASK) as *mut SizeLink);
            // Check if the next link is the end of list sentinel.
            if next.is_nil() {
                // Return a reference to the end of list sentinel.
                return next;
            }
            // Increment the reference count of the extent of the next link,
            // synchronizing with reference releases.
            (*next.extent()).refcount.fetch_add(1, Acquire);
            // Check if the successor to this link remains unchanged,
            // synchronizing with level list mutations.
            if next.ptr == ((*self.ptr).succ.load(SeqCst) & ADDR_MASK) as *mut SizeLink {
                // Return the acquired reference to the next link.
                return next;
            } else {
                // Successor changed; release the reference we acquired to the
                // previous successor, and try again.
                mem::drop(next);
            }
        }
    }

    /// Returns a reference to the predecessor of this marked link,
    /// incrementing its reference count.
    #[inline]
    unsafe fn acquire_back(&self) -> SizeLinkRef {
        // Loop until a reference is acquired to the predecessor of this link.
        loop {
            // Load the address of the predecessor to this link in the level list,
            // synchronizing with back reference releases.
            let prev = SizeLinkRef::from_raw((*self.ptr).back.load(Acquire));
            // The back pointer must not be null.
            debug_assert!(!prev.ptr.is_null());
            // The back pointer cannot be null.
            assume(!prev.ptr.is_null());
            // Increment the reference count of the extent of the prev link,
            // synchronizing with reference releases.
            (*prev.extent()).refcount.fetch_add(1, Acquire);
            // Check if the predecessor to this link remains unchanged,
            // synchronizing with back reference releases.
            if prev.ptr == (*self.ptr).back.load(Acquire) {
                // Return the acquired reference to the prev link.
                return prev;
            } else {
                // Successor changed; release the reference we acquired to the
                // previous predecessor, and try again.
                mem::drop(prev);
            }
        }
    }

    /// Returns two consecutive links in the list at `level`, with the extent
    /// of the first link having a key less than or equal to the search key,
    /// and the extent of the second link having a key strictly greater than
    /// the search `key`. Assumes that the address of the extent of this link
    /// precedes the search address.
    unsafe fn search_right(mut self, key: SizeKey, level: usize) -> (SizeLinkRef, SizeLinkRef) {
        // Acquire a reference to the next link in the level list.
        let mut next = self.acquire_next();
        // Loop while the key of the extent of the next link does not exceed the search key.
        while !next.is_nil() && next.key() <= key {
            // Load the insertion status of the extent in which the next link resides.
            let status = next.status();
            // Check if the extent is flagged for merging or splitting.
            if status & MERGE_FLAG != 0 {
                // Help merge the extent in which this link resides with its successor.
                next.extent().help_merge();
            } else if status & SPLIT_FLAG != 0 {
                // Help split the extent in which this link resides at the size boundary
                // enocded in the high order bits of the status field.
                next.extent().help_split(status & SIZE_MASK);
            }
            // Loop while the next node is marked for removal.
            while next.is_superfluous(level) {
                // The next node is marked for removal; try to freeze its predecessor.
                let (prev, result) = next.try_freeze(self, level);
                // Point the current link at the latest predecrssor of the next link.
                self = prev;
                // Check if the current link is frozen.
                if result & FREEZE_FLAG != 0 {
                    // If so, help remove the next link.
                    next.help_freeze(&self);
                }
                // Acquire a reference to the new next link in the level list.
                next = self.acquire_next();
            }
            // Check if the key of the extent of the next link still precedes
            // the search key.
            if next.key() <= key {
                // If so, step into the next link.
                self = next;
                // And acquire a reference to the new current link's successor.
                next = self.acquire_next();
            }
        }
        // The key of the extent of the next link now exceeds the search key;
        // return the consecutive links, which bound the search key.
        (self, next)
    }

    /// Inserts this link into the list at `level`, between the `prev` link and
    /// the `next` link; returns the predecessor to the inserted link, and the
    /// inserted link itself.
    unsafe fn insert(self, mut prev: SizeLinkRef, mut next: SizeLinkRef, level: usize)
        -> (SizeLinkRef, SizeLinkRef)
    {
        // Check if this link is already inserted into the list.
        if prev.key() == self.key() {
            // Duplicate link.
            return (prev, SizeLinkRef::nil());
        }
        // Loop until the link is inserted into the list.
        loop {
            // Load the successor to the predecessor link, synchronizing with
            // list mutations.
            let prev_succ = (*prev.ptr).succ.load(SeqCst);
            // Check if the successor to the predecessor link is frozen.
            if prev_succ & FREEZE_FLAG != 0 {
                // If so, borrow the predecessor's reference to its successor;
                // safe because the predecessor is frozen.
                let succ = SizeLinkRef::from_raw((prev_succ & ADDR_MASK) as *mut SizeLink);
                // Help remove the predecessor's successor.
                succ.help_freeze(&prev);
                // Discard the borrowed successor reference.
                mem::forget(succ);
            } else {
                // Set the successor of this link to the next link in this list;
                // no need to synchronize because this new link is not aliased.
                (*self.ptr).succ.store(next.ptr as usize, Relaxed);
                // Try to set the successor of the predecessor link to this link,
                // synchronizing with list mutations; linearization point for
                // link insertion.
                match (*prev.ptr).succ.compare_exchange(next.ptr as usize,
                                                        self.ptr as usize,
                                                        SeqCst, SeqCst) {
                    // Link successfully inserted; return the new link bounds.
                    Ok(_) => return (prev, self),
                    // Failed to insert link.
                    Err(result) => {
                        // Check if the predecessor link was frozen.
                        if result & FREEZE_FLAG != 0 {
                            // If so, borrow the predecessor's reference to its successor;
                            // safe because the predecessor is frozen.
                            let succ = SizeLinkRef::from_raw((result & ADDR_MASK) as *mut SizeLink);
                            // Help remove the predecessor's successor.
                            succ.help_freeze(&prev);
                            // Discard the borrowed successor reference.
                            mem::forget(succ);
                        }
                        // Loop while the sucessor to the predecessor link is
                        // marked for removal, synchronizing with list mutations.
                        while (*prev.ptr).succ.load(SeqCst) & REMOVE_FLAG != 0 {
                            // Acquire a reference to the predecessor of the predecessor link.
                            prev = prev.acquire_back();
                        }
                    },
                }
            }
            // Get the links that bound the search address in the level list.
            let (new_prev, new_next) = prev.search_right(self.key(), level);
            // Update the predecessor link.
            prev = new_prev;
            // Update the successor link.
            next = new_next;
            // Check if this link is already inserted into the list.
            if prev.key() == self.key() {
                // Duplicate link.
                return (prev, SizeLinkRef::nil());
            }
        }
    }

    /// Removes this link from the `level` list, following the `prev` link.
    unsafe fn remove(self, mut prev: SizeLinkRef, level: usize) -> SizeLinkRef {
        // Try to freeze the predecessor of this link.
        let (new_prev, result) = self.try_freeze(prev, level);
        // Update to the latest predecessor of this link.
        prev = new_prev;
        // Check if the predecessor link is frozen.
        if result & FREEZE_FLAG != 0 {
            // If so, help remove this link.
            self.help_freeze(&prev);
        }
        // Check if this link was not in the list.
        if result & REMOVE_FLAG == 0 {
            // Return nil to indicate that no link was removed.
            return SizeLinkRef::nil();
        }
        // Return a pointer to the removed link.
        return self;
    }

    /// Attempts to set the freeze bit on the successor pointer of the predecessor
    /// of this link, freezing the predecessor from concurrent mutation while
    /// the node in which this link resides is removed. Returns the latest
    /// predecessor to this link, and a bit mask with the freeze bit set if the
    /// predecessor is frozen, and with the remove flag set if this operation
    /// is the one that transitioned the predecessor into the frozen state.
    #[inline]
    unsafe fn try_freeze(&self, mut prev: SizeLinkRef, level: usize) -> (SizeLinkRef, usize) {
        loop {
            // Check if the predecessor is already frozen.
            if (*prev.ptr).succ.load(SeqCst) == self.ptr as usize & FREEZE_FLAG {
                // Already frozen.
                return (prev, FREEZE_FLAG);
            }
            // Try to set the freeze bit on the successor pointer of the predecessor link;
            // linearization point for successor removal.
            match (*prev.ptr).succ.compare_exchange(self.ptr as usize,
                                                    self.ptr as usize | FREEZE_FLAG,
                                                    SeqCst, SeqCst) {
                // Successfully frozen.
                Ok(_) => return (prev, FREEZE_FLAG | REMOVE_FLAG),
                // Failed to set freeze bit.
                Err(result) => {
                    // Check if this link is still the successor to the precedessor,
                    // and if the successor pointer to the predecessor link has its
                    // freeze bit set.
                    if result == self.ptr as usize | FREEZE_FLAG {
                        // Predecessor was concurrently frozen; return the address
                        // of the predecessor link, with the freeze bit set.
                        return (prev, FREEZE_FLAG);
                    }
                    // Loop while the sucessor to the predecessor link is
                    // marked for removal, synchronizing with list mutations.
                    while (*prev.ptr).succ.load(SeqCst) & REMOVE_FLAG != 0 {
                        // Acquire a reference to the predecessor of the predecessor link.
                        prev = prev.acquire_back();
                    }
                    // Successor to the predecessor concurrently changed;
                    // search for the links that bound the key
                    // "infinitessimally" smaller than this key.
                    let (new_prev, new_next) = prev.search_right(self.glb(), level);
                    // Update to the latest predecessor link.
                    prev = new_prev;
                    // Check if this link is no longer the successor to the
                    // predecessor of this key.
                    if new_next.ptr != self.ptr {
                        // This link was concurrently removed; return the address
                        // of the predecessor link.
                        return (prev, 0);
                    }
                },
            }
        }
    }

    /// Attempts to mark and remove this link, which is a successor of the `prev` link.
    #[inline]
    unsafe fn help_freeze(&self, prev: &SizeLinkRef) {
        // Store a weak reference to the predecessor link in the back pointer
        // of this link, synchronizing with back pointer reads.
        (*self.ptr).back.store(prev.ptr, Release);
        // Check if this link hasn't yet been marked for removal.
        if (*self.ptr).succ.load(SeqCst) & REMOVE_FLAG == 0 {
            // Try to mark this link for removal.
            self.try_remove();
        }
        // Help physically remove this link.
        self.help_remove(prev);
    }

    /// Attempts to mark this link for removal.
    #[inline]
    unsafe fn try_remove(&self) {
        // Loops until this link is marked for removal.
        loop {
            // Load the address of the successor to this link,
            // synchronized by subsequent CAS.
            let next = (*self.ptr).succ.load(Relaxed) & ADDR_MASK;
            // Try to set the remove flag of the successor pointer;
            // linearization point for logical link removal.
            match (*self.ptr).succ.compare_exchange(next, next | REMOVE_FLAG, SeqCst, SeqCst) {
                // Successfully marked.
                Ok(_) => break,
                // Failed to set remove flag.
                Err(result) => {
                    // Check if this link is frozen.
                    if result & FREEZE_FLAG != 0 {
                        // Borrow this link's reference to its successor;
                        // safe because this link is frozen.
                        let succ = SizeLinkRef::from_raw((result & ADDR_MASK) as *mut SizeLink);
                        // Help remove the successor of the successor of this link.
                        succ.acquire_next().help_freeze(self);
                        // Discard the borrowed successor reference.
                        mem::forget(succ);
                    }
                    // Check if this link is marked.
                    if result & REMOVE_FLAG != 0 {
                        // Link already marked for removal.
                        break;
                    }
                },
            }
        }
    }

    /// Attempts to physically remove this link from the list.
    #[inline]
    unsafe fn help_remove(&self, prev: &SizeLinkRef) {
        // Load the successor of this link, synchronizing with list mutation.
        let next = (*self.ptr).succ.load(SeqCst) & ADDR_MASK;
        // Try to set the successor of the predecessor link to the successor of
        // this link; linearization point for physical link removal.
        let _ = (*prev.ptr).succ.compare_exchange(self.ptr as usize | FREEZE_FLAG, next, SeqCst, Relaxed);
    }
}

impl Clone for SizeLinkRef {
    #[inline]
    fn clone(&self) -> SizeLinkRef {
        unsafe {
            self.extent().retain();
            SizeLinkRef::from_raw(self.ptr)
        }
    }
}

impl Drop for SizeLinkRef {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            self.extent().release();
        }
    }
}
