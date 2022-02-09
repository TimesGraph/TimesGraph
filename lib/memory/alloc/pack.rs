use core::cmp;
use core::marker::PhantomPinned;
use core::mem;
use core::ptr;
use core::sync::atomic::AtomicU32;
use core::sync::atomic::Ordering::{Acquire, Relaxed, Release, SeqCst};
use core::u32;
use tg_core::reify::{Reified, Reify};
use crate::block::{Block, Layout};
use crate::alloc::{AllocTag, Hold, HoldError};

/// Base linear allocator for a fixed-size memory block.
///
/// A pack allocates space by advancing a pointer into its memory block,
/// similar to stack allocation. Packs only reclaim space when the most
/// recent allocation drops, and when the whole pack drops.
pub(crate) struct PackBase<'a> {
    /// Polymorphic hold type.
    base: Reified<Hold<'a>>,
    /// Total number of bytes in the memory block, including pack header.
    size: u32,
    /// Offset from the base pack address of the next free byte in the memory block.
    mark: AtomicU32,
    /// Pin to the base address of the memory block.
    #[allow(dead_code)]
    pinned: PhantomPinned,
}

impl<'a> PackBase<'a> {
    /// Constructs a `PackBase` in the memory `block`, with a reserved header.
    ///
    /// # Safety
    ///
    /// Assumes `header_size` is large enough to hold the `PackBase` header.
    pub(crate) fn from_block(block: Block<'a>, header_size: usize) -> *mut PackBase<'a> {
        // Get the alignment of the allocation tag.
        let tag_align = mem::align_of::<AllocTag>();
        // Round the header size up to the alignment of the first allocation tag.
        let header_size = header_size.wrapping_add(tag_align).wrapping_sub(1) & !tag_align.wrapping_sub(1);
        // Get the size of the memory block.
        let block_size = block.size();
        if block_size < header_size {
            panic!("block too small");
        }
        if block_size > u32::MAX as usize {
            panic!("block too large");
        }
        // Get the address of the block slice.
        let pack_ptr = block.as_ptr() as *mut PackBase<'a>;
        unsafe {
            // Initialize the base pack header to the beginning of the memory block.
            ptr::write(pack_ptr, PackBase {
                base: Reified::uninitialized(),
                size: block_size as u32,
                mark: AtomicU32::new(header_size as u32),
                pinned: PhantomPinned,
            });
            // Return a pointer to the base pack header.
            pack_ptr
        }
    }

    /// Returns the total number of bytes in the memory block, including the pack header.
    #[inline]
    pub(crate) fn size(&self) -> usize {
        self.size as usize
    }

    /// Returns the number of free bytes available for allocation in this pack.
    #[inline]
    pub(crate) fn free(&self) -> usize {
        self.size.wrapping_sub(self.mark.load(Relaxed)) as usize
    }

    /// Returns the memory block managed by this `PackBase`.
    #[inline]
    pub(crate) unsafe fn as_block(&mut self) -> Block<'a> {
        let data = self as *mut PackBase<'a> as *mut u8;
        let size = self.size as usize;
        Block::from_raw_parts(data, size)
    }

    #[inline]
    pub(crate) unsafe fn alloc(&self, layout: Layout) -> Result<Block<'a>, HoldError> {
        // Get the alignment of the allocation tag.
        let tag_align = mem::align_of::<AllocTag>();
        // Get the size of the allocation tag.
        let tag_size = mem::size_of::<AllocTag>();
        // Align the block for the preceding allocation tag.
        let align = cmp::max(layout.align(), tag_align);
        // Round the block size up to the alignment of the next allocation tag.
        let size = layout.size().wrapping_add(tag_align).wrapping_sub(1) & !tag_align.wrapping_sub(1);
        // Get the base address of the memory block.
        let base_addr = self as *const PackBase<'a> as usize;

