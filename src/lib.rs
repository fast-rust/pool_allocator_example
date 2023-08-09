#![feature(thread_local)]

use std::alloc::{GlobalAlloc, Layout, System};

struct PoolAllocator;

// A chunk is either a pointer to the next free chunk
// or some bytes.
#[repr(C)]
union Chunk<const SIZE : usize> {
    free_ptr: * mut Chunk<SIZE>,
    mem: [u8; SIZE],
}

// Grab another N elements (we could use mmap for this).
unsafe fn init_pool<const SIZE : usize>(
    pool: &mut *mut Chunk<SIZE>
) {
    const N : usize = 4096;
    let mem = System.alloc(
        Layout::from_size_align(
            SIZE * 4096, 32).unwrap()
    ) as * mut Chunk<SIZE>;
    for i in 0..N {
        let chunk_ref = mem
            .offset(i as isize)
            .as_mut().unwrap();
        let free_ptr = mem
            .offset(i as isize + 1);
        chunk_ref.free_ptr = if i == N-1 {
            std::ptr::null_mut()
        } else {
            free_ptr
        };
    }
    *pool = mem;
}

// Thread local variable, so no atomics needed.
//
// Although you may be tempted, never ever use Ordering::Relaxed
// to guard a memory allocation.
#[thread_local]
static mut POOL32: * mut Chunk<32> = std::ptr::null_mut();

#[thread_local]
static mut ENABLE : bool = false;

unsafe impl<'a> GlobalAlloc for PoolAllocator {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if ENABLE && layout.size() <= 32
                && layout.align() <= 32 {
            if POOL32.is_null() {
                init_pool(&mut POOL32);
            }
            let res = POOL32 as * mut u8;
            POOL32 = POOL32.as_mut().unwrap().free_ptr;
            // eprintln!("alloc {layout:?} @ {res:?}");
            res
        } else {
            System.alloc(layout)
        }
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if ENABLE && layout.size() <= 32
                && layout.align() <= 32 && !ptr.is_null() {
            // eprintln!("dealloc {layout:?} @ {ptr:?}");
            let chunk = ptr as * mut Chunk<32>;
            let chunk_ref = chunk.as_mut().unwrap();
            chunk_ref.free_ptr = POOL32;
            POOL32 = chunk;
        } else {
            System.dealloc(ptr, layout);
        }
    }
}

#[global_allocator]
static GLOBAL: PoolAllocator = PoolAllocator;

#[test]
fn test_pool() {
    unsafe { ENABLE = true; }
    let _ = Box::new(1);
    for _ in 0..10 {
        let mut time = [0; 10];
        let _ = vec![0; 1000000];
        for i in 0..10 {
            let t0 = std::time::Instant::now();
            let _ = vec![1, 2, 3];
            let _ = vec![1, 2, 3];
            let _ = vec![1, 2, 3];
            time[i] = t0.elapsed().as_nanos();
        }
        println!("pool: {time:?}");
    }
    unsafe { ENABLE =false; }
}

#[test]
fn test_malloc() {
    let _ = Box::new(1);
    for _ in 0..10 {
        let mut time = [0; 10];
        let _ = vec![0; 1000000];
        for i in 0..10 {
            let t0 = std::time::Instant::now();
            let _ = vec![1, 2, 3];
            let _ = vec![1, 2, 3];
            let _ = vec![1, 2, 3];
            time[i] = t0.elapsed().as_nanos();
        }
        println!("malloc: {time:?}");
    }
}
