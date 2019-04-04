use core::alloc::{AllocErr, GlobalAlloc, Layout};
use core::ptr::NonNull;
use linked_list_allocator::Heap;
use spin::Mutex;

use paging::ActivePageTable;

static HEAP: Mutex<Option<&'static mut Heap>> = Mutex::new(None);


pub struct RestorableAllocator;

impl RestorableAllocator {
    pub unsafe fn new_heap(offset: usize, size: usize) -> Heap {
        Heap::new(offset, size)
    }

    pub fn init(heap: &'static mut Heap) {
        *HEAP.lock() = Some(heap);
    }
}

unsafe impl GlobalAlloc for RestorableAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        loop {
            let res = if let Some(ref mut heap) = *HEAP.lock() {
                heap.allocate_first_fit(layout)
            } else {
                panic!("__rust_allocate: restorable heap not initialized");
            };

            match res {
                Err(AllocErr) => {
                    let size = if let Some(ref heap) = *HEAP.lock() {
                        heap.size()
                    } else {
                        panic!("__rust_allocate: restorable heap not initialized");
                    };

                    crate::allocator::map_heap(&mut ActivePageTable::new(), ::KERNEL_RESTORABLE_HEAP_OFFSET + size, ::KERNEL_RESTORABLE_HEAP_SIZE);

                    if let Some(ref mut heap) = *HEAP.lock() {
                        heap.extend(::KERNEL_RESTORABLE_HEAP_SIZE);
                    } else {
                        panic!("__rust_allocate: restorable heap not initialized");
                    }
                },
                other => return other.ok().map_or(0 as *mut u8, |allocation| allocation.as_ptr()),
            }
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if let Some(ref mut heap) = *HEAP.lock() {
            heap.deallocate(NonNull::new_unchecked(ptr), layout)
        } else {
            panic!("__rust_deallocate: restorable heap not initialized");
        }
    }
}
