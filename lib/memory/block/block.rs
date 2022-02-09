use core::fmt;
use core::hash;
use core::marker::PhantomData;
use core::ptr::NonNull;
use core::slice;
use crate::block::ZSP;

/// Address, size, and lifetime of a raw memory area.
#[derive(Clone, Copy)]
pub struct Block<'a> {
    /// Non-null pointer to the base address of the memory area.
    data: NonNull<u8>,
    /// Number of bytes in the memory area.
    size: usize,
    /// Covariant lifetime of the memory area.
    marker: PhantomData<&'a ()>,
}

unsafe impl<'a> Send for Block<'a> {
}

unsafe impl<'a> Sync for Block<'a> {
}

impl<'a> Block<'a> {
    /// Returns a zero-length `Block` with an undereferenceable sentinel pointer.
    #[inline]
    pub const fn empty() -> Block<'a> {
        Block {
            data: unsafe { NonNull::new_unchecked(ZSP) },
            size: 0,
            marker: PhantomData,
        }
    }

    /// Constructs a `Block` from a non-zero `data` pointer to `size` bytes.
    ///
    /// # Safety
    ///
    /// The returned `Block` logically takes ownership of the pointed-to `data`.
    #[inline]
    pub const unsafe fn from_raw_parts(data: *mut u8, size: usize) -> Block<'a> {
        Block {
            data: NonNull::new_unchecked(data),
            size: size,
            marker: PhantomData,
        }
    }

    /// Constructs a `Block` from a slice of bytes.
    ///
    /// # Safety
    ///
    /// The returned `Block` logically takes ownership of the bytes in `slice`.
    #[inline]
    pub fn from_slice(slice: &'a mut [u8]) -> Block<'a> {
        Block {
            data: unsafe { NonNull::new_unchecked(slice.as_mut_ptr()) },
            size: slice.len(),
            marker: PhantomData,
        }
    }

    /// Returns the number of bytes of memory owned by this `Block`.
    #[inline]
    pub fn size(&self) -> usize {
        self.size
    }

    /// Returns a slice of the memory owned by this `Block`.
    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.data.as_ptr(), self.size) }
    }

    /// Returns a mutable slice of the memory owned by this `Block`.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.data.as_ptr(), self.size) }
    }

    /// Returns a pointer to the memory owned by this `Block.`
    #[inline]
    pub fn as_ptr(&self) -> *mut u8 {
        self.data.as_ptr()
    }

    /// Consumes this `Block` and returns a mutable pointer to its memory.
    #[inline]
    pub fn into_raw(self) -> *mut u8 {
        self.data.as_ptr()
    }
}

impl<'a> PartialEq for Block<'a> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.data.as_ptr() == other.data.as_ptr() && self.size == other.size
    }
}

impl<'a> Eq for Block<'a> {
}

impl<'a> hash::Hash for Block<'a> {
    #[inline]
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.data.as_ptr().hash(state)
    }
}

impl<'a> fmt::Debug for Block<'a> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.data.as_ptr(), f)
    }
}

impl<'a> fmt::Pointer for Block<'a> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.data.as_ptr(), f)
    }
}
