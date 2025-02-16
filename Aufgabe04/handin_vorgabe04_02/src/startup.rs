/* ╔═════════════════════════════════════════════════════════════════════════╗
   ║ Module: startup                                                         ║
   ╟─────────────────────────────────────────────────────────────────────────╢
   ║ Descr.: Here is the function 'kmain' called from the boot code and the  ║
   ║         panic handler. All features are set and all modules are         ║
   ║         imported.                                                       ║
   ╟─────────────────────────────────────────────────────────────────────────╢
   ║ Author: Michael Schoettner, Univ. Duesseldorf, 15.8.2023                ║
   ╚═════════════════════════════════════════════════════════════════════════╝
*/
#![no_std]
#![feature(const_mut_refs)]
#![allow(dead_code)] // avoid warnings
#![allow(unused_variables)] // avoid warnings
#![allow(unused_imports)]
#![allow(unused_macros)]
#![feature(alloc_error_handler)]
#![feature(naked_functions)]

extern crate alloc;
extern crate spin; // we need a mutex in devices::cga_print
extern crate x86;
extern crate bitflags;

// insert other modules
#[macro_use] // import macros, too
mod devices;
mod boot;
mod consts;
mod kernel;
mod mylib;
mod user;

use alloc::boxed::Box;
use boot::multiboot;
use consts::KERNEL_HEAP_SIZE;
use consts::PAGE_FRAME_SIZE;
use consts::TEMP_HEAP_SIZE;
use kernel::paging::frames;
use kernel::paging::frames::PhysAddr;
use kernel::paging::pages;
use core::panic::PanicInfo;

use devices::cga;
use devices::cga_print; // used to import code needed by println!
use devices::keyboard; // keyboard
use devices::kprint; // used to import code needed by kprintln!
use devices::pit; // timer

use kernel::allocator;
use kernel::cpu;
use kernel::interrupts;
use kernel::syscall::syscall_dispatcher;
use kernel::threads::idle_thread;
use kernel::threads::scheduler;
use kernel::threads::thread::Thread;

use user::hello_world_thread;

use crate::boot::multiboot::PhysRegion;

// Konstanten im Linker-Skript
extern "C" {
    static ___KERNEL_DATA_START__: u64;
    static ___KERNEL_DATA_END__: u64;
}

// Start- und Endadresse des Kernel-Images ermitteln,
// aufrunden auf das naechste volle MB und zurueckgeben
fn get_kernel_image_region() -> multiboot::PhysRegion {
    let kernel_start: usize;
    let kernel_end: usize;

    unsafe {
        kernel_start = &___KERNEL_DATA_START__ as *const u64 as usize;
        kernel_end = &___KERNEL_DATA_END__ as *const u64 as usize;
    }

    // Kernel-Image auf das naechste MB aufrunden
    let mut kernel_rounded_end = kernel_end & 0xFFFFFFFFFFF00000;
    kernel_rounded_end += 0x100000 - 1; // 1 MB aufaddieren

    PhysRegion {
        start: kernel_start as u64,
        end: kernel_rounded_end as u64,
    }
}

// Einen temperoraeren Heap anlegen, nach dem Ende des Kernel-Images
fn create_temp_heap(kernel_end: usize) -> multiboot::PhysRegion {
    let heap_start = kernel_end + 1;

    // Temporaeren Heap einrichten, nach dem Kernel-Image
    allocator::init(heap_start, TEMP_HEAP_SIZE);

    PhysRegion {
        start: heap_start as u64,
        end: (heap_start + TEMP_HEAP_SIZE - 1) as u64,
    }
}

// wird in boot.asm aufgerufen mit "mbi = _multiboot_addr" als parameter
#[no_mangle]
pub extern "C" fn kmain(mbi: u64) {
    kprintln!("kmain");

    let kernel_region = get_kernel_image_region();
    kprintln!("kmain, kernel_image: {:?}", kernel_region);

    // Verfuegbaren physikalischen Speicher ermitteln (exklusive Kernel-Image und Heap)
    let heap_region = create_temp_heap(kernel_region.end as usize);
    kprintln!("kmain, heap: {:?}", heap_region);

    // Verfuegbaren physikalischen Speicher ermitteln (exklusive Kernel-Image und Heap)
    let mut phys_mem = multiboot::get_free_memory(mbi, kernel_region, heap_region);
    kprintln!("kmain, free physical memory: {:?}", phys_mem);

    // Dump multiboot infos
    // mbi == multiboot address 
    multiboot::dump(mbi);

    // Page-Frame-Management einrichten
    frames::pf_init(&mut phys_mem);

    // Paging fuer den Kernel aktivieren
    let pml4_addr = pages::pg_init_kernel_tables();
    kprintln!("kmain: setze CR3 auf 0x{:x}", pml4_addr.raw());
    pages::pg_set_cr3(pml4_addr);

    // Kernel Heap einrichten
    kprintln!("kmain: Kernel Heap einrichten");
    let kernel_heap = frames::pf_alloc(KERNEL_HEAP_SIZE.div_ceil(PAGE_FRAME_SIZE), true); // Teilen und aufrunden um 4kb alignment sicherzustellen
    allocator::init(kernel_heap.to_start_address(), TEMP_HEAP_SIZE);

    kprintln!(".... dumping ....");
    frames::pf_dump_lists();
    kprintln!("...........");
   
    // Interrupt-Strukturen initialisieren
    interrupts::init();

    // Interrupt Descriptor Table an Stelle 0x80 Trap-Gate erstellen
    syscall_dispatcher::init();

    // Tastatur-Unterbrechungsroutine 'einstoepseln'
    keyboard::Keyboard::plugin();

    // Zeitgeber-Unterbrechungsroutine 'einstoepseln'
    pit::plugin();

    // Idle-Thread eintragen
    let idle_thread = Thread::new(
        scheduler::next_thread_id(),
        idle_thread::idle_thread_entry,
        true, //hier setzen welcher Ring Thread Idle läuft Aufgabe 1
    );
    scheduler::Scheduler::ready(idle_thread);

    /*// HelloWorld-Thread eintragen
    let hello_world_thread = Thread::new(
        scheduler::next_thread_id(),
        hello_world_thread::hello_world_thread_entry,
        false, //hier setzen welcher Ring Thread Hello World läuft Aufgabe 1
    );
    scheduler::Scheduler::ready(hello_world_thread);
    */
    // Scheduler starten & Interrupts erlauben
    scheduler::Scheduler::schedule();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("Panic: {}", info);
    loop {}
}
