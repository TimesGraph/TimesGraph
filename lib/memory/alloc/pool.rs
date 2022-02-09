use core::mem;
use core::ptr;
use core::sync::atomic::{AtomicPtr, AtomicUsize};
use core::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use tg_core::reify::{Reified, Reify};
use crate::block::{Block, Layout};
use crate::alloc::{Heap, Hold, HoldError};
use crate::alloc::pack::PackBase;

/// Linear allocator for a dynamically growable set of memory blocks.
///
/// A `Pool` allocates space in a sequence of linear memory `Pack`s allocated
/// by a `Heap`. Pools only reclaim space when the most recent allocation drops,
/// and when the whole pool drops. To free a pool, use `IntoHold` to move live
/// values to another hold, then drop the pool.
pub struct Pool<'a> {
    /// Heap used to allocate memory for the pack list.
    heap: &'a Heap<'a>,
    /// Pointer to the first pack in the used pack list.
    head: AtomicPtr<PackList<'a>>,
    /// Number of reserved bytes in the pool.
    size: AtomicUsize,
    /// Number of live allocations in the pool.
    live: AtomicUsize,
    /// Number of currently allocated bytes in the pool.
    used: AtomicUsize,
}

impl<'a> Pool<'a> {
    /// Returns a new `Pool` that allocates memory from the given `heap`.
    #[inline]
    pub fn new(heap: &'a Heap<'a>) -> Pool<'a> {
        Pool {
            heap: heap,
            head: AtomicPtr::new(ptr::null_mut()),
            size: AtomicUsize::new(0),
            live: AtomicUsize::new(0),
            used: AtomicUsize::new(0),
        }
    }

    /// Returns the `Heap` used to allocate memory for the internal pack list.
    #[inline]
    pub fn heap(&self) -> &'a Heap<'a> {
        self.heap
    }

    /// Returns the number of reserved bytes in this `Pool`.
    #[inline]
    pub fn size(&self) -> usize {
        self.size.load(Relaxed)
    }

    /// Return the number of live allocations in this `Pool`.
    #[inline]
    pub fn live(&self) -> usize {
        self.live.load(Relaxed)
    }

    /// Returns the number of bytes currently allocated in this `Pool`.
    #[inline]
    pub fn used(&self) -> usize {
        self.used.load(Relaxed)
    }

    /// Acquires a new pack list item from this pool's `Heap`.
    fn alloc_pack(&self, layout: Layout) -> Result<*mut PackList<'a>, HoldError> {
        unsafe {
            // Make room for the pack list header.
            let layout = Layout::for_type::<PackList<'a>>().extended(layout)?.0;
            // Allocate a new memory block, bailing on failure.
            let block = self.heap.alloc(layout)?;
            // Capture the size of the allocated memory block.
            let size = block.size();
            // Get a mutable pointer to this pool to give to the pack list item.
            let pool = self as *const Pool<'a> as *mut Pool<'a>;
            // Construct a pack list item in the new memory block.
            let pack = PackList::from_block(block, pool);

            // Increase the pool size.
            self.size.fetch_add(size, Relaxed);

            // Return a pointer to the new pack list item.
            Ok(pack)
        }
    }

    /// Releases a pack list item back to this pool's `Heap`.
    unsafe fn dealloc_pack(&self, pack: *mut PackList<'a>) {
        // Get the pack's memory block.
        let block = (*pack).as_block();

        // Decrease the pool size.
        self.size.fetch_sub(block.size(), Relaxed);

        // Deallocate the memory block.
        self.heap.dealloc(block);
    }

    /// Accounts for the allocation of a `size` byte block.
    unsafe fn did_alloc(&self, size: usize) {
        // Increment the live allocation count.
        self.live.fetch_add(1, Relaxed);
        // Increase the allocated byte count.
        self.used.fetch_add(size, Relaxed);
    }

    /// Accounts for the deallocation of a `size` byte block.
    unsafe fn did_dealloc(&self, size: usize) {
        // Decrement the live allocation count.
        self.live.fetch_sub(1, Relaxed);
        // Decrease the allocated byte count.
        self.used.fetch_sub(size, Relaxed);
    }

    /// Accounts for the resizing of a block from `old_size` to `new_size` bytes.
    unsafe fn did_resize(&self, old_size: usize, new_size: usize) {
        let size_diff = (new_size as isize).wrapping_sub(old_size as isize);
        // Adjust the allocated byte count by the size difference.
        if size_diff > 0 {
            self.used.fetch_add(size_diff as usize, Relaxed);
        } else if size_diff < 0 {
            self.used.fetch_sub(-size_diff as usize, Relaxed);
        }
    }
}

