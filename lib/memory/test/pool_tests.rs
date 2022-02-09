extern crate swim_mem;

use swim_mem::block::Block;
use swim_mem::alloc::{Slab, Pool};
use swim_mem::lease::{RawBox, RawBuf};

#[test]
fn test_pool_alloc_dealloc_boxes() {
    static mut TEST_HUNK: [u8; 4096] = [0; 4096];
    let slab = Slab::new(unsafe { Block::from_slice(&mut TEST_HUNK) }, 256);
    let pool = &Pool::new(&slab);

    assert_eq!(pool.live(), 0);
    assert_eq!(pool.used(), 0);
    {
        let x = RawBox::hold_new(pool, 5usize);
        assert_eq!(pool.live(), 1);
        assert_eq!(pool.used(), 8);
        assert_eq!(*x, 5);
        {
            let y = RawBox::hold_new(pool, 9usize);
            assert_eq!(pool.live(), 2);
            assert_eq!(pool.used(), 16);
            assert_eq!(*x, 5);
            assert_eq!(*y, 9);
        }
        assert_eq!(pool.live(), 1);
        assert_eq!(pool.used(), 8);
        assert_eq!(*x, 5);
    }
    assert_eq!(pool.live(), 0);
    assert_eq!(pool.used(), 0);
}

#[test]
pub fn test_pool_alloc_dealloc_bufs() {
    static mut TEST_HUNK: [u8; 4096] = [0; 4096];
    let slab = Slab::new(unsafe { Block::from_slice(&mut TEST_HUNK) }, 256);
    let pool = &Pool::new(&slab);

    assert_eq!(pool.live(), 0);
    assert_eq!(pool.used(), 0);
    {
        let mut xs = RawBuf::<usize>::hold_empty(pool);
        assert_eq!(pool.live(), 1);
        assert_eq!(pool.used(), 0);
        xs.push(5);
        assert_eq!(pool.live(), 1);
        assert_eq!(pool.used(), 8);
        assert_eq!(xs[0], 5);
        xs.push(9);
        assert_eq!(pool.live(), 1);
        assert_eq!(pool.used(), 16);
        assert_eq!(xs[0], 5);
        assert_eq!(xs[1], 9);
        {
            let mut ys = RawBuf::<usize>::hold_empty(pool);
            assert_eq!(pool.live(), 2);
            assert_eq!(pool.used(), 16);
            ys.push(11);
            assert_eq!(pool.live(), 2);
            assert_eq!(pool.used(), 24);
            assert_eq!(xs[0], 5);
            assert_eq!(xs[1], 9);
            assert_eq!(ys[0], 11);
            ys.push(13);
            assert_eq!(pool.live(), 2);
            assert_eq!(pool.used(), 32);
            assert_eq!(xs[0], 5);
            assert_eq!(xs[1], 9);
            assert_eq!(ys[0], 11);
            assert_eq!(ys[1], 13);
        }
        assert_eq!(pool.live(), 1);
        assert_eq!(pool.used(), 16);
        assert_eq!(xs[0], 5);
        assert_eq!(xs[1], 9);
    }
    assert_eq!(pool.live(), 0);
    assert_eq!(pool.used(), 0);
}
