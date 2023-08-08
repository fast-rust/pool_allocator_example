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

// Grab another 4096 elements (we could use mmap for this).
unsafe fn init_pool<const SIZE : usize>(
    pool: &mut *mut Chunk<SIZE>
) {
    let mem = System.alloc(
        Layout::from_size_align(
            SIZE * 4096, 32).unwrap());
    *pool = mem as * mut Chunk<SIZE>;
}

// Thread local variable, so not atomics needed.
// Although you may be tempted, never ever use Ordering::Relaxed
// to guard a memory allocation.
#[thread_local]
static mut POOL32: * mut Chunk<32> = std::ptr::null_mut();
static mut ENABLE : bool = false;

unsafe impl<'a> GlobalAlloc for PoolAllocator {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if ENABLE && layout.size() <= 32
                && layout.align() <= 32 {
            eprintln!("alloc {layout:?}");
            if POOL32.is_null() {
                init_pool(&mut POOL32);
            }
            let res = POOL32 as * mut u8;
            res
        } else {
            System.alloc(layout)
        }
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if ENABLE && layout.size() <= 32
                && layout.align() <= 32 && !ptr.is_null() {
            eprintln!("dealloc {layout:?}");
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
fn test_me() {
    unsafe { ENABLE = true; }
    {
        let s = vec![1, 2, 3];
    }
    unsafe { ENABLE =false; }
}
