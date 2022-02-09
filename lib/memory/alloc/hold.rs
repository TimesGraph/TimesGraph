use core::cell::UnsafeCell;
use core::mem;
use core::ptr;
use tg_core::reify::{Reified, Reify};
use crate::block::{Block, Layout, LayoutError};
use crate::alloc::{AllocTag, HeapError};

#[allow(improper_ctypes)]
extern "Rust" {
    #[no_mangle]
    fn _tg_global_hold<'a>() -> &'a dyn Hold<'a>;
}

/// Limited lifetime memory allocator.
///
/// # Safety
///
/// Every `Hold` implementation that directly allocates memory blocks *must*
/// have a valid `Reified` vtable as its first struct member. The `Reified`
/// vtable must point to the vtable of the concrete `Hold` implementation.
///
/// Each memory block allocated by a `Hold` must have an `AllocTag` in the
/// bytes immediately preceding the block. The tag must contain a valid thin
/// pointer to a `Reified` struct that, when reified to a `Hold` trait object,
/// can deallocate the tagged block.
pub unsafe trait Hold<'a> {
    /// Returns an unmanaged pointer to an uninitialized memory block sized
    /// and aligned to `layout`; returns  an `Err` if the allocation fails.
    /// The allocated block will have a valid `AllocTag` in the bytes
    /// immediately preceding the block.
    unsafe fn alloc(&self, layout: Layout) -> Result<Block<'a>, HoldError>;

    /// Releases a memory `block` allocated by this `Hold`.
    /// Returns the number of freed bytes.
    unsafe fn dealloc(&self, block: Block<'a>) -> usize;

    /// Attempts to resize in place a memory `block` allocated by this `Hold`
    /// to fit a new `layout`. Returns `Ok` with the resized memory block on
    /// success; returns a `HoldError` on failure.
    unsafe fn resize(&self, block: Block<'a>, layout: Layout) -> Result<Block<'a>, HoldError>;

    /// Attempts to resize a memory `block` allocated by this `Hold` to fit
    /// a new `layout`. Returns `Ok` with the resized memory block on success;
    /// returns a `HoldError` on failure.
    unsafe fn realloc(&self, block: Block<'a>, layout: Layout) -> Result<Block<'a>, HoldError> {
        match self.resize(block, layout) {
            ok @ Ok(_) => ok,
            Err(_) => match self.alloc(layout) {
                Ok(new_block) => {
                    self.dealloc(block);
                    Ok(new_block)
                },
                err @ Err(_) => err,
            },
        }
    }
}

impl<'a> Hold<'a> {
    /// Returns a handle to a the global `Hold` allocator.
    pub fn global() -> &'a dyn Hold<'a> {
        unsafe { _tg_global_hold() }
    }

    /// Returns a handle to a the thread-local `Hold` allocator.
    ///
    /// # Safety
    ///
    /// Unsafe because the returned reference's lifetime does not necessarily
    /// match the lifetime of the current thread local `Hold`. Callers must
    /// take care not to acquire a local `Hold` reference with a lifetime that
    /// exceeds the current thread local `Hold`.
    ///
    /// # Panics
    ///
    /// Panics if there is no current thread local `Hold`.
    pub unsafe fn local() -> &'a dyn Hold<'a> {
        match LocalHold::get() {
            Some(scope) => mem::transmute::<&'static dyn Hold<'static>, &'a dyn Hold<'a>>(scope.hold),
            None => panic!("no local Hold"),
        }
    }

    /// Returns a reference to a `Hold` that can only allocate zero-sized values.
    pub fn empty() -> &'a impl Hold<'a> {
        // Declare the empty hold singleton with an uninitialized vtable.
        // Rust apparently has no way to get a static pointer to a vtable.
        static mut EMPTY: EmptyHold<'static> = EmptyHold {
            base: unsafe { Reified::uninitialized() },
            zero: unsafe { AllocTag::null() },
        };
        unsafe {
            // Initialize the empty hold's vtable (idempotent).
            EmptyHold::deify(&mut EMPTY);
            // Initialize the empty hold's shared zero tag (idempotent).
            EMPTY.zero.init(&EMPTY.base);
            // Return a reference to the empty hold.
            mem::transmute::<_, &EmptyHold<'a>>(&EMPTY)
        }
    }
}

/// Linked list of `Hold` references representing a stack of memory allocation
/// contexts.
pub struct HoldScope<'a> {
    hold: &'a dyn Hold<'a>,
    next: Option<&'a HoldScope<'a>>,
}

