extern crate tg_mem;

use tg_mem::block::Block;
use tg_mem::alloc::Pack;
use tg_mem::lease::RawBox;

#[test]
fn test_raw_box_hold_new() {
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
        assert_eq!(x, x);
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
fn test_raw_box_hold_copy_slice() {
    static mut TEST_AREA: [u8; 4096] = [0; 4096];
    let pack = Pack::new(unsafe { Block::from_slice(&mut TEST_AREA) });

    assert_eq!(pack.live(), 0);
    assert_eq!(pack.used(), 0);
    assert_eq!(pack.free(), 4064);
    {
        let xs = RawBox::<[usize]>::hold_copy(pack, &[5, 9]);
        assert_eq!(pack.live(), 1);
        assert_eq!(pack.used(), 16);
        assert_eq!(pack.free(), 4040);
        assert_eq!(xs.len(), 2);
        assert_eq!(&*xs, &[5, 9]);
    }
    assert_eq!(pack.live(), 0);
    assert_eq!(pack.used(), 0);
    assert_eq!(pack.free(), 4064);
}

#[test]
fn test_raw_box_hold_copy_str() {
    static mut TEST_AREA: [u8; 4096] = [0; 4096];
    let pack = Pack::new(unsafe { Block::from_slice(&mut TEST_AREA) });

    assert_eq!(pack.live(), 0);
    assert_eq!(pack.used(), 0);
    assert_eq!(pack.free(), 4064);
    {
        let xs = RawBox::hold_copy(pack, "test");
        assert_eq!(pack.live(), 1);
        assert_eq!(pack.used(), 8);
        assert_eq!(pack.free(), 4048);
        assert_eq!(xs.len(), 4);
        assert_eq!(&*xs, "test");
    }
    assert_eq!(pack.live(), 0);
    assert_eq!(pack.used(), 0);
    assert_eq!(pack.free(), 4064);
}
