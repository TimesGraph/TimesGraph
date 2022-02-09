extern crate tg_mem;

use tg_mem::block::Block;
use tg_mem::alloc::Pack;
use tg_mem::lease::{RawBox, RawBuf};

#[test]
fn test_pack_alloc_dealloc_boxes() {
    static mut TEST_AREA: [u8; 4096] = [0; 4096];
    let pack = Pack::new(unsafe { Block::from_slice(&mut TEST_AREA) });

    assert_eq!(pack.live(), 0);
    assert_eq!(pack.used(), 0);
    assert_eq!(pack.free(), 4064);
    {
        let x = RawBox::hold_new(pack, 5usize);
        assert_eq!(pack.live(), 1);
        assert_eq!(pack.used(), 8);
        assert_eq!(pack.free(), 4048);
        assert_eq!(*x, 5);
        {
            let y = RawBox::hold_new(pack, 9usize);
            assert_eq!(pack.live(), 2);
            assert_eq!(pack.used(), 16);
            assert_eq!(pack.free(), 4032);
            assert_eq!(*x, 5);
            assert_eq!(*y, 9);
        }
        assert_eq!(pack.live(), 1);
        assert_eq!(pack.used(), 8);
        assert_eq!(pack.free(), 4048);
        assert_eq!(*x, 5);
    }
    assert_eq!(pack.live(), 0);
    assert_eq!(pack.used(), 0);
    assert_eq!(pack.free(), 4064);
}

#[test]
pub fn test_pack_alloc_dealloc_bufs() {
    static mut TEST_AREA: [u8; 4096] = [0; 4096];
    let pack = Pack::new(unsafe { Block::from_slice(&mut TEST_AREA) });

    assert_eq!(pack.live(), 0);
    assert_eq!(pack.used(), 0);
    {
        let mut xs = RawBuf::<usize>::hold_empty(pack);
        assert_eq!(pack.live(), 1);
        assert_eq!(pack.used(), 0);
        xs.push(5);
        assert_eq!(pack.live(), 1);
        assert_eq!(pack.used(), 8);
        assert_eq!(xs[0], 5);
        xs.push(9);
        assert_eq!(pack.live(), 1);
        assert_eq!(pack.used(), 16);
        assert_eq!(xs[0], 5);
        assert_eq!(xs[1], 9);
        {
            let mut ys = RawBuf::<usize>::hold_empty(pack);
            assert_eq!(pack.live(), 2);
            assert_eq!(pack.used(), 16);
            ys.push(11);
            assert_eq!(pack.live(), 2);
            assert_eq!(pack.used(), 24);
            assert_eq!(xs[0], 5);
            assert_eq!(xs[1], 9);
            assert_eq!(ys[0], 11);
            ys.push(13);
            assert_eq!(pack.live(), 2);
            assert_eq!(pack.used(), 32);
            assert_eq!(xs[0], 5);
            assert_eq!(xs[1], 9);
            assert_eq!(ys[0], 11);
            assert_eq!(ys[1], 13);
        }
        assert_eq!(pack.live(), 1);
        assert_eq!(pack.used(), 16);
        assert_eq!(xs[0], 5);
        assert_eq!(xs[1], 9);
    }
    assert_eq!(pack.live(), 0);
    assert_eq!(pack.used(), 0);
}
