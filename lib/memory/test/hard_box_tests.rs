extern crate swim_mem;

use std::mem;
use swim_mem::block::Block;
use swim_mem::alloc::{StowInto, Pack};
use swim_mem::lease::{ArcError, HardBox, SoftBox, RefBox, MutBox};

#[test]
fn test_hard_box_hold_new() {
    static mut TEST_AREA: [u8; 4096] = [0; 4096];
    let pack = Pack::new(unsafe { Block::from_slice(&mut TEST_AREA) });

    assert_eq!(pack.live(), 0);
    assert_eq!(pack.used(), 0);
    assert_eq!(pack.free(), 4064);
    {
        let x = HardBox::hold_new(pack, 5usize);
        assert_eq!(pack.live(), 1);
        assert_eq!(pack.used(), 24);
        assert_eq!(pack.free(), 4032);
        assert_eq!(x.hard_count(), 1);
        assert_eq!(x.soft_count(), 0);
        assert_eq!(x.ref_count(), 0);
        assert_eq!(x.is_mut(), false);
        assert_eq!(x.is_relocated(), false);
        assert_eq!(x.is_aliased(), false);
    }
    assert_eq!(pack.live(), 0);
    assert_eq!(pack.used(), 0);
    assert_eq!(pack.free(), 4064);
}

#[test]
fn test_hard_box_to_soft() {
    static mut TEST_AREA: [u8; 4096] = [0; 4096];
    let pack = Pack::new(unsafe { Block::from_slice(&mut TEST_AREA) });

    assert_eq!(pack.live(), 0);
    assert_eq!(pack.used(), 0);
    assert_eq!(pack.free(), 4064);
    {
        let x = HardBox::hold_new(pack, 5usize);
        assert_eq!(pack.live(), 1);
        assert_eq!(pack.used(), 24);
        assert_eq!(pack.free(), 4032);
        assert_eq!(x.hard_count(), 1);
        assert_eq!(x.soft_count(), 0);
        assert_eq!(x.ref_count(), 0);
        assert_eq!(x.is_mut(), false);
        assert_eq!(x.is_relocated(), false);
        assert_eq!(x.is_aliased(), false);

        let y = x.to_soft();
        assert_eq!(pack.live(), 1);
        assert_eq!(pack.used(), 24);
        assert_eq!(pack.free(), 4032);
        assert_eq!(x.hard_count(), 1);
        assert_eq!(x.soft_count(), 1);
        assert_eq!(x.ref_count(), 0);
        assert_eq!(x.is_mut(), false);
        assert_eq!(x.is_relocated(), false);
        assert_eq!(x.is_aliased(), false);
        assert_eq!(y.hard_count(), 1);
        assert_eq!(y.soft_count(), 1);
        assert_eq!(y.ref_count(), 0);
        assert_eq!(y.is_mut(), false);
        assert_eq!(y.is_relocated(), false);
        assert_eq!(y.is_aliased(), false);
        assert_eq!(unsafe { x.as_ptr_unchecked() }, unsafe { y.as_ptr_unchecked() });

        let z = y.to_hard();
        mem::drop(x);
        mem::drop(y);
        assert_eq!(pack.live(), 1);
        assert_eq!(pack.used(), 24);
        assert_eq!(pack.free(), 4032);
        assert_eq!(z.hard_count(), 1);
        assert_eq!(z.soft_count(), 0);
        assert_eq!(z.ref_count(), 0);
        assert_eq!(z.is_mut(), false);
        assert_eq!(z.is_relocated(), false);
        assert_eq!(z.is_aliased(), false);
    }
    assert_eq!(pack.live(), 0);
    assert_eq!(pack.used(), 0);
    assert_eq!(pack.free(), 4064);
}

#[test]
fn test_hard_box_into_soft() {
    static mut TEST_AREA: [u8; 4096] = [0; 4096];
    let pack = Pack::new(unsafe { Block::from_slice(&mut TEST_AREA) });

    assert_eq!(pack.live(), 0);
    assert_eq!(pack.used(), 0);
    assert_eq!(pack.free(), 4064);
    {
        let x = HardBox::hold_new(pack, 5usize);
        assert_eq!(pack.live(), 1);
        assert_eq!(pack.used(), 24);
        assert_eq!(pack.free(), 4032);
        assert_eq!(x.hard_count(), 1);
        assert_eq!(x.soft_count(), 0);
        assert_eq!(x.ref_count(), 0);
        assert_eq!(x.is_mut(), false);
        assert_eq!(x.is_relocated(), false);
        assert_eq!(x.is_aliased(), false);

        let x = x.into_soft();
        assert_eq!(pack.live(), 1);
        assert_eq!(pack.used(), 24);
        assert_eq!(pack.free(), 4032);
        assert_eq!(x.hard_count(), 0);
        assert_eq!(x.soft_count(), 1);
        assert_eq!(x.ref_count(), 0);
        assert_eq!(x.is_mut(), false);
        assert_eq!(x.is_relocated(), false);
        assert_eq!(x.is_aliased(), false);

        assert_eq!(SoftBox::try_to_hard(&x).err().unwrap(), ArcError::Cleared);
    }
    assert_eq!(pack.live(), 0);
    assert_eq!(pack.used(), 0);
    assert_eq!(pack.free(), 4064);
}

