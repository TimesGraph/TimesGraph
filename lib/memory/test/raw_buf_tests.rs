extern crate tg_mem;

use tg_mem::block::Block;
use tg_mem::alloc::Pack;
use tg_mem::lease::RawBuf;

#[test]
fn test_raw_buf_hold_cap() {
    static mut TEST_AREA: [u8; 4096] = [0; 4096];
    let pack = Pack::new(unsafe { Block::from_slice(&mut TEST_AREA) });

    assert_eq!(pack.live(), 0);
    assert_eq!(pack.used(), 0);
    assert_eq!(pack.free(), 4064);
    {
        let mut xs = RawBuf::<usize>::hold_cap(pack, 2);
        assert_eq!(pack.live(), 1);
        assert_eq!(pack.used(), 16);
        assert_eq!(pack.free(), 4040);
        assert_eq!(xs.len(), 0);
        assert_eq!(xs.cap(), 2);

        xs.push(5);
        assert_eq!(pack.live(), 1);
        assert_eq!(pack.used(), 16);
        assert_eq!(pack.free(), 4040);
        assert_eq!(xs.len(), 1);
        assert_eq!(xs.cap(), 2);
        assert_eq!(xs[0], 5);

        xs.push(9);
        assert_eq!(pack.live(), 1);
        assert_eq!(pack.used(), 16);
        assert_eq!(pack.free(), 4040);
        assert_eq!(xs.len(), 2);
        assert_eq!(xs.cap(), 2);
        assert_eq!(xs[0], 5);
        assert_eq!(xs[1], 9);
    }
    assert_eq!(pack.live(), 0);
    assert_eq!(pack.used(), 0);
    assert_eq!(pack.free(), 4064);
}
