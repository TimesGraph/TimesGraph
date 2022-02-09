use crate::block::{Block, Layout, LayoutError};

#[allow(improper_ctypes)]
extern "Rust" {
    #[no_mangle]
    fn _tg_global_heap<'a>() -> &'a dyn Heap<'a>;
}

/// Allocator for large memory blocks.
pub trait Heap<'a> {
    /// Allocates a new memory block sized and aligned to at least `Layout`;
    /// returns `None` if the allocation fails.
    unsafe fn alloc(&self, layout: Layout) -> Result<Block<'a>, HeapError>;

    /// Deallocates a memory block previously allocated by `alloc`.
    /// Returns the number of freed bytes.
    unsafe fn dealloc(&self, block: Block<'a>) -> usize;
}

impl<'a> Heap<'a> {
    /// Returns a handle to a the global `Heap` allocator.
    #[inline]
    pub fn global() -> &'a dyn Heap<'a> {
        unsafe { _swim_global_heap() }
    }
}

/// Heap memory allocation error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HeapError {
    /// Improper structure alignment.
    Misaligned,
    /// Structure size overflow.
    Oversized,
    /// Insufficient available memory.
    OutOfMemory,
    /// Unsupported operation; will never succeed.
    Unsupported(&'static str),
}

impl From<LayoutError> for HeapError {
    #[inline]
    fn from(error: LayoutError) -> HeapError {
        match error {
            LayoutError::Misaligned => HeapError::Misaligned,
            LayoutError::Oversized => HeapError::Oversized,
        }
    }
}