#[test]
fn test_hard_box_to_ref() {
    static mut TEST_AREA: [u8; 4096] = [0; 4096];
    let pack = Pack::new(unsafe { Block::from_slice(&mut TEST_AREA) });

    assert_eq!(pack.live(), 0);
    assert_eq!(pack.used(), 0);
    assert_eq!(pack.free(), 4064);
    {
        let x = HardBox::hold_new(pack, 5usize);
        assert_eq!(pack.live(), 1);
        assert_eq!(pack.used(), 24);
        assert_eq!(pack.free(), 4032);
        assert_eq!(x.hard_count(), 1);
        assert_eq!(x.soft_count(), 0);
        assert_eq!(x.ref_count(), 0);
        assert_eq!(x.is_mut(), false);
        assert_eq!(x.is_relocated(), false);
        assert_eq!(x.is_aliased(), false);

        let y = x.to_ref();
        assert_eq!(pack.live(), 1);
        assert_eq!(pack.used(), 24);
        assert_eq!(pack.free(), 4032);
        assert_eq!(x.hard_count(), 2);
        assert_eq!(x.soft_count(), 0);
        assert_eq!(x.ref_count(), 1);
        assert_eq!(x.is_mut(), false);
        assert_eq!(x.is_relocated(), false);
        assert_eq!(x.is_aliased(), true);
        assert_eq!(RefBox::hard_count(&y), 2);
        assert_eq!(RefBox::soft_count(&y), 0);
        assert_eq!(RefBox::ref_count(&y), 1);
        assert_eq!(unsafe { x.as_ptr_unchecked() }, unsafe { RefBox::as_ptr_unchecked(&y) });
        assert_eq!(*y, 5);

        mem::drop(y);
        assert_eq!(pack.live(), 1);
        assert_eq!(pack.used(), 24);
        assert_eq!(pack.free(), 4032);
        assert_eq!(x.hard_count(), 1);
        assert_eq!(x.soft_count(), 0);
        assert_eq!(x.ref_count(), 0);
        assert_eq!(x.is_mut(), false);
        assert_eq!(x.is_relocated(), false);
        assert_eq!(x.is_aliased(), false);
    }
    assert_eq!(pack.live(), 0);
    assert_eq!(pack.used(), 0);
    assert_eq!(pack.free(), 4064);
}

#[test]
fn test_hard_box_into_ref() {
    static mut TEST_AREA: [u8; 4096] = [0; 4096];
    let pack = Pack::new(unsafe { Block::from_slice(&mut TEST_AREA) });

    assert_eq!(pack.live(), 0);
    assert_eq!(pack.used(), 0);
    assert_eq!(pack.free(), 4064);
    {
        let x = HardBox::hold_new(pack, 5usize);
        assert_eq!(pack.live(), 1);
        assert_eq!(pack.used(), 24);
        assert_eq!(pack.free(), 4032);
        assert_eq!(x.hard_count(), 1);
        assert_eq!(x.soft_count(), 0);
        assert_eq!(x.ref_count(), 0);
        assert_eq!(x.is_mut(), false);
        assert_eq!(x.is_relocated(), false);
        assert_eq!(x.is_aliased(), false);

        let x = x.into_ref();
        assert_eq!(pack.live(), 1);
        assert_eq!(pack.used(), 24);
        assert_eq!(pack.free(), 4032);
        assert_eq!(RefBox::hard_count(&x), 1);
        assert_eq!(RefBox::soft_count(&x), 0);
        assert_eq!(RefBox::ref_count(&x), 1);
        assert_eq!(*x, 5);

        let x = RefBox::into_hard(x);
        assert_eq!(pack.live(), 1);
        assert_eq!(pack.used(), 24);
        assert_eq!(pack.free(), 4032);
        assert_eq!(x.hard_count(), 1);
        assert_eq!(x.soft_count(), 0);
        assert_eq!(x.ref_count(), 0);
        assert_eq!(x.is_mut(), false);
        assert_eq!(x.is_relocated(), false);
        assert_eq!(x.is_aliased(), false);
    }
    assert_eq!(pack.live(), 0);
    assert_eq!(pack.used(), 0);
    assert_eq!(pack.free(), 4064);
}

