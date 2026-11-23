//! A simple bump (early) memory allocator.
//!
//! The allocator splits one contiguous memory region into two parts:
//!
//! ```text
//! [ bytes-used | available | pages-used ]
//! |            | -->   <-- |            |
//! start       b_pos       p_pos        end
//! ```
//!
//! * Byte allocations grow forward from `b_pos`.
//! * Page allocations grow backward from `p_pos`.
//!
//! Byte allocations may be freed.  A simple "all-or-nothing" strategy is used:
//! the number of outstanding byte allocations (`b_count`) is recorded, and the
//! whole byte area is reclaimed only when the last allocation is freed.
//! Pages are never freed, because the free space between the byte area and the
//! page area cannot be compacted once allocations are interleaved.

#![no_std]

use allocator::{AllocError, AllocResult, BaseAllocator, ByteAllocator, PageAllocator};
use core::alloc::Layout;
use core::ptr::NonNull;

#[inline]
const fn align_up(pos: usize, align: usize) -> usize {
    (pos + align - 1) & !(align - 1)
}

#[inline]
const fn align_down(pos: usize, align: usize) -> usize {
    pos & !(align - 1)
}

/// Early memory allocator.
///
/// `SIZE` is the page size used by the page allocator part.
pub struct EarlyAllocator<const SIZE: usize> {
    /// Start address of the managed region.
    start: usize,
    /// Exclusive end address of the managed region.
    end: usize,
    /// Current bump pointer of the byte allocator.
    b_pos: usize,
    /// Number of outstanding byte allocations.
    b_count: usize,
    /// Current bottom address of the page allocator.
    p_pos: usize,
}

impl<const SIZE: usize> EarlyAllocator<SIZE> {
    /// Creates an empty allocator. It must be initialized with [`init`].
    ///
    /// [`init`]: BaseAllocator::init
    pub const fn new() -> Self {
        Self {
            start: 0,
            end: 0,
            b_pos: 0,
            b_count: 0,
            p_pos: 0,
        }
    }

    const fn is_initialized(&self) -> bool {
        self.end > self.start
    }
}

impl<const SIZE: usize> Default for EarlyAllocator<SIZE> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const SIZE: usize> BaseAllocator for EarlyAllocator<SIZE> {
    fn init(&mut self, start: usize, size: usize) {
        assert!(SIZE.is_power_of_two(), "SIZE must be a power of two");
        let start = align_up(start, SIZE);
        let end = align_down(start.saturating_add(size), SIZE);
        assert!(end > start, "memory region is too small");
        self.start = start;
        self.end = end;
        self.b_pos = start;
        self.b_count = 0;
        self.p_pos = end;
    }

    fn add_memory(&mut self, start: usize, size: usize) -> AllocResult {
        if !self.is_initialized() {
            self.init(start, size);
            return Ok(());
        }
        let start = align_up(start, SIZE);
        let end = align_down(start.saturating_add(size), SIZE);
        if start != self.end || end <= start {
            // Only a directly adjacent, page-aligned region can be merged.
            return Err(AllocError::NoMemory);
        }
        self.end = end;
        self.p_pos = end;
        Ok(())
    }
}

impl<const SIZE: usize> ByteAllocator for EarlyAllocator<SIZE> {
    fn alloc(&mut self, layout: Layout) -> AllocResult<NonNull<u8>> {
        if !self.is_initialized() {
            return Err(AllocError::NoMemory);
        }
        let align = layout.align();
        if !align.is_power_of_two() {
            return Err(AllocError::InvalidParam);
        }

        let pos = align_up(self.b_pos, align);
        let Some(new_pos) = pos.checked_add(layout.size()) else {
            return Err(AllocError::NoMemory);
        };
        // The byte area must not grow into the page area.
        if new_pos > self.p_pos {
            return Err(AllocError::NoMemory);
        }

        self.b_pos = new_pos;
        self.b_count += 1;
        // `pos` is inside the heap and therefore never zero.
        NonNull::new(pos as *mut u8).ok_or(AllocError::NoMemory)
    }

    fn dealloc(&mut self, _pos: NonNull<u8>, _layout: Layout) {
        debug_assert!(self.b_count > 0, "deallocate an unallocated region");
        if self.b_count > 0 {
            self.b_count -= 1;
            if self.b_count == 0 {
                // All byte allocations are gone: reclaim the whole byte area.
                self.b_pos = self.start;
            }
        }
    }

    fn total_bytes(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    fn used_bytes(&self) -> usize {
        self.b_pos.saturating_sub(self.start)
    }

    fn available_bytes(&self) -> usize {
        self.p_pos.saturating_sub(self.b_pos)
    }
}

impl<const SIZE: usize> PageAllocator for EarlyAllocator<SIZE> {
    const PAGE_SIZE: usize = SIZE;

    fn alloc_pages(&mut self, num_pages: usize, align_pow2: usize) -> AllocResult<usize> {
        if !self.is_initialized() || num_pages == 0 || !align_pow2.is_power_of_two() {
            return Err(AllocError::InvalidParam);
        }
        // A page allocation is at least page aligned.
        let align = align_pow2.max(SIZE);
        let Some(size) = num_pages.checked_mul(SIZE) else {
            return Err(AllocError::NoMemory);
        };
        let Some(top) = self.p_pos.checked_sub(size) else {
            return Err(AllocError::NoMemory);
        };
        // The page area grows downward, so align the new bottom downwards.
        let pos = align_down(top, align);
        if pos < self.b_pos {
            return Err(AllocError::NoMemory);
        }
        self.p_pos = pos;
        Ok(pos)
    }

    fn dealloc_pages(&mut self, _pos: usize, _num_pages: usize) {
        // Pages are never reclaimed by the early allocator: the bump pointer
        // of the byte area may already have moved past the page region that
        // would have to be merged back.
    }

    fn total_pages(&self) -> usize {
        self.end.saturating_sub(self.start) / SIZE
    }

    fn used_pages(&self) -> usize {
        self.end.saturating_sub(self.p_pos) / SIZE
    }

    fn available_pages(&self) -> usize {
        self.p_pos.saturating_sub(self.b_pos) / SIZE
    }
}
