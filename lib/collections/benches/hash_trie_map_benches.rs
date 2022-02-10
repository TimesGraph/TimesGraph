#![feature(test)]

extern crate test;
extern crate tg_mem;
extern crate tg_collections;

use test::Bencher;
use tg_mem::block::Block;
use tg_mem::alloc::{Slab, Pool, StowFrom};
use tg_collections::hash_trie::HashTrieMap;

macro_rules! bench_update {
    ($bench:expr, $hold:expr, $n:expr) => ({
        let mut xs = HashTrieMap::<i32, i32>::hold_new($hold);
        for k in 0..$n {
            xs.insert(k, -k).unwrap();
        }
        let mut ys = HashTrieMap::stow_from(xs, $hold);
        let mut i = 0;
        $bench.iter(|| {
            ys.insert(i, i).unwrap();
            i = i.wrapping_add(1) % $n;
        });
    });
}

#[bench]
fn bench_update_depth0(bench: &mut Bencher) {
    static mut TEST_HUNK: [u8; 16*1024*1024] = [0; 16*1024*1024];
    let slab = Slab::new(unsafe { Block::from_slice(&mut TEST_HUNK) }, 4096);
    let pool = &Pool::new(&slab);

    bench_update!(bench, pool, 1 << 3);
}

#[bench]
fn bench_update_depth1(bench: &mut Bencher) {
    static mut TEST_HUNK: [u8; 16*1024*1024] = [0; 16*1024*1024];
    let slab = Slab::new(unsafe { Block::from_slice(&mut TEST_HUNK) }, 4096);
    let pool = &Pool::new(&slab);

    bench_update!(bench, pool, 1 << 5);
}

#[bench]
fn bench_update_depth2(bench: &mut Bencher) {
    static mut TEST_HUNK: [u8; 16*1024*1024] = [0; 16*1024*1024];
    let slab = Slab::new(unsafe { Block::from_slice(&mut TEST_HUNK) }, 4096);
    let pool = &Pool::new(&slab);

    bench_update!(bench, pool, 1 << 10);
}

#[bench]
fn bench_update_depth3(bench: &mut Bencher) {
    static mut TEST_HUNK: [u8; 16*1024*1024] = [0; 16*1024*1024];
    let slab = Slab::new(unsafe { Block::from_slice(&mut TEST_HUNK) }, 4096);
    let pool = &Pool::new(&slab);

    bench_update!(bench, pool, 1 << 15);
}

#[bench]
fn bench_update_depth4(bench: &mut Bencher) {
    static mut TEST_HUNK: [u8; 256*1024*1024] = [0; 256*1024*1024];
    let slab = Slab::new(unsafe { Block::from_slice(&mut TEST_HUNK) }, 4096);
    let pool = &Pool::new(&slab);

    bench_update!(bench, pool, 1 << 20);
}