#[test]
fn test_hard_box_to_mut() {
    static mut TEST_AREA: [u8; 4096] = [0; 4096];
    let pack = Pack::new(unsafe { Block::from_slice(&mut TEST_AREA) });

    assert_eq!(pack.live(), 0);
    assert_eq!(pack.used(), 0);
    assert_eq!(pack.free(), 4064);
    {
        let x = HardBox::hold_new(pack, 5usize);
        assert_eq!(pack.live(), 1);
        assert_eq!(pack.used(), 24);
        assert_eq!(pack.free(), 4032);
        assert_eq!(x.hard_count(), 1);
        assert_eq!(x.soft_count(), 0);
        assert_eq!(x.ref_count(), 0);
        assert_eq!(x.is_mut(), false);
        assert_eq!(x.is_relocated(), false);
        assert_eq!(x.is_aliased(), false);

        let mut y = unsafe { x.to_mut() };
        assert_eq!(pack.live(), 1);
        assert_eq!(pack.used(), 24);
        assert_eq!(pack.free(), 4032);
        assert_eq!(x.hard_count(), 2);
        assert_eq!(x.soft_count(), 0);
        assert_eq!(x.ref_count(), 0);
        assert_eq!(x.is_mut(), true);
        assert_eq!(x.is_relocated(), false);
        assert_eq!(x.is_aliased(), true);
        assert_eq!(MutBox::hard_count(&y), 2);
        assert_eq!(MutBox::soft_count(&y), 0);
        assert_eq!(MutBox::ref_count(&y), 0);
        assert_eq!(unsafe { x.as_ptr_unchecked() }, unsafe { MutBox::as_ptr_unchecked(&y) });
        assert_eq!(*y, 5);

        *y = 9;
        assert_eq!(*y, 9);

        mem::drop(y);
        assert_eq!(pack.live(), 1);
        assert_eq!(pack.used(), 24);
        assert_eq!(pack.free(), 4032);
        assert_eq!(x.hard_count(), 1);
        assert_eq!(x.soft_count(), 0);
        assert_eq!(x.ref_count(), 0);
        assert_eq!(x.is_mut(), false);
        assert_eq!(x.is_relocated(), false);
        assert_eq!(x.is_aliased(), false);
    }
    assert_eq!(pack.live(), 0);
    assert_eq!(pack.used(), 0);
    assert_eq!(pack.free(), 4064);
}

#[test]
fn test_hard_box_into_mut() {
    static mut TEST_AREA: [u8; 4096] = [0; 4096];
    let pack = Pack::new(unsafe { Block::from_slice(&mut TEST_AREA) });

    assert_eq!(pack.live(), 0);
    assert_eq!(pack.used(), 0);
    assert_eq!(pack.free(), 4064);
    {
        let x = HardBox::hold_new(pack, 5usize);
        assert_eq!(pack.live(), 1);
        assert_eq!(pack.used(), 24);
        assert_eq!(pack.free(), 4032);
        assert_eq!(x.hard_count(), 1);
        assert_eq!(x.soft_count(), 0);
        assert_eq!(x.ref_count(), 0);
        assert_eq!(x.is_mut(), false);
        assert_eq!(x.is_relocated(), false);
        assert_eq!(x.is_aliased(), false);

        let mut x = unsafe { x.into_mut() };
        assert_eq!(pack.live(), 1);
        assert_eq!(pack.used(), 24);
        assert_eq!(pack.free(), 4032);
        assert_eq!(MutBox::hard_count(&x), 1);
        assert_eq!(MutBox::soft_count(&x), 0);
        assert_eq!(MutBox::ref_count(&x), 0);
        assert_eq!(*x, 5);

        *x = 9;
        assert_eq!(*x, 9);

        let x = MutBox::into_hard(x);
        assert_eq!(pack.live(), 1);
        assert_eq!(pack.used(), 24);
        assert_eq!(pack.free(), 4032);
        assert_eq!(x.hard_count(), 1);
        assert_eq!(x.soft_count(), 0);
        assert_eq!(x.ref_count(), 0);
        assert_eq!(x.is_mut(), false);
        assert_eq!(x.is_relocated(), false);
        assert_eq!(x.is_aliased(), false);
    }
    assert_eq!(pack.live(), 0);
    assert_eq!(pack.used(), 0);
    assert_eq!(pack.free(), 4064);
}

