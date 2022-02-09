use core::cmp;
use core::fmt;
use core::mem;
use core::num::NonZeroUsize;
use core::usize;

/// Size and alignment constraints for a memory block.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    /// Required size in bytes of a valid memory block.
    size: usize,
    /// Required power-of-two base address alignment for a valid memory block.
    align: NonZeroUsize,
}

impl Layout {
    /// Returns a zero-sized `Layout` with byte alignment.
    #[inline]
    pub const fn empty() -> Layout {
        unsafe { Layout::from_size_align_unchecked(0, 1) }
    }

    /// Returns a `Layout` with the given size and power-of-two alignment.
    #[inline]
    pub const unsafe fn from_size_align_unchecked(size: usize, align: usize) -> Layout {
        Layout { size: size, align: NonZeroUsize::new_unchecked(align) }
    }

    /// Returns a `Layout` with the given size and power-of-two alignment,
    /// or `None` for invalid constraints.
    #[inline]
    pub fn from_size_align(size: usize, align: usize) -> Result<Layout, LayoutError> {
        if !align.is_power_of_two() {
            return Err(LayoutError::Misaligned);
        }
        if align > 1 << 31 {
            return Err(LayoutError::Misaligned);
        }
        if size > usize::MAX - (align - 1) {
            return Err(LayoutError::Oversized);
        }
        Ok(unsafe { Layout::from_size_align_unchecked(size, align) })
    }

    /// Returns the `Layout` of the parameterized type.
    #[inline]
    pub fn for_type<T>() -> Layout {
        let size = mem::size_of::<T>();
        let align = mem::align_of::<T>();
        unsafe { Layout::from_size_align_unchecked(size, align) }
    }

    /// Returns the `Layout` of the given `value`.
    #[inline]
    pub fn for_value<T: ?Sized>(value: &T) -> Layout {
        let size = mem::size_of_val(value);
        let align = mem::align_of_val(value);
        unsafe { Layout::from_size_align_unchecked(size, align) }
    }

    /// Returns the `Layout` of an array of `len` values of the parameterized type.
    #[inline]
    pub fn for_array<T>(len: usize) -> Result<Layout, LayoutError> {
        let align = mem::align_of::<T>();
        let stride = mem::size_of::<T>().wrapping_add(align).wrapping_sub(1) & !align.wrapping_sub(1);
        let size = match stride.checked_mul(len) {
            Some(size) => size,
            None => return Err(LayoutError::Oversized),
        };
        if size > usize::MAX - (align - 1) {
            return Err(LayoutError::Oversized);
        }
        Ok(unsafe { Layout::from_size_align_unchecked(size, align) })
    }

    #[inline]
    pub unsafe fn for_array_unchecked<T>(len: usize) -> Layout {
        let align = mem::align_of::<T>();
        let stride = mem::size_of::<T>().wrapping_add(align).wrapping_sub(1) & !align.wrapping_sub(1);
        let size = stride.wrapping_mul(len);
        Layout::from_size_align_unchecked(size, align)
    }

    /// Returns the required size in bytes of a valid memory block.
    #[inline]
    pub const fn size(&self) -> usize {
        self.size
    }

    /// Returns the required power-of-two base address alignment for a valid memory block.
    #[inline]
    pub fn align(&self) -> usize {
        self.align.get()
    }

    /// Returns this layout with at least `align` byte alignment.
    #[inline]
    pub fn aligned_to(&self, align: usize) -> Layout {
        if !align.is_power_of_two() {
            panic!();
        }
        let align = cmp::max(self.align.get(), align);
        unsafe { Layout::from_size_align_unchecked(self.size, align) }
    }

    /// Returns this layout with at least the alignment of the parameterized type.
    #[inline]
    pub fn aligned_to_type<T>(&self) -> Layout {
        let align = cmp::max(self.align.get(), mem::align_of::<T>());
        unsafe { Layout::from_size_align_unchecked(self.size, align) }
    }

    /// Returns this layout with at least the alignment of the given value.
    #[inline]
    pub fn aligned_to_value<T: ?Sized>(&self, value: &T) -> Layout {
        let align = cmp::max(self.align.get(), mem::align_of_val(value));
        unsafe { Layout::from_size_align_unchecked(self.size, align) }
    }

    /// Returns this layout with its size rounded up to the given alignment.
    #[inline]
    pub fn padded_to(&self, align: usize) -> Layout {
        if !align.is_power_of_two() {
            panic!();
        }
        let size = self.size.wrapping_add(align).wrapping_sub(1) & !align.wrapping_sub(1);
        unsafe { Layout::from_size_align_unchecked(size, cmp::max(self.align.get(), align)) }
    }

