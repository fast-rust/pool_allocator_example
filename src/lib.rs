#![feature(allocator_api)]
/// Note that this is not Send or Sync!
pub struct PoolAllocator<const SIZE : usize> {
    next_free: std::cell::UnsafeCell<* mut u8>,
}

impl<const SIZE : usize> PoolAllocator<SIZE> {
    pub fn new(n: usize, align: usize) -> Self {
        unsafe {
            assert!(SIZE % align == 0);
            assert!(SIZE >= std::mem::size_of::<* mut u8>());
            let layout = std::alloc::Layout::from_size_align(
                SIZE * n, align).unwrap();
            let mem = std::alloc::alloc(layout);
            if mem.is_null() { panic!("out of memory") }

            // put in the links.
            for i in 0..n {
                let ptr = mem.offset((i * SIZE) as isize);
                *(ptr as * mut * mut u8) = if i != n-1 {
                    ptr.offset(SIZE as isize)
                } else {
                    std::ptr::null_mut()
                };
            }
            Self {
                next_free: std::cell::UnsafeCell::new(mem)
            }
        }
    }
}

unsafe impl<const SIZE : usize> std::alloc::Allocator for PoolAllocator<SIZE> {
    fn allocate(&self, layout: std::alloc::Layout) -> Result<std::ptr::NonNull<[u8]>, std::alloc::AllocError> {
        unsafe {
            let next_free_ptr = self.next_free.get();
            let next_free = *next_free_ptr;
            if !next_free.is_null() && layout.size() == SIZE {
                let slice = std::slice::from_raw_parts_mut(next_free, SIZE);
                let next = *(next_free as * mut * mut u8);
                *next_free_ptr = next;
                Ok(std::ptr::NonNull::new(slice as * mut [u8]).unwrap())
            } else {
                Err(std::alloc::AllocError)
            }
        }
    }

    unsafe fn deallocate(&self, ptr: std::ptr::NonNull<u8>, layout: std::alloc::Layout) {
        // TODO:
        println!("free {:?}", ptr);
    }
}

#[cfg(test)]
mod test {

    #[test]
    fn test_pool_allocator() {
        use crate::PoolAllocator;

        let pool = PoolAllocator::<8>::new(4, 8);

        let b = Box::new_in(1_u64, &pool);
        println!("{:016x}", *b);

        let b = Box::new_in(1_u64, &pool);
        println!("{:016x}", *b);

        let b = Box::new_in(1_u64, &pool);
        println!("{:016x}", *b);

        let b = Box::new_in(1_u64, &pool);
        println!("{:016x}", *b);

        // This will fail.
        // let b = Box::new_in(1_u64, &pool);
        // println!("{:016x}", *b);
    }
}
