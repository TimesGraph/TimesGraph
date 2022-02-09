use core::marker::PhantomData;
use core::mem;
use core::ptr;
use core::sync::atomic::{AtomicPtr, AtomicU32};
use core::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use core::u32;
use crate::block::{Block, Layout};
use crate::alloc::{Heap, HeapError};

/// Allocator for a hunk of memory partitioned into fixed size memory blocks.
pub struct Slab<'a> {
    /// Hunk of memory to partition into `unit` size blocks.
    hunk: Block<'a>,
    /// Number of bytes in each memory block.
    unit: u32,
    /// Number of currently allocated memory blocks.
    live: AtomicU32,
    /// Pointer to the first block in the free block list.
    head: AtomicPtr<FreeList>,
    /// Variant over 'a.
    hunk_marker: PhantomData<&'a ()>,
}

impl<'a> Slab<'a> {
    /// Returns a new `Slab` that allocates a hunk of memory in `unit`-sized blocks.
    #[inline]
    pub fn new(hunk: Block<'a>, unit: usize) -> Slab<'a> {
        if unit < mem::size_of::<FreeList>() {
            panic!("unit too small");
        }
        if unit > u32::MAX as usize {
            panic!("unit too large");
        }
        // Initialize the head of the free list to nil.
        let mut head = ptr::null_mut();
        // Compute the number of blocks that can fit in the hunk.
        let block_count = hunk.size() / unit;
        // Check if at least one block fits in the hunk.
        if block_count != 0 {
            // Get the base address of the hunk.
            let base = hunk.as_ptr() as usize;
            // Start with the last block in the hunk.
            let mut next = base.wrapping_add(unit.wrapping_mul(block_count.wrapping_sub(1)));
            // Loop over all blocks in the hunk, back to front.
            loop {
                // Interpret the next block as the current tail of the free list.
                let tail = next as *mut FreeList;
                // Set the next pointer of the current tail to the current head of the free list.
                unsafe { ptr::write(&mut (*tail).next, AtomicPtr::new(head)); }
                // Make the current tail the new head of the free list.
                head = tail;
                // Break if the next block is the first block in the hunk.
                if next == base {
                    break;
                }
                // Set the next block to the previous block in the hunk.
                next = next.wrapping_sub(unit);
            }
        }
        Slab {
            hunk: hunk,
            unit: unit as u32,
            live: AtomicU32::new(0),
            head: AtomicPtr::new(head),
            hunk_marker: PhantomData,
        }
    }

    /// Returns the total number of bytes in this `Slab`.
    #[inline]
    pub fn size(&self) -> usize {
        self.hunk.size()
    }

    /// Returns the number of bytes in each memory block of this `Slab`.
    #[inline]
    pub fn block_size(&self) -> usize {
        self.unit as usize
    }

    /// Returns the total number of blocks in this `Slab`.
    #[inline]
    pub fn block_count(&self) -> usize {
        self.hunk.size() / self.unit as usize
    }

    /// Returns the number of currently allocated memory blocks.
    #[inline]
    pub fn live(&self) -> usize {
        self.live.load(Relaxed) as usize
    }

    /// Returns the number of memory blocks currently available for allocation.
    #[inline]
    pub fn dead(&self) -> usize {
        self.block_count() - self.live()
    }

    /// Consumes this `Slab` and returns its hunk of memory.
    #[inline]
    pub fn into_block(self) -> Block<'a> {
        if self.live.load(Relaxed) != 0 {
            panic!("leaky slab");
        }
        self.hunk
    }
}

impl<'a> Heap<'a> for Slab<'a> {
    unsafe fn alloc(&self, layout: Layout) -> Result<Block<'a>, HeapError> {
        // Check if the layout will fit in a block.
        if layout.size() > self.unit as usize {
            return Err(HeapError::Oversized);
        }
        // Load the current head of the free block list.
        let mut head = self.head.load(Relaxed);
        loop {
            // Check for the end of the free block list.
            if head.is_null() {
                // No free blocks.
                return Err(HeapError::OutOfMemory);
            }
            // Check if the free block has appropriate alignment.
            if head as usize % layout.align() != 0 {
                // Misaligned.
                return Err(HeapError::Misaligned);
            }
            // Load the tail of the free block list.
            let tail = (*head).next.load(Relaxed);
            // Compare and swap the current free list for the tail of the free list.
            match self.head.compare_exchange_weak(head, tail, Release, Relaxed) {
                Ok(block) => { // CAS succeeded.
                    // Increment the live block count.
                    self.live.fetch_add(1, Relaxed);
                    // Return the free block.
                    return Ok(Block::from_raw_parts(block as *mut u8, self.unit as usize));
                },
                Err(block) => { // CAS failed.
                    // Set the head pointer to the new head of the free list and try again.
                    head = block;
                    continue;
                },
            };
        }
    }

    unsafe fn dealloc(&self, block: Block<'a>) -> usize {
        let size = block.size();
        // Interpret the memory block as the new head of free block list.
        let head = block.as_ptr() as *mut FreeList;
        // Load the current head of the free block list.
        let mut tail = self.head.load(Relaxed);
        loop {
            // Set the next pointer of the new head block to the current free block list.
            ptr::write(&mut (*head).next, AtomicPtr::new(tail));
            // Compare and swap the current free list for the new free list.
            match self.head.compare_exchange_weak(tail, head, Acquire, Relaxed) {
                Ok(_) => { // CAS succeeded.
                    // Decrement the live block count.
                    self.live.fetch_sub(1, Relaxed);
                    // Finished.
                    return size;
                },
                Err(block) => { // CAS failed.
                    // Set the tail pointer to the new head of free block list and try again.
                    tail = block;
                    continue;
                },
            };
        }
    }
}

#[repr(C)]
struct FreeList {
    /// Pointer to the next block in the free block list
    next: AtomicPtr<FreeList>,
}