impl<'a> HoldScope<'a> {
    pub fn new(hold: &'a dyn Hold<'a>) -> HoldScope<'a> {
        HoldScope {
            hold: hold,
            next: None,
        }
    }
}

unsafe impl<'a> Hold<'a> for HoldScope<'a> {
    unsafe fn alloc(&self, layout: Layout) -> Result<Block<'a>, HoldError> {
        self.hold.alloc(layout)
    }

    unsafe fn dealloc(&self, _block: Block<'a>) -> usize {
        // Never directly deallocates anything; deallocations always dispatch
        // to the underlying hold.
        unimplemented!();
    }

    unsafe fn resize(&self, _block: Block<'a>, _layout: Layout) -> Result<Block<'a>, HoldError> {
        // Never directly resizes anything; reallocations always dispatch to
        // underlying hold.
        unimplemented!();
    }
}

#[thread_local]
static LOCAL_HOLD_SCOPE: UnsafeCell<Option<&'static HoldScope<'static>>> = UnsafeCell::new(None);

#[macro_export]
macro_rules! hold_scope {
    // local hold $name = $hold;
    ($(#[$attr:meta])* local hold $name:ident = $hold:expr;) => (
        let mut __scope = $crate::alloc::HoldScope::new($hold);
        #[allow(unused_variables)]
        $(#[$attr])* let $name = $crate::alloc::LocalHold::enter(&mut __scope);
    );
}

/// RAII `HoldScope` frame representing an entry in the thread local `Hold`
/// memory allocator stack.
pub struct LocalHold<'a> {
    scope: &'a HoldScope<'a>,
}

impl<'a> !Send for LocalHold<'a> {
}

impl<'a> !Sync for LocalHold<'a> {
}

impl LocalHold<'static> {
    unsafe fn get() -> Option<&'static HoldScope<'static>> {
        *LOCAL_HOLD_SCOPE.get()
    }

    unsafe fn set(scope: Option<&'static HoldScope<'static>>) {
        ptr::write(LOCAL_HOLD_SCOPE.get(), scope);
    }
}

impl<'a> LocalHold<'a> {
    pub fn enter(scope: &'a mut HoldScope<'a>) -> LocalHold<'a> {
        unsafe {
            scope.next = mem::transmute(LocalHold::get());
            LocalHold::set(Some(mem::transmute::<&'a HoldScope<'a>, &'static HoldScope<'static>>(scope)));
            LocalHold { scope: scope }
        }
    }
}

unsafe impl<'a> Hold<'a> for LocalHold<'a> {
    unsafe fn alloc(&self, layout: Layout) -> Result<Block<'a>, HoldError> {
        self.scope.alloc(layout)
    }

    unsafe fn dealloc(&self, _block: Block<'a>) -> usize {
        // Never directly deallocates anything; deallocations always dispatch
        // to the underlying hold.
        unimplemented!();
    }

    unsafe fn resize(&self, _block: Block<'a>, _layout: Layout) -> Result<Block<'a>, HoldError> {
        // Never directly resizes anything; reallocations always dispatch to
        // underlying hold.
        unimplemented!();
    }
}

impl<'a> Drop for LocalHold<'a> {
    fn drop(&mut self) {
        unsafe { LocalHold::set(mem::transmute::<Option<&'a HoldScope<'a>>, Option<&'static HoldScope<'static>>>(self.scope.next)); }
    }
}

