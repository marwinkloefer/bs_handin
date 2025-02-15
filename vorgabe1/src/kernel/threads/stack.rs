/* ╔═════════════════════════════════════════════════════════════════════════╗
   ║ Module: stack                                                           ║
   ╟─────────────────────────────────────────────────────────────────────────╢
   ║ Descr.: Allocating and deallocation memory for a stack.                 ║
   ╟─────────────────────────────────────────────────────────────────────────╢
   ║ Autor:  Michael Schoettner, 15.05.2023                                  ║
   ╚═════════════════════════════════════════════════════════════════════════╝
*/
use alloc::alloc::Layout;
use alloc::boxed::Box;
use core::fmt;

use crate::consts;
use crate::kernel::allocator;
use crate::kernel::cpu;
use crate::kernel::paging::frames::PhysAddr;
use crate::kernel::paging::pages::pg_mmap_user_stack;

#[repr(C)]
pub struct Stack {
    data: *mut u8,
    size: usize,
}

impl Stack {
    /*pub fn new(size: usize) -> Box<Stack> {
        // 64 bit alignment for stack
        let layout = unsafe { Layout::from_size_align_unchecked(size, consts::STACK_ALIGNMENT) };

        // alloc memory for stack and set ptr. to end of block - consts::STACK_ENTRY_SIZE
        let start = allocator::alloc(layout);
        let data = ((start as usize) + (size as usize) - consts::STACK_ENTRY_SIZE) as *mut u8;
        if data.is_null() {
            println!("Panic: failed in 'Stack::new'");
            cpu::halt();
        }

        kprintln!(
            "Stack::new, memory block = [0x{:x}; 0x{:x}]",
            start as usize,
            (data as usize + consts::STACK_ENTRY_SIZE)
        );

        Box::new(Stack { data, size })
    }*/
    
    //TODO
    pub fn new(size: usize, kernel_stack: bool, pml4_addr: PhysAddr) -> Box<Stack> {  
        if kernel_stack{ // wie zuvor für kernel
            // 64 bit alignment for stack
            let layout = unsafe { Layout::from_size_align_unchecked(size, consts::STACK_ALIGNMENT) };

            // alloc memory for stack and set ptr. to end of block - consts::STACK_ENTRY_SIZE
            let start = allocator::alloc(layout);
            let data = ((start as usize) + (size as usize) - consts::STACK_ENTRY_SIZE) as *mut u8;
            if data.is_null() {
                println!("Panic: failed in 'Stack::new::kernel_stack'");
                cpu::halt();
            }

            kprintln!(
                "Stack::new, memory block = [0x{:x}; 0x{:x}]",
                start as usize,
                (data as usize + consts::STACK_ENTRY_SIZE)
            );

            Box::new(Stack { data, size }) 
        } 
        else // für user thread muss mapping in pages erstellt werden
        { 
            // fordere einen neuen stack an im virt raum (64 TiB bis 64 TiB + 64 KiB) 
            let start = pg_mmap_user_stack(pml4_addr);
            let data = ((start as usize) + (size as usize) - consts::STACK_ENTRY_SIZE) as *mut u8;
            if data.is_null() {
                println!("Panic: failed in 'Stack::new::user_stack'");
                cpu::halt();
            }

            kprintln!(
                "Stack::new, memory block = [0x{:x}; 0x{:x}]",
                start as usize,
                (data as usize + consts::STACK_ENTRY_SIZE)
            );

            Box::new(Stack { data, size }) 
        }        
    } 

    pub fn stack_end(&self) -> *mut u64 {
        self.data as *mut u64
    }
}

impl Drop for Stack {
    fn drop(&mut self) {
        unsafe {
            let layout = Layout::from_size_align_unchecked(self.size, consts::STACK_ALIGNMENT);
            allocator::dealloc(self.data, layout);
        }
    }
}

impl Default for Stack {
    fn default() -> Self {
        Self {
            data: 0 as *mut u8,
            size: 0,
        }
    }
}
