#![feature(test)]

extern crate test;
extern crate tg_mem;

use test::Bencher;
use tg_mem::block::Block;
use tg_mem::alloc::Pack;
use tg_mem::lease::RawBox;

#[bench]
fn bench_pack_alloc_dealloc(bench: &mut Bencher) {
    static mut TEST_AREA: [u8; 4096] = [0; 4096];
    let pack = Pack::new(unsafe { Block::from_slice(&mut TEST_AREA) });

    let mut n: usize = 0;
    bench.iter(|| {
        let x = RawBox::hold_new(pack, n);
        n = n.wrapping_add(*x);
    });
}
