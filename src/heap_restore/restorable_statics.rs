use super::allocator::RestorableAllocator;
use core::alloc::{GlobalAlloc, Layout};

pub struct RestorableStatics {
    first: Option<&'static mut RestorableStatic>,
}

pub struct RestorableStatic {
    static_addr: usize,
    value: *mut u8,
    next: Option<&'static mut RestorableStatic>,
}

impl RestorableStatics {
    pub fn new() -> Self {
        RestorableStatics {
            first: None,
        }
    }

    pub unsafe fn register_static<T>(&mut self, static_addr: usize, init_object: impl FnOnce()-> T) -> &'static mut T
        where T: 'static + Restorable,
    {
        let mut slot = &mut self.first;
        loop {
            if let Some(s) = slot {
                if s.static_addr == static_addr {
                    // previous state of variable present -> try to restore it
                    let value = s.value as *mut T;
                    if !value.is_consistent() {
                        // TODO: let caller decide what to do on inconsistent value (reboot (normally), re-init value)
                        // TODO: override the inconsistent value: value.write(init_object()); (problem: garbage collection of previous value?)
                        panic!("restored value inconsistent");
                    }
                    return &mut *value;
                }
            }
            if slot.is_none() {
                // no previous state of variable found -> initialize it at the end

                // allocate object on the restorable heap
                let object_ptr = unsafe {RestorableAllocator.alloc(Layout::new::<T>()) } as *mut T;
                unsafe { object_ptr.write(init_object()) };

                // allocate a new slot on the restorable heap
                let restorable_static = RestorableStatic {
                    static_addr,
                    value: object_ptr as *mut u8,
                    next: None,
                };
                let restorable_static_ptr = unsafe { RestorableAllocator.alloc(Layout::new::<RestorableStatic>()) } as *mut RestorableStatic;
                unsafe { restorable_static_ptr.write(restorable_static) };

                // append the new slot to the list
                *slot = Some(unsafe {&mut *restorable_static_ptr});

                // return a reference to the created value
                return unsafe { &mut *object_ptr }
            }
            if let Some(s) = {slot} {
                slot = &mut s.next;
            } else {
                unreachable!();
            }
        }
    }
}

unsafe impl Send for RestorableStatics {}

pub trait Restorable: RestoreSafe + ConsistencyCheckable {}

impl <T> Restorable for T where T: RestoreSafe + ConsistencyCheckable {}

pub unsafe trait ConsistencyCheckable {
    fn is_consistent(self: *mut Self) -> bool;
}

pub unsafe auto trait RestoreSafe {}

impl<T> !RestoreSafe for *mut T {}
impl<T> !RestoreSafe for *const T {}
impl<'a, T> !RestoreSafe for &'a T {}
impl<'a, T> !RestoreSafe for &'a mut T {}
