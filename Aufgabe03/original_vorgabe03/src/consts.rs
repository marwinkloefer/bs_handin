#![allow(dead_code)] // avoid warnings

// Speicher pro Stack = 64 KB
pub const STACK_SIZE: usize = 0x1_0000;

// 1 MB Heap für das Einrichten des Systems (siehe 'kmain')
pub const TEMP_HEAP_SIZE: usize = 0x10_0000;

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
