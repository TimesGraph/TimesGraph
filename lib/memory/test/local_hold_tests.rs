#[macro_use]
extern crate tg_mem;

use tg_mem::block::Block;
use tg_mem::alloc::{Hold, Pack};
use tg_mem::lease::RawBox;

#[test]
fn test_local_hold_alloc_dealloc_boxes() {
    static mut TEST_AREA: [u8; 4096] = [0; 4096];
    let pack = Pack::new(unsafe { Block::from_slice(&mut TEST_AREA) });
    hold_scope! {
        local hold scope = pack;
    }

    assert_eq!(pack.live(), 0);
    assert_eq!(pack.used(), 0);
    assert_eq!(pack.free(), 4064);
    {
        let x = RawBox::hold_new(unsafe { Hold::local() }, 5usize);
        assert_eq!(pack.live(), 1);
        assert_eq!(pack.used(), 8);
        assert_eq!(pack.free(), 4048);
        assert_eq!(*x, 5);
        {
            let y = RawBox::hold_new(unsafe { Hold::local() }, 9usize);
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
