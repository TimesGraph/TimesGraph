#[macro_use]
extern crate tg_mem;

use tg_mem::block::Layout;
use tg_mem::alloc::Heap;

#[test]
fn test_addr_space_init() {
    addr_space! {
        pub heap GLOBAL = [4*4096];
    }
    println!("GLOBAL.size: {}", GLOBAL.size());
    println!("GLOBAL.live: {}", GLOBAL.live());
    println!("GLOBAL.used: {}", GLOBAL.used());
    let _x = unsafe { GLOBAL.alloc(Layout::for_type::<usize>()) }.unwrap();
    println!("Allocated ptr: {:p}; size: {}", _x.as_ptr(), _x.size());
    println!("GLOBAL.live: {}", GLOBAL.live());
    println!("GLOBAL.used: {}", GLOBAL.used());
    let _y = unsafe { GLOBAL.alloc(Layout::for_type::<usize>()) }.unwrap();
    println!("Allocated ptr: {:p}; size: {}", _y.as_ptr(), _y.size());
    println!("GLOBAL.live: {}", GLOBAL.live());
    println!("GLOBAL.used: {}", GLOBAL.used());
}
