

// ...


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


#[no_mangle]
pub extern "C" fn kmain(mbi: u64) {
    kprintln!("kmain");

    let kernel_region = get_kernel_image_region();
    kprintln!("kmain, kernel_image: {:?}", kernel_region);

    // Verfuegbaren physikalischen Speicher ermitteln (exklusive Kernel-Image und Heap)
    let heap_region = create_temp_heap(kernel_region.end as usize);
    kprintln!("kmain, heap: {:?}", heap_region);

    // Verfuegbaren physikalischen Speicher ermitteln (exklusive Kernel-Image und Heap)
    let phys_mem = multiboot::get_free_memory(mbi, kernel_region, heap_region);
    kprintln!("kmain, free physical memory: {:?}", phys_mem);

    // Dump multiboot infos
    multiboot::dump(mbi);

    // Page-Frame-Management einrichten
    frames::pf_init(phys_mem);

    // Kernel Heap einrichten, nach dem Kernel-Image
    /*
      
      hier muss Code eingefügt werden
      
     */
   
    // ...
}
