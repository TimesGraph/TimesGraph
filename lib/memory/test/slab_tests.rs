extern crate tg_mem;

use tg_mem::block::{Block, Layout};
use tg_mem::alloc::{Heap, Slab};

#[test]
fn test_slab_alloc_dealloc() {
    static mut TEST_HUNK: [u8; 4096] = [0; 4096];
    unsafe {
        let slab = Slab::new(Block::from_slice(&mut TEST_HUNK), 256);

        assert_eq!(slab.live(), 0);
        assert_eq!(slab.dead(), 16);
        {
            let x = slab.alloc(Layout::from_size_align_unchecked(256, 1)).unwrap();
            assert_eq!(slab.live(), 1);
            assert_eq!(slab.dead(), 15);
            {
                let y = slab.alloc(Layout::from_size_align_unchecked(256, 1)).unwrap();
                assert_eq!(slab.live(), 2);
                assert_eq!(slab.dead(), 14);
                slab.dealloc(y);
            }
            assert_eq!(slab.live(), 1);
            assert_eq!(slab.dead(), 15);
            slab.dealloc(x);
        }
        assert_eq!(slab.live(), 0);
        assert_eq!(slab.dead(), 16);
    }
}
