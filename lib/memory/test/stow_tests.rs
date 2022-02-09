extern crate tg_mem;

use tg_mem::block::Block;
use tg_mem::alloc::{StowInto, Pack};
use tg_mem::lease::RawBox;

#[test]
fn test_stow_boxes() {
    static mut TEST0_AREA: [u8; 4096] = [0; 4096];
    static mut TEST1_AREA: [u8; 4096] = [0; 4096];
    let pack0 = Pack::new(unsafe { Block::from_slice(&mut TEST0_AREA) });

    assert_eq!(pack0.live(), 0);
    assert_eq!(pack0.used(), 0);
    assert_eq!(pack0.free(), 4064);
    {
        let y0: RawBox<usize>;
        let pack1 = Pack::new(unsafe { Block::from_slice(&mut TEST1_AREA) });
        assert_eq!(pack1.live(), 0);
        assert_eq!(pack1.used(), 0);
        assert_eq!(pack1.free(), 4064);
        let x1 = RawBox::hold_new(pack1, 5usize);
        assert_eq!(pack1.live(), 1);
        assert_eq!(pack1.used(), 8);
        assert_eq!(pack1.free(), 4048);
        assert_eq!(*x1, 5);
        {
            let y1 = RawBox::hold_new(pack1, 9usize);
            assert_eq!(pack1.live(), 2);
            assert_eq!(pack1.used(), 16);
            assert_eq!(pack1.free(), 4032);
            assert_eq!(*x1, 5);
            assert_eq!(*y1, 9);
            y0 = y1.stow_into(pack0);
            assert_eq!(pack0.live(), 1);
            assert_eq!(pack0.used(), 8);
            assert_eq!(pack0.free(), 4048);
            assert_eq!(pack1.live(), 1);
            assert_eq!(pack1.used(), 8);
            assert_eq!(pack1.free(), 4048);
            assert_eq!(*x1, 5);
            assert_eq!(*y0, 9);
        }
        let x0: RawBox<usize> = x1.stow_into(pack0);
        assert_eq!(pack0.live(), 2);
        assert_eq!(pack0.used(), 16);
        assert_eq!(pack0.free(), 4032);
        assert_eq!(pack1.live(), 0);
        assert_eq!(pack1.used(), 0);
        assert_eq!(pack1.free(), 4064);
        assert_eq!(*x0, 5);
        assert_eq!(*y0, 9);
    }
    assert_eq!(pack0.live(), 0);
    assert_eq!(pack0.used(), 0);
    assert_eq!(pack0.free(), 4064);
}