unsafe impl<'a> Hold<'a> for Pool<'a> {
    unsafe fn alloc(&self, layout: Layout) -> Result<Block<'a>, HoldError> {
        // Allocated block in the proposed new head of the pack list.
        let mut block = None;
        // Proposed new head of the pack list.
        let mut head: *mut PackList<'a> = ptr::null_mut();
        // Load the current head of the pack list.
        let mut next = self.head.load(Relaxed);
        loop {
            // Try to allocate a block in the next pack in the list.
            if !next.is_null() {
                // Next pack exists; try the allocation.
                if let Ok(block) = (*next).base.alloc(layout) {
                    // Successfully allocated new block.
                    // Check if we previously poposed a new head pack.
                    if !head.is_null() {
                        // Free it if we did.
                        self.dealloc_pack(&mut *head);
                    }
                    // Account for the allocation.
                    self.did_alloc(block.size());
                    // Return the new block.
                    return Ok(block);
                }
            }

            // Failed to allocate a new block in the current head of the pack list.
            // Propose a new head pack containing a pre-allocated block.
            if head.is_null() {
                // Try to allocate a proposed new head pack.
                if let Ok(pack) = self.alloc_pack(layout) {
                    // Pack allocation succeeded.
                    // Try to pre-allocate a block in the new pack.
                    if let Ok(new_block) = (*pack).base.alloc(layout) {
                        // Block allocation succeeded.
                        // Save the new pack reference in case the head CAS fails.
                        head = pack;
                        // Save the block reference in case the head CAS fails.
                        block = Some(new_block);
                    } else {
                        // Failed to pre-allocate a block in the new pack.
                        // Free the pack.
                        self.dealloc_pack(pack);
                        // And give up.
                        return Err(HoldError::OutOfMemory);
                    }
                } else {
                    // Failed to allocate a new pack. Give up.
                    return Err(HoldError::OutOfMemory);
                }
            }

            // Set the tail of the proposed new head pack to the current pack list.
            (*head).next.store(next, Relaxed);
            // Compare and swap the current pack list for the new pack list.
            // Synchronize to prevent head dealloc from reordering before this.
            let commit = self.head.compare_exchange_weak(next, head, Acquire, Relaxed);

            // Check if the CAS failed.
            if let Err(pack) = commit {
                // Set the next pointer to the new head of the pack list and try again.
                next = pack;
                continue;
            }

            // Successfully updated the pack list.
            let block = block.unwrap();
            // Account for the allocation.
            self.did_alloc(block.size());
            // Return the preallocated block.
            return Ok(block);
        }
    }

    unsafe fn dealloc(&self, _block: Block<'a>) -> usize {
        // Never directly deallocates anything; deallocations always dispatch
        // to the allocating pack list item.
        unimplemented!();
    }

    unsafe fn resize(&self, _block: Block<'a>, _layout: Layout) -> Result<Block<'a>, HoldError> {
        // Never directly resizes anything; reallocations always dispatch to
        // allocating pack list item.
        unimplemented!();
    }
}

impl<'a> Drop for Pool<'a> {
    fn drop(&mut self) {
        if self.live.load(Relaxed) != 0 {
            panic!("leaky pool");
        }
        // Load the current head of the pack list.
        let mut next = self.head.load(Relaxed);
        loop {
            // Check for the end of the pack list.
            if next.is_null() {
                // All done.
                return;
            }
            unsafe {
                // Load the tail of the pack list.
                let tail = (*next).next.load(Relaxed);
                // Compare and swap the current pack list for the tail of the pack list.
                match self.head.compare_exchange_weak(next, tail, Release, Relaxed) {
                    Ok(pack) => { // CAS succeeded.
                        // Deallocate the head pack.
                        self.dealloc_pack(&mut *next);
                        // Set the next pointer to the tail of the pack list and continue.
                        next = pack;
                        continue;
                    },
                    Err(pack) => { // CAS failed.
                        // Set the next pointer to the new head of the pack list and try again.
                        next = pack;
                        continue;
                    },
                }
            }
        }
    }
}

struct PackList<'a> {
    /// Inner pack allocator.
    base: PackBase<'a>,
    /// Pointer to the next pack in the used pack list.
    next: AtomicPtr<PackList<'a>>,
    /// Non-zero pointer to the pool that owns this pack.
    pool: *mut Pool<'a>,
}

impl<'a> PackList<'a> {
    // Constructs a `PackList` in a memory `block`.
    unsafe fn from_block(block: Block<'a>, pool: *mut Pool<'a>) -> *mut PackList<'a> {
        // Construct a base pack in the memory block with a pack list header.
        let pack = PackBase::from_block(block, mem::size_of::<PackList<'a>>()) as *mut PackList<'a>;
        // Initialize the next pointer.
        ptr::write(&mut (*pack).next, AtomicPtr::new(ptr::null_mut()));
        // Initialize the pool pointer.
        ptr::write(&mut (*pack).pool, pool);
        // Initialize the hold base with the concrete type of the pack list.
        PackList::deify(&mut *pack);
        // Return a pointer to the pack list header.
        pack
    }

    /// Returns the memory block managed by this `PackList` item.
    #[inline]
    unsafe fn as_block(&mut self) -> Block<'a> {
        self.base.as_block()
    }
}

unsafe impl<'a> Hold<'a> for PackList<'a> {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> Result<Block<'a>, HoldError> {
        // Delegate allocation back to the pool.
        (*self.pool).alloc(layout)
    }

    #[inline]
    unsafe fn dealloc(&self, block: Block<'a>) -> usize {
        // Delegate deallocation to the base pack.
        let size = self.base.dealloc(block);
        // Inform the pool of the deallocation.
        (*self.pool).did_dealloc(size);
        // Return the number of freed bytes.
        size
    }

    #[inline]
    unsafe fn resize(&self, block: Block<'a>, layout: Layout) -> Result<Block<'a>, HoldError> {
        // Get the size of the current block.
        let old_size = block.size();
        // Delegate resizing to the base pack.
        match self.base.resize(block, layout) {
            Ok(block) => {
                // Get the size of the resized block.
                let new_size = block.size();
                // Inform the pool of the resize.
                (*self.pool).did_resize(old_size, new_size);
                // Return the resized block.
                Ok(block)
            },
            err @ Err(_) => err,
        }
    }
}

impl<'a> Reify<'a, Hold<'a> + 'a> for PackList<'a> {
    #[inline]
    unsafe fn deify(object: &mut (Hold<'a> + 'a)) {
        Reified::<Hold<'a>>::deify(mem::transmute(object));
    }

    #[inline]
    unsafe fn reify(base: &'a Reified<Hold<'a> + 'a>) -> &'a (Hold<'a> + 'a) {
        mem::transmute(base.reify())
    }
}