        // Load the current mark offset.
        let mut old_mark = self.mark.load(Relaxed);
        loop {
            // Compute the block start address by adding the mark offset to the base address.
            let start_addr = base_addr.wrapping_add(old_mark as usize);
            // Make room for the allocation tag that will directly precede the allocated block.
            let block_addr = start_addr.wrapping_add(tag_size);
            // Round up to the alignment required by the block.
            let block_addr = block_addr.wrapping_add(align).wrapping_sub(1) & !align.wrapping_sub(1);

            // Compute the end address of the block; bail on overflow.
            let end_addr = match block_addr.checked_add(size) {
                Some(addr) => addr,
                None => return Err(HoldError::OutOfMemory),
            };
            // Compute the new mark offset by subtracting the base pointer.
            let new_mark = end_addr.wrapping_sub(base_addr);

            // Bail if the block would overflow the memory block.
            if new_mark > self.size as usize {
                return Err(HoldError::OutOfMemory);
            }

            // Attempt to allocate the block by atomically swapping the mark offset.
            // Synchronize to prevent dealloc from reordering before this.
            let commit = self.mark.compare_exchange_weak(old_mark, new_mark as u32, Acquire, Relaxed);

            // Check if the CAS failed.
            if let Err(mark) = commit {
                // Set the mark offset to the new mark and try again.
                old_mark = mark;
                continue;
            }

            // Subtract the tag size from the block address.
            let tag_addr = block_addr.wrapping_sub(mem::size_of::<AllocTag>()) as *mut AllocTag<'a>;
            // Initialize the allocation tag.
            ptr::write(tag_addr, AllocTag::new(&self.base));

            // Return the allocated block.
            return Ok(Block::from_raw_parts(block_addr as *mut u8, size))
        }
    }

    #[inline]
    pub(crate) unsafe fn dealloc(&self, block: Block<'a>) -> usize {
        // Get the alignment of the allocation tag.
        let tag_align = mem::align_of::<AllocTag>();
        // Get the size of the allocation tag.
        let tag_size = mem::size_of::<AllocTag>();
        // Compute the size of the allocated block.
        let size = block.size().wrapping_add(tag_align).wrapping_sub(1) & !tag_align.wrapping_sub(1);

        // Check if the block has non-zero size.
        if size != 0 {
            // Get the base address of the memory block.
            let base_addr = self as *const PackBase<'a> as usize;

            // Compute the end address of the block.
            let end_addr = (block.as_ptr() as usize).wrapping_add(size);
            // Compute the address of the allocation tag.
            let header_addr = (block.as_ptr() as usize).wrapping_sub(tag_size);

            // Compute the offset of the end address by subtracting the base address.
            let old_mark = end_addr.wrapping_sub(base_addr) as u32;
            // Compute the offset of the tag address by subtracting the base address.
            let new_mark = header_addr.wrapping_sub(base_addr) as u32;

            // Rewind the mark offset if it still points to the end of the block,
            // i.e. pop the stack, if we can. Synchronize to prevent alloc from
            // reordering after this.
            let _ = self.mark.compare_exchange(old_mark, new_mark, Release, Relaxed);
        }

        // Return the number of freed bytes.
        size
    }

    #[inline]
    pub(crate) unsafe fn resize(&self, block: Block<'a>, layout: Layout) -> Result<Block<'a>, HoldError> {
        let tag_align = mem::align_of::<AllocTag>();
        // Get the address of the current block.
        let block_addr = block.as_ptr() as usize;
        // Check if the alignment of the current block is suitable for the proposed new block.
        if block_addr % layout.align() != 0 {
            // Misaligned.
            return Err(HoldError::Misaligned);
        }

        // Round the old block size up to the alignment of the next allocation tag.
        let old_size = block.size().wrapping_add(tag_align).wrapping_sub(1) & !tag_align.wrapping_sub(1);
        // Check if the old block has zero size.
        if old_size == 0 {
            // Can't resize zero size blocks.
            return Err(HoldError::Unsupported("resize from zero"));
        }
        // Round the new block size up to the alignment of the next allocation tag.
        let new_size = layout.size().wrapping_add(tag_align).wrapping_sub(1) & !tag_align.wrapping_sub(1);
        // Check if the new block has zero size.
        if new_size == 0 {
            // Can't resize to zero.
            return Err(HoldError::Unsupported("resize to zero"));
        }
        // Compute the size difference between the current block and the proposed new block.
        let size_diff = (new_size as isize).wrapping_sub(old_size as isize);
        // Get the base address of the memory block.
        let base_addr = self as *const PackBase<'a> as usize;

        // Compute the end address of the old block.
        let old_end_addr = block_addr.wrapping_add(old_size);
        // Compute the end offset of the current end address by subtracting the base address.
        let old_end_mark = old_end_addr.wrapping_sub(base_addr) as u32;
        // Compute the end address of the proposed new block; bail on under/overflow.
        let new_end_addr = if size_diff != 0 {
            match block_addr.checked_add(new_size) {
                Some(addr) => addr,
                None => return Err(HoldError::Oversized),
            }
        } else {
            // Unchanged size. Return the original block.
            return Ok(block);
        };
        // Compute the end offset of the proposed new block by subtracting the base address.
        let new_end_mark = new_end_addr.wrapping_sub(base_addr) as u32;

        // Move the mark offset if it still points to the end of the current block.
        // Synchronize to prevent alloc and dealloc from relative to this.
        let commit = self.mark.compare_exchange(old_end_mark, new_end_mark, SeqCst, Relaxed);

        // Check if the CAS failed.
        if let Err(_) = commit {
            // Can only resize the most recently allocated block.
            return Err(HoldError::Oversized);
        }

        // Return the resized block.
        return Ok(Block::from_raw_parts(block.as_ptr(), new_size))
    }
}