#[test]
fn test_hard_box_stow_into() {
    static mut TEST_AREA0: [u8; 4096] = [0; 4096];
    let pack0 = Pack::new(unsafe { Block::from_slice(&mut TEST_AREA0) });
    static mut TEST_AREA1: [u8; 4096] = [0; 4096];
    let pack1 = Pack::new(unsafe { Block::from_slice(&mut TEST_AREA1) });

    assert_eq!(pack0.live(), 0);
    assert_eq!(pack0.used(), 0);
    assert_eq!(pack0.free(), 4064);
    assert_eq!(pack1.live(), 0);
    assert_eq!(pack1.used(), 0);
    assert_eq!(pack1.free(), 4064);
    {
        let x = HardBox::hold_new(pack0, 5usize);
        let y = x.clone();
        assert_eq!(pack0.live(), 1);
        assert_eq!(pack0.used(), 24);
        assert_eq!(pack0.free(), 4032);
        assert_eq!(pack1.live(), 0);
        assert_eq!(pack1.used(), 0);
        assert_eq!(pack1.free(), 4064);
        assert_eq!(x.hard_count(), 2);
        assert_eq!(x.soft_count(), 0);
        assert_eq!(x.ref_count(), 0);
        assert_eq!(x.is_mut(), false);
        assert_eq!(x.is_relocated(), false);
        assert_eq!(x.is_aliased(), false);
        assert_eq!(y.hard_count(), 2);
        assert_eq!(y.soft_count(), 0);
        assert_eq!(y.ref_count(), 0);
        assert_eq!(y.is_mut(), false);
        assert_eq!(y.is_relocated(), false);
        assert_eq!(y.is_aliased(), false);
        assert_eq!(unsafe { x.as_ptr_unchecked() }, unsafe { y.as_ptr_unchecked() });

        let x: HardBox<usize> = x.stow_into(pack1);
        assert_eq!(pack0.live(), 1);
        assert_eq!(pack0.used(), 24);
        assert_eq!(pack0.free(), 4032);
        assert_eq!(pack1.live(), 1);
        assert_eq!(pack1.used(), 24);
        assert_eq!(pack1.free(), 4032);
        assert_eq!(x.hard_count(), 2);
        assert_eq!(x.soft_count(), 0);
        assert_eq!(x.ref_count(), 0);
        assert_eq!(x.is_mut(), false);
        assert_eq!(x.is_relocated(), false);
        assert_eq!(x.is_aliased(), false);
        assert_eq!(y.hard_count(), 1);
        assert_eq!(y.soft_count(), 0);
        assert_eq!(y.ref_count(), 0);
        assert_eq!(y.is_mut(), false);
        assert_eq!(y.is_relocated(), true);
        assert_eq!(y.is_aliased(), false);
        assert_eq!(unsafe { x.as_ptr_unchecked() == y.as_ptr_unchecked() }, false);

        let y: HardBox<usize> = y.stow_into(pack1);
        assert_eq!(pack0.live(), 0);
        assert_eq!(pack0.used(), 0);
        assert_eq!(pack0.free(), 4064);
        assert_eq!(pack1.live(), 1);
        assert_eq!(pack1.used(), 24);
        assert_eq!(pack1.free(), 4032);
        assert_eq!(x.hard_count(), 2);
        assert_eq!(x.soft_count(), 0);
        assert_eq!(x.ref_count(), 0);
        assert_eq!(x.is_mut(), false);
        assert_eq!(x.is_relocated(), false);
        assert_eq!(x.is_aliased(), false);
        assert_eq!(y.hard_count(), 2);
        assert_eq!(y.soft_count(), 0);
        assert_eq!(y.ref_count(), 0);
        assert_eq!(y.is_mut(), false);
        assert_eq!(y.is_relocated(), false);
        assert_eq!(y.is_aliased(), false);
        assert_eq!(unsafe { x.as_ptr_unchecked() }, unsafe { y.as_ptr_unchecked() });
    }
    assert_eq!(pack0.live(), 0);
    assert_eq!(pack0.used(), 0);
    assert_eq!(pack0.free(), 4064);
    assert_eq!(pack1.live(), 0);
    assert_eq!(pack1.used(), 0);
    assert_eq!(pack1.free(), 4064);
}
