use paging::ActivePageTable;
use self::allocator::RestorableAllocator;
use self::restorable_statics::{RestorableStatics, ConsistencyCheckable};
use spin::Mutex;
use core::{mem, alloc::{Layout, GlobalAlloc}, cell::UnsafeCell};

mod allocator;
mod restorable_statics;

static RESTORABLE_STATICS: Mutex<Option<&'static mut RestorableStatics>> = Mutex::new(None);

pub unsafe fn init(active_table: &mut ActivePageTable) {
    let offset = ::KERNEL_RESTORABLE_HEAP_OFFSET;
    let size = ::KERNEL_RESTORABLE_HEAP_SIZE;

    // Map heap pages
    crate::allocator::map_heap(active_table, offset, size); 

    let marker_ptr = offset as *mut u64;
    let allocator_ptr = marker_ptr.offset(1) as *mut linked_list_allocator::Heap;
    let statics_ptr = allocator_ptr.offset(1) as *mut restorable_statics::RestorableStatics;
    let heap_offset = statics_ptr.offset(1) as usize;

    // read and print marker
    const MARKER: u64 = 0x0000ACCE55ED0000;
    let marker_value = marker_ptr.read();
    if marker_value == MARKER {
        println!("\n\n\nTHIS IS A HOT REBOOT\n\n\n");
    } else {
        println!("\n\n\nTHIS IS A NORMAL BOOT\n(marker value {:#x} != {:#x}\n\n", marker_value, MARKER);

        let allocator = RestorableAllocator::new_heap(heap_offset, ::KERNEL_RESTORABLE_HEAP_SIZE);

        marker_ptr.write(MARKER);
        allocator_ptr.write(allocator);
        statics_ptr.write(restorable_statics::RestorableStatics::new());
    }

    RestorableAllocator::init(&mut *allocator_ptr);

    println!("Allocation: {:#?}\n\n", RestorableAllocator.alloc(Layout::new::<u32>())); // TODO remove

    *RESTORABLE_STATICS.lock() = Some(&mut *statics_ptr);

    println!("RESTORE_ME: {}", RESTORE_ME.get());
    RESTORE_ME.set(103);
    println!("\n\n\n");
}

use core::ops::Deref;

static RESTORE_ME: RestoreTest = RestoreTest;

struct RestoreTest;

impl Deref for RestoreTest {
    type Target = RestoreTestValue;

    fn deref(&self) -> &Self::Target {
        fn init() -> RestoreTestValue {
            // place the static initialization code here
            RestoreTestValue::new(42)
        }

        static VALUE: spin::Once<&'static mut RestoreTestValue> = spin::Once::INIT;

        VALUE.call_once(|| {
            if let Some(ref mut restorable_statics) = *RESTORABLE_STATICS.lock() {
                unsafe {
                    restorable_statics.register_static(self as *const Self as usize, init)
                }
            } else {
                panic!("RESTORABLE_STATICS not initialized");
            }
        })
    }
}

struct RestoreTestValue {
    value: Mutex<u32>,
    copy: Mutex<[u8; mem::size_of::<Mutex<u32>>()]>,
}

unsafe impl ConsistencyCheckable for RestoreTestValue {
    fn is_consistent(self: *mut Self) -> bool {
        let s = unsafe { &*self };
        let value = Self::create_copy(&s.value);
        *s.copy.lock() == value
    }
}

impl RestoreTestValue {
    fn new(number: u32) -> Self {
        let value = spin::Mutex::new(number);
        let copy = Mutex::new(Self::create_copy(&value));
        RestoreTestValue { value, copy }
    }

    fn get(&self) -> u32 {
        *self.value.lock()
    }

    fn set(&self, value: u32) {
        *self.value.lock() = value;
        *self.copy.lock() = Self::create_copy(&self.value);
    }

    fn create_copy(value: &Mutex<u32>) -> [u8; mem::size_of::<Mutex<u32>>()] {
        unsafe { mem::transmute_copy(value) }
    }
}