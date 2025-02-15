/* ╔═════════════════════════════════════════════════════════════════════════╗
   ║ Module: allocator                                                       ║
   ╟─────────────────────────────────────────────────────────────────────────╢
   ║ Descr.: Imnplementing functions for the heap allocator used by the rust ║
   ║         compiler. The function 'init' must be called early in 'startup'.║
   ║                                                                         ║ 
   ║         Memory-Laylout                                                  ║
   ║            0x0         real mode & bios stuff       	                 ║
   ║            0x100000    our OS image, including global variables         ║ 
   ║            heap_start  start address of our heap, see 'startup.rs'      ║ 
   ║                                                                         ║ 
   ║         Remarks                                                         ║
   ║            - Lowest loading address for grub is 1 MB                    ║ 
   ╟─────────────────────────────────────────────────────────────────────────╢
   ║ Author: Philipp Oppermann                                               ║
   ║         https://os.phil-opp.com/allocator-designs/                      ║
   ╚═════════════════════════════════════════════════════════════════════════╝
*/
use alloc::alloc::Layout;

use crate::consts;
use crate::kernel::allocator::list::LinkedListAllocator;

pub mod list;

#[global_allocator]
static ALLOCATOR: Locked<LinkedListAllocator> = Locked::new(LinkedListAllocator::new());


/**
 Description: Initialization of the allocator. Must be called early in 'startup'.
*/
pub fn init(heap_start: usize, heap_size: usize) {
    unsafe {
        ALLOCATOR.lock().init(heap_start, heap_size);
    }
}

/**
 Description: Allocates memory from the heap. 
              Compiler generates code calling this function.
*/
pub fn alloc(layout: Layout) -> *mut u8 {
    unsafe {
        ALLOCATOR.lock().alloc(layout)
    }
}

/**
 Description: Deallocates memory from the heap. 
              Compiler generates code calling this function.
*/
pub fn dealloc(ptr: *mut u8, layout: Layout) {
    unsafe {
        ALLOCATOR.lock().dealloc(ptr, layout)
    }
}

/**
 Description: Dump heap free list. Must be called by own program.
              Can be used for debugging the heap allocator. 
*/
pub fn dump_free_list() {
   ALLOCATOR.lock().dump_free_list();
}

/**
 Description: A wrapper around spin::Mutex to permit trait implementations
              Required for implementing `GlobalAlloc` in `bump.rs` and 
             `list.rs`. Can be used for debugging the heap allocator. 
*/
pub struct Locked<A> {
    inner: spin::Mutex<A>,
}

impl<A> Locked<A> {
    pub const fn new(inner: A) -> Self {
        Locked {
            inner: spin::Mutex::new(inner),
        }
    }

    pub fn lock(&self) -> spin::MutexGuard<A> {
        self.inner.lock()
    }
}

/**
 Description: Helper function used in in `bump.rs` and `list.rs`. 
              Rust requires pointers to be aligned.
*/
fn align_up(addr: usize, align: usize) -> usize {
    let remainder = addr % align;
    if remainder == 0 {
        addr // addr already aligned
    } else {
        addr - remainder + align
    }
}

#[alloc_error_handler]
pub fn rust_oom(layout: Layout) -> ! {
    kprintln!(
        "[!!!OOM!!!] Memory allocation of {} bytes failed",
        layout.size()
    );

    loop {}
}