/// Degenerate `Hold` that can only `alloc` and `dealloc` zero-sized blocks.
#[repr(C)]
struct EmptyHold<'a> {
    /// Polymorphic hold type
    base: Reified<Hold<'a>>,
    /// Tag shared by all zero-sized allocations in the empty hold
    zero: AllocTag<'a>,
}

impl<'a> EmptyHold<'a> {
    /// Returns the zero-sized block for the empty hold.
    #[inline(always)]
    fn empty(&self) -> Block<'a> {
        // Get the address of the zero-sized allocation tag.
        let tag_addr = &self.zero as *const AllocTag<'a> as usize;
        // Get the address of the zero-sized block immediately following the tag.
        let zero_addr = tag_addr.wrapping_add(mem::size_of::<AllocTag<'a>>());
        // Return the zero-sized block for the empty hold.
        unsafe { Block::from_raw_parts(zero_addr as *mut u8, 0) }
    }
}

unsafe impl<'a> Send for EmptyHold<'a> {
}

unsafe impl<'a> Sync for EmptyHold<'a> {
}

unsafe impl<'a> Hold<'a> for EmptyHold<'a> {
    unsafe fn alloc(&self, layout: Layout) -> Result<Block<'a>, HoldError> {
        // Check if the layout represents a zero-sized type.
        if layout.size() == 0 {
            // Return our shared empty block.
            return Ok(self.empty());
        }
        // Can't actually allocate anything.
        Err(HoldError::Unsupported("empty hold"))
    }

    unsafe fn dealloc(&self, block: Block<'a>) -> usize {
        // Check if the block is our shared empty block.
        if block == self.empty() {
            // "Deallocate" the block.
            return 0;
        }
        // Can't actually deallocate anything.
        panic!("empty hold");
    }

    unsafe fn resize(&self, block: Block<'a>, layout: Layout) -> Result<Block<'a>, HoldError> {
        // Check if the block is our shared empty block,
        // and if the layout represents a zero-sized type.
        if block == self.empty() && layout.size() == 0 {
            // Return our shared empty block.
            return Ok(block);
        }
        // Can't actually resize anything.
        Err(HoldError::Unsupported("empty hold"))
    }
}

impl<'a> Reify<'a, Hold<'a> + 'a> for EmptyHold<'a> {
    #[inline]
    unsafe fn deify(object: &mut (Hold<'a> + 'a)) {
        Reified::<Hold<'a> + 'a>::deify(mem::transmute(object));
    }

    #[inline]
    unsafe fn reify(base: &'a Reified<Hold<'a> + 'a>) -> &'a (Hold<'a> + 'a) {
        mem::transmute(base.reify())
    }
}

/// An object allocated by a `Hold`.
pub trait Holder<'a> {
    /// Returns the `Hold` that allocated this object.
    fn holder(&self) -> &'a dyn Hold<'a>;
}

/// Hold memory allocation error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HoldError {
    /// Improper structure alignment.
    Misaligned,
    /// Structure size overflow.
    Oversized,
    /// Insufficient available memory.
    OutOfMemory,
    /// Unsupported operation; will never succeed.
    Unsupported(&'static str),
}

impl From<LayoutError> for HoldError {
    #[inline]
    fn from(error: LayoutError) -> HoldError {
        match error {
            LayoutError::Misaligned => HoldError::Misaligned,
            LayoutError::Oversized => HoldError::Oversized,
        }
    }
}

impl From<HeapError> for HoldError {
    #[inline]
    fn from(error: HeapError) -> HoldError {
        match error {
            HeapError::Misaligned => HoldError::Misaligned,
            HeapError::Oversized => HoldError::Oversized,
            HeapError::OutOfMemory => HoldError::OutOfMemory,
            HeapError::Unsupported(reason) => HoldError::Unsupported(reason),
        }
    }
}
