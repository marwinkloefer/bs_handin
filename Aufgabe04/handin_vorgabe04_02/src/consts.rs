#![allow(dead_code)] // avoid warnings

// Stack size for each new thread
// old STACK_SIZE prev Blatt 3
//pub const STACK_SIZE: usize = 0x80000; // 512 KB for each stack
pub const STACK_ALIGNMENT: usize = 8;
pub const STACK_ENTRY_SIZE: usize = 8;

pub const HEAP_SIZE: usize = 16 * 1024 * 1024; // 16 MB heap size


// Speicher pro Stack = 64 KB
pub const STACK_SIZE: usize = 0x1_0000;

// 1 MB Heap für das Einrichten des Systems (siehe 'kmain')
pub const TEMP_HEAP_SIZE: usize = 0x10_0000;

// Seitengroesse = 4 KB
pub const PAGE_SIZE: usize = 0x1000;

// 1 MB Heap für das Einrichten des Systems (siehe 'kmain')
pub const KERNEL_HEAP_SIZE: usize = 0x10_0000;

// Kachelgroesse = 4 KB
pub const PAGE_FRAME_SIZE: usize = 0x1000;

//
// Konstanten fuer den physikalischen Adresseraum des Kernels
//
pub const KERNEL_PHYS_SIZE: usize = 0x400_0000; // 64 MiB DRAM fuer den Kernel
pub const KERNEL_PHYS_START: usize = 0;
pub const KERNEL_PHYS_END: usize = KERNEL_PHYS_SIZE - 1;

//
// Konstanten fuer den virtuellen Adresseraum des Kernels
//
// Kernel Pages werden 1:1 abgebildet (virt. Adresse = phys. Adresse)
pub const KERNEL_VM_SIZE: usize = 0x100_0000_0000;  // 1 TiB
pub const KERNEL_VM_START: usize = 0;
pub const KERNEL_VM_END: usize = KERNEL_VM_SIZE - 1;

//
// Konstanten fuer den virtuellen Adresseraum des User-Modes
// Vorerst nur fuer den Stack des User-Mode-Threads, beginnt ab 64 TiB - 1.
//
pub const USER_STACK_VM_START:usize = 0x4000_0000_0000;
pub const USER_STACK_VM_END: usize = USER_STACK_VM_START + STACK_SIZE - 1;
