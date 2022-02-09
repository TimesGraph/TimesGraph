#![feature(test)]

extern crate test;
extern crate tg_mem;

use test::Bencher;
use tg_mem::block::Block;
use tg_mem::alloc::{Slab, Pool};
use tg_mem::lease::{RawBox, RawBuf};

#[bench]
fn bench_pool_alloc_dealloc(bench: &mut Bencher) {
    static mut TEST_HUNK: [u8; 4096] = [0; 4096];
    let slab = Slab::new(unsafe { Block::from_slice(&mut TEST_HUNK) }, 256);
    let pool = &Pool::new(&slab);

    let mut n: usize = 0;
    bench.iter(|| {
        let x = RawBox::hold_new(pool, n);
        n = n.wrapping_add(*x);
    });
}

#[bench]
fn bench_pool_alloc_dealloc_1mib(bench: &mut Bencher) {
    static mut TEST_HUNK: [u8; 1024*1024] = [0; 1024*1024];
    let mut k: usize = 0;
    bench.iter(|| {
        let slab = Slab::new(unsafe { Block::from_slice(&mut TEST_HUNK) }, 4096);
        let pool = &Pool::new(&slab);

        k = 0;
        let mut n: usize = 0;
        let mut x = RawBox::hold_new(pool, n);
        while k < 32768 {
            k = k.wrapping_add(1);
            n = n.wrapping_add(*x);
            x = RawBox::hold_new(pool, n);
        }
    });
}

#[bench]
fn bench_pool_alloc_dealloc_bufs(bench: &mut Bencher) {
    static mut TEST_HUNK: [u8; 8*1024*1024] = [0; 8*1024*1024];
    let mut k: usize = 0;
    bench.iter(|| {
        let slab = Slab::new(unsafe { Block::from_slice(&mut TEST_HUNK) }, 4096);
        let pool = &Pool::new(&slab);

        k = 0;
        let mut n: usize = 1;
        let mut _x = RawBuf::<usize>::hold_cap(pool, n);
        while k < 32768 {
            k = k.wrapping_add(1);
            n = n.wrapping_add(1) % 32;
            _x = RawBuf::hold_cap(pool, n);
        }
    });
}
