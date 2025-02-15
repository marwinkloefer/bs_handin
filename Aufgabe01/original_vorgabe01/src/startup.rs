/* ╔═════════════════════════════════════════════════════════════════════════╗
   ║ Module: startup                                                         ║
   ╟─────────────────────────────────────────────────────────────────────────╢
   ║ Descr.: Here is the function 'kmain' called from the boot code and the  ║
   ║         panic handler. All features are set and all modules are         ║
   ║         imported.                                                       ║
   ╟─────────────────────────────────────────────────────────────────────────╢
   ║ Author: Michael Schoettner, Univ. Duesseldorf, 15.8.2023                ║
   ╚═════════════════════════════════════════════════════════════════════════╝
*/#![no_std]
#![feature(const_mut_refs)]
#![allow(dead_code)] // avoid warnings
#![allow(unused_variables)] // avoid warnings
#![allow(unused_imports)]
#![allow(unused_macros)]
#![feature(alloc_error_handler)]

extern crate alloc;
extern crate spin; // we need a mutex in devices::cga_print
extern crate x86;

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
use core::panic::PanicInfo;

use devices::cga;
use devices::cga_print; // used to import code needed by println!
use devices::keyboard; // keyboard
use devices::kprint; // used to import code needed by kprintln!
use devices::pit; // timer

use kernel::allocator;
use kernel::cpu;
use kernel::interrupts;
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

#[no_mangle]
pub extern "C" fn kmain(mbi: u64) {
    kprintln!("kmain");

    let kernel_region = get_kernel_image_region();
    kprintln!("   kernel_region: {:?}", kernel_region);

    // Speicherverwaltung (1 MB) oberhalb des Images initialisieren
    let heap_start = kernel_region.end as usize + 1;
    allocator::init(heap_start, consts::HEAP_SIZE);

    // Multiboot-Infos ausgeben
    multiboot::dump(mbi);

    // Interrupt-Strukturen initialisieren
    interrupts::init();

    // Tastatur-Unterbrechungsroutine 'einstoepseln'
    keyboard::Keyboard::plugin();

    // Zeitgeber-Unterbrechungsroutine 'einstoepseln'
    pit::plugin();

    // Idle-Thread eintragen
    let idle_thread = Thread::new(scheduler::next_thread_id(), idle_thread::idle_thread_entry, true);
    scheduler::Scheduler::ready(idle_thread);

    // HelloWorld-Thread eintragen
    let hello_world_thread = Thread::new(scheduler::next_thread_id(), hello_world_thread::hello_world_thread_entry, true);
    scheduler::Scheduler::ready(hello_world_thread);

    // Scheduler starten & Interrupts erlauben
    scheduler::Scheduler::schedule();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("Panic: {}", info);
    loop {}
}
