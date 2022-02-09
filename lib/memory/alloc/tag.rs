use core::marker::PhantomPinned;
use core::mem;
use core::ptr;
use core::sync::atomic::AtomicPtr;
use core::sync::atomic::Ordering::Relaxed;
use swim_core::reify::Reified;
use crate::block::{Block, Layout};
use crate::alloc::{Hold, HoldError};

/// Back-pointer to the `Hold` that allocated a block. An `AllocTag`
/// immediately precedes every memory block allocated by a `Hold`.
#[repr(C)]
pub struct AllocTag<'a> {
    /// Atomic thin pointer to the `Hold` that allocated this tag. The pointed-to
    /// `HoldBase` contains the vtable of the reified `Hold` trait object.
    pub(crate) base: AtomicPtr<Reified<Hold<'a>>>,
    /// Pin to the preceding aligned address of the tagged memory block.
    pinned: PhantomPinned,
}

impl<'a> AllocTag<'a> {
    /// Returns a new `AllocTag` that points to an invalid hold address. Used
    /// as a placeholder when statically initializing fixed memory allocations.
    /// The tag is not safe to use until `AllocTag::init` is called with the
    /// base address of a valid `Hold`.
    ///
    /// # Safety
    ///
    /// Exposing a null `AllocTag` can cause segmentation faults.
    #[inline(always)]
    pub const unsafe fn null() -> AllocTag<'a> {
        AllocTag {
            base: AtomicPtr::new(ptr::null_mut()),
            pinned: PhantomPinned,
        }
    }

    /// Returns a new `AllocTag` that points to the empty hold.
    #[inline]
    pub fn empty() -> AllocTag<'a> {
        AllocTag {
            base: AtomicPtr::new(&*Hold::empty() as *const Hold<'a> as *mut Reified<Hold<'a>>),
            pinned: PhantomPinned,
        }
    }

    /// Returns a new `AllocTag` that points back to the `Hold` that allocated this tag.
    #[inline]
    pub fn new(base: &Reified<Hold<'a>>) -> AllocTag<'a> {
        AllocTag {
            base: AtomicPtr::new(base as *const Reified<Hold<'a>> as *mut Reified<Hold<'a>>),
            pinned: PhantomPinned,
        }
    }

    /// Initializes this `AllocTag` to point to the `Hold` at `base`. This
    /// operation is idempotent when called repeatedly with the same `base` address.
    #[inline(always)]
    pub fn init(&mut self, base: &Reified<Hold<'a>>) {
        self.base.store(base as *const Reified<Hold<'a>> as *mut Reified<Hold<'a>>, Relaxed);
    }

    /// Returns a pointer to the `AllocTag` preceding a `data` pointer allocated by a `Hold`.
    #[inline]
    pub fn from_ptr(data: *mut u8) -> *mut AllocTag<'a> {
        (data as usize).wrapping_sub(mem::size_of::<AllocTag>()) as *mut AllocTag
    }

    /// Returns a reference to the `Hold` that allocated this tag.
    #[inline]
    pub fn holder(self: *mut AllocTag<'a>) -> &'a dyn Hold<'a> {
        // Get the thin base pointer. No ordering constraint.
        let base = unsafe { (*self).base.load(Relaxed) };
        if base.is_null() {
            panic!("dangling pointer");
        }
        // Reify the thin base pointer into a trait object and return it.
        unsafe { mem::transmute((&*base).reify()) }
    }

    /// Instructs the `Hold` that allocated this tag to deallocate the `block`.
    #[inline]
    pub unsafe fn dealloc(self: *mut AllocTag<'a>, block: Block<'a>) {
        let base;
        // Check if the block has zero size.
        if block.size() != 0 {
            // Get and clear the thin base pointer. No ordering constraint.
            base = (*self).base.swap(ptr::null_mut(), Relaxed);
            if base.is_null() {
                panic!("double dealloc");
            }
        } else {
            // Get the thin base pointer. No ordering constraint.
            base = (*self).base.load(Relaxed);
        }
        // Reify the thin base pointer into a trait object.
        let hold = mem::transmute::<_, &'a dyn Hold<'a>>((&*base).reify());
        // Deallocate the block.
        hold.dealloc(block);
    }

    /// Asks the `Hold` that allocated this tag to attempt to resize the `block`.
    #[inline]
    pub unsafe fn resize(self: *mut AllocTag<'a>, block: Block<'a>, layout: Layout)
        -> Result<Block<'a>, HoldError>
    {
        // Get the thin base pointer. No ordering constraint.
        let base = (*self).base.load(Relaxed);
        if base.is_null() {
            panic!("dangling pointer");
        }
        // Reify the thin base pointer into a trait object.
        let hold = mem::transmute::<_, &'a dyn Hold<'a>>((&*base).reify());
        // Try to resize the block.
        hold.resize(block, layout)
    }
}
