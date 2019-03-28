use paging::{ActivePageTable, Page, VirtualAddress};
use paging::entry::EntryFlags;
use paging::mapper::MapperFlushAll;

#[cfg(not(feature="slab"))]
pub use self::linked_list::Allocator;

#[cfg(feature="slab")]
pub use self::slab::Allocator;

#[cfg(not(feature="slab"))]
mod linked_list;

#[cfg(feature="slab")]
mod slab;

unsafe fn map_heap(active_table: &mut ActivePageTable, offset: usize, size: usize) {
    let mut flush_all = MapperFlushAll::new();

    let heap_start_page = Page::containing_address(VirtualAddress::new(offset));
    let heap_end_page = Page::containing_address(VirtualAddress::new(offset + size-1));
    for page in Page::range_inclusive(heap_start_page, heap_end_page) {
        let result = active_table.map(page, EntryFlags::PRESENT | EntryFlags::GLOBAL | EntryFlags::WRITABLE | EntryFlags::NO_EXECUTE);
        flush_all.consume(result);
    }

    flush_all.flush(active_table);
}

pub unsafe fn init(active_table: &mut ActivePageTable) {
    let offset = ::KERNEL_HEAP_OFFSET;
    let size = ::KERNEL_HEAP_SIZE;

    // Map heap pages
    map_heap(active_table, offset, size);

    // Initialize global heap
    Allocator::init(offset + core::mem::size_of::<u64>(), size);


    // read and print marker
    const MARKER: u64 = 0x0000ACCE55ED0000;
    let marker_ptr = offset as *mut u64;
    let marker_value = marker_ptr.read();
    if marker_value == MARKER {
        println!("\n\n\nTHIS IS A HOT REBOOT\n\n\n");
    } else {
        println!("\n\n\nTHIS IS A NORMAL BOOT\n(marker value {:#x} != {:#x}\n\n", marker_value, MARKER);
        marker_ptr.write(MARKER);
    }
}