impl<'a> Reify<'a, Hold<'a> + 'a> for PackBase<'a> {
    #[inline]
    unsafe fn deify(object: &mut (Hold<'a> + 'a)) {
        Reified::<Hold<'a>>::deify(mem::transmute(object));
    }

    #[inline]
    unsafe fn reify(base: &'a Reified<Hold<'a> + 'a>) -> &'a (Hold<'a> + 'a) {
        mem::transmute(base.reify())
    }
}

/// Linear allocator for a fixed-size memory block.
///
/// A pack allocates space by advancing a pointer into its memory block,
/// similar to stack allocation. Packs only reclaim space when the most
/// recent allocation drops, and when the whole pack drops.
pub struct Pack<'a> {
    /// Inner pack allocator.
    base: PackBase<'a>,
    /// Number of live allocations in this pack.
    live: AtomicU32,
    /// Number of currently allocated bytes in this pack.
    used: AtomicU32,
    /// Tag shared by all zero-sized allocations in this pack.
    zero: AllocTag<'a>,
}

impl<'a> Pack<'a> {
    /// Constructs a `Pack` in a memory `block`.
    pub fn new(block: Block<'a>) -> &'a Pack<'a> {
        unsafe { &*Pack::from_block(block, mem::size_of::<Pack<'a>>()) }
    }

    /// Constructs a `Pack` in a memory `block` with a reserved header.
    ///
    /// # Safety
    ///
    /// Assumes `header_size` is large enough to hold the `Pack` header.
    pub fn from_block(block: Block<'a>, header_size: usize) -> *mut Pack<'a> {
        // Construct a base pack in the memory block with a pack header.
        let pack = PackBase::from_block(block, header_size) as *mut Pack<'a>;
        unsafe {
            // Initialize the live allocation count.
            ptr::write(&mut (*pack).live, AtomicU32::new(0));
            // Initialize the allocated byte count.
            ptr::write(&mut (*pack).used, AtomicU32::new(0));
            // Initialize the zero-sized allocation tag.
            ptr::write(&mut (*pack).zero, AllocTag::new(&(*pack).base.base));
            // Initialize the hold base with the concrete type of the pack.
            Pack::deify(&mut *pack);
        }
        // Return a pointer to the pack header.
        pack
    }

    /// Returns the zero-sized block for this `Pack`.
    #[inline]
    fn empty(&self) -> Block<'a> {
        // Get the address of the zero-sized allocation tag.
        let tag_addr = &self.zero as *const AllocTag<'a> as usize;
        // Get the address of the zero-sized block immediately following the tag.
        let zero_addr = tag_addr.wrapping_add(mem::size_of::<AllocTag<'a>>());
        // Return the zero-sized block for this pack.
        unsafe { Block::from_raw_parts(zero_addr as *mut u8, 0) }
    }

    /// Returns the total number of bytes in the memory block, including the pack header.
    #[inline]
    pub fn size(&self) -> usize {
        self.base.size()
    }

    /// Returns the number of free bytes available for allocation in this `Pack`.
    #[inline]
    pub fn free(&self) -> usize {
        self.base.free()
    }

    /// Return the number of live allocations in this `Pack`.
    #[inline]
    pub fn live(&self) -> usize {
        self.live.load(Relaxed) as usize
    }

    /// Returns the number of bytes currently allocated in this `Pack`.
    #[inline]
    pub fn used(&self) -> usize {
        self.used.load(Relaxed) as usize
    }

    /// Returns the memory block managed by this `Pack`.
    #[inline]
    pub unsafe fn as_block(&mut self) -> Block<'a> {
        self.base.as_block()
    }
}

unsafe impl<'a> Hold<'a> for Pack<'a> {
    unsafe fn alloc(&self, layout: Layout) -> Result<Block<'a>, HoldError> {
        // Check if the layout represents a zero-sized type.
        if layout.size() == 0 {
            // Increment the live allocation count.
            self.live.fetch_add(1, Relaxed);
            // Return the empty block.
            return Ok(self.empty());
        }
        // Delegate allocation to the base pack.
        let block = self.base.alloc(layout)?;
        // Increment the live allocation count.
        self.live.fetch_add(1, Relaxed);
        // Increase the allocated byte count.
        self.used.fetch_add(block.size() as u32, Relaxed);
        // Return the allocated block.
        Ok(block)
    }

    unsafe fn dealloc(&self, block: Block<'a>) -> usize {
        // Delegate deallocation to the base pack.
        let size = self.base.dealloc(block);
        // Decrease the allocated byte count.
        self.used.fetch_sub(size as u32, Relaxed);
        // Decrement the live allocation count.
        self.live.fetch_sub(1, Relaxed);
        // Return the number of freed bytes.
        size
    }

    unsafe fn resize(&self, block: Block<'a>, layout: Layout) -> Result<Block<'a>, HoldError> {
        // Get the size of the current block.
        let old_size = block.size();
        // Delegate resizing to the base pack.
        match self.base.resize(block, layout) {
            Ok(block) => {
                // Get the size of the resized block;
                let new_size = block.size();
                // Compute the size difference between the original block and the resized block.
                let size_diff = (new_size as isize).wrapping_sub(old_size as isize);
                // Adjust the allocated byte count by the size difference.
                if size_diff > 0 {
                    self.used.fetch_add(size_diff as u32, Relaxed);
                } else if size_diff < 0 {
                    self.used.fetch_sub(-size_diff as u32, Relaxed);
                }
                // Return the resized block.
                Ok(block)
            },
            err @ Err(_) => err,
        }
    }
}

impl<'a> Reify<'a, Hold<'a> + 'a> for Pack<'a> {
    #[inline]
    unsafe fn deify(object: &mut (Hold<'a> + 'a)) {
        Reified::<Hold<'a>>::deify(mem::transmute(object));
    }

    #[inline]
    unsafe fn reify(base: &'a Reified<Hold<'a> + 'a>) -> &'a (Hold<'a> + 'a) {
        mem::transmute(base.reify())
    }
}