    /// Returns this layout with its size rouneded up to the alignment of a subsequent field
    /// of the given type.
    #[inline]
    pub fn padded_to_type<T>(&self) -> Layout {
        let align = mem::align_of::<T>();
        let size = self.size.wrapping_add(align).wrapping_sub(1) & !align.wrapping_sub(1);
        unsafe { Layout::from_size_align_unchecked(size, cmp::max(self.align.get(), align)) }
    }

    /// Returns this layout with its size rounded up to the the alignment of alignment of the
    /// given field.
    #[inline]
    pub fn padded_to_value<T: ?Sized>(&self, value: &T) -> Layout {
        let align = mem::align_of_val(value);
        let size = self.size.wrapping_add(align).wrapping_sub(1) & !align.wrapping_sub(1);
        unsafe { Layout::from_size_align_unchecked(size, cmp::max(self.align.get(), align)) }
    }

    /// Returns the `Layout` of a struct with this layout as its first member,
    /// and `that` layout as its second member.
    #[inline]
    pub fn extended(&self, that: Layout) -> Result<(Layout, usize), LayoutError> {
        let next_align = that.align.get();
        let align = cmp::max(self.align.get(), next_align);
        let offset = self.size.wrapping_add(next_align).wrapping_sub(1) & !next_align.wrapping_sub(1);
        let size = match offset.checked_add(that.size) {
            Some(size) => size,
            None => return Err(LayoutError::Oversized),
        };
        if size > usize::MAX - (align - 1) {
            return Err(LayoutError::Oversized);
        }
        Ok((unsafe { Layout::from_size_align_unchecked(size, align) }, offset))
    }

    #[inline]
    pub fn extended_by_type<T>(&self) -> Result<(Layout, usize), LayoutError> {
        self.extended(Layout::for_type::<T>())
    }

    #[inline]
    pub fn extended_by_value<T: ?Sized>(&self, value: &T) -> Result<(Layout, usize), LayoutError> {
        self.extended(Layout::for_value(value))
    }

    #[inline]
    pub fn extended_by_array<T>(&self, len: usize) -> Result<(Layout, usize), LayoutError> {
        self.extended(Layout::for_array::<T>(len)?)
    }

    /// Returns the `Layout` of a struct with this layout as its first member,
    /// and `that` layout as its second member, without checking for size overflow.
    #[inline]
    pub unsafe fn extended_unchecked(&self, that: Layout) -> (Layout, usize) {
        let next_align = that.align.get();
        let align = cmp::max(self.align.get(), next_align);
        let offset = self.size.wrapping_add(next_align).wrapping_sub(1) & !next_align.wrapping_sub(1);
        let size = offset.wrapping_add(that.size);
        (Layout::from_size_align_unchecked(size, align), offset)
    }

    #[inline]
    pub unsafe fn extended_by_type_unchecked<T>(&self) -> (Layout, usize) {
        self.extended_unchecked(Layout::for_type::<T>())
    }

    #[inline]
    pub unsafe fn extended_by_value_unchecked<T: ?Sized>(&self, value: &T) -> (Layout, usize) {
        self.extended_unchecked(Layout::for_value(value))
    }

    #[inline]
    pub unsafe fn extended_by_array_unchecked<T>(&self, len: usize) -> (Layout, usize) {
        self.extended_unchecked(Layout::for_array_unchecked::<T>(len))
    }

    /// Returns the `Layout` of an array with `len` elements of this layout.
    #[inline]
    pub fn repeated(&self, len: usize) -> Result<(Layout, usize), LayoutError> {
        let align = self.align.get();
        let stride = self.size.wrapping_add(align).wrapping_sub(1) & !align.wrapping_sub(1);
        let size = match stride.checked_mul(len) {
            Some(size) => size,
            None => return Err(LayoutError::Oversized),
        };
        if size > usize::MAX - (align - 1) {
            return Err(LayoutError::Oversized);
        }
        Ok((unsafe { Layout::from_size_align_unchecked(size, align) }, stride))
    }

    /// Returns the `Layout` of an array with `len` elements of this layout,
    /// without checking for size overflow.
    #[inline]
    pub unsafe fn repeated_unchecked(&self, len: usize) -> (Layout, usize) {
        let align = self.align.get();
        let stride = self.size.wrapping_add(align).wrapping_sub(1) & !align.wrapping_sub(1);
        let size = stride.wrapping_mul(len);
        (Layout::from_size_align_unchecked(size, align), stride)
    }
}

impl fmt::Debug for Layout {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Layout")
            .field("size", &self.size)
            .field("align", &self.align.get())
            .finish()
    }
}

/// Memory layout error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LayoutError {
    /// Improper structure alignment.
    Misaligned,
    /// Structure size overflow.
    Oversized,
}
