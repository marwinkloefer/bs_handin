/* ╔═════════════════════════════════════════════════════════════════════════╗
   ║ Module: list                                                            ║
   ╟─────────────────────────────────────────────────────────────────────────╢
   ║ Descr.: Implementing a list heap allocator.                             ║
   ╟─────────────────────────────────────────────────────────────────────────╢
   ║ Author: Philipp Oppermann                                               ║
   ║         https://os.phil-opp.com/allocator-designs/                      ║
   ╚═════════════════════════════════════════════════════════════════════════╝
*/

use super::{align_up, Locked};
use crate::{boot::multiboot::PhysRegion, consts::PAGE_FRAME_SIZE, kernel::cpu};
use alloc::{
    alloc::{GlobalAlloc, Layout},
    string::String,
    vec::Vec,
};
use core::{mem, ptr};

/**
 Description: Metadata of a free memory block in the list allocator
*/
struct ListNode {
    // size of the memory block
    size: usize,

    // &'static mut type semantically describes an owned object behind
    // a pointer. Basically, it’s a Box without a destructor that frees
    // the object at the end of the scope.
    next: Option<&'static mut ListNode>,
}

enum MergeCase {
    /// Case 1: Merge all three blocks (prior, new, and next).
    MergeAll,
    /// Case 2: Merge the new block with the next block.
    MergeWithNext,
    /// Case 3: Merge the new block with the prior block.
    MergeWithPrior,
    /// Case 4: No merge occurs.
    NoMerge,
}

impl ListNode {
    // Create new ListMode on Stack
    // (must be 'const')
    const fn new(size: usize) -> Self {
        ListNode { size, next: None }
    }

    // return start address of memory block
    fn start_addr(&self) -> usize {
        self as *const Self as usize
    }

    // return end address of memory block
    fn end_addr(&self) -> usize {
        self.start_addr() + self.size
    }

    // Setzt die Startadresse der Node neu
    fn set_start_addr(&mut self, new_start_addr: usize) {
        let size = self.size;
        let next = self.next.take();
        unsafe {
            let new_ptr = new_start_addr as *mut ListNode;
            ptr::write(new_ptr, ListNode { size, next });
            *self = ptr::read(new_ptr);
        }
    }
}

/**
 Description: Metadata of the list allocator
*/
pub struct LinkedListAllocator {
    head: ListNode,
    heap_start: usize,
    heap_end: usize,
}

pub struct PfListAllocator {
    head: ListNode,
}

impl PfListAllocator {
    // Creates an empty PfListAllocator.
    //
    // Must be const because needs to be evaluated at compile time
    // because it will be used for initializing the ALLOCATOR static
    // see 'allocator.rs'
    pub const fn new() -> Self {
        Self {
            head: ListNode::new(0),
        }
    }

    unsafe fn align_phys_regions_to_4_kb(free: &mut Vec<PhysRegion>) {
        let mut aligned_regions = Vec::new();

        for region in free.iter_mut() {
            // Align `start` to the next multiple of 4 KB
            region.start = (region.start + 0xFFF) & !0xFFF;

            // Align `end` to the previous multiple of 4 KB
            region.end = region.end & !0xFFF;

            // Add to aligned_regions only if the region is valid
            if region.start <= region.end {
                // Clone the region or create a new instance and push it
                aligned_regions.push(PhysRegion {
                    start: region.start,
                    end: region.end,
                });
            }
        }
        // Replace the original vector with the aligned and filtered regions
        *free = aligned_regions;
    }

    // Initialize the allocator with the given heap bounds.
    //
    // This function is unsafe because the caller must guarantee that
    // the given heap bounds are valid. This method must be called only once.
    pub unsafe fn init(&mut self, free: &mut Vec<PhysRegion>, kernel_frames: bool) {
        PfListAllocator::align_phys_regions_to_4_kb(free);
        //free.sort_by_key(|region| region.start);

        for region in free {
            if kernel_frames {
                // Kernel-Speicherbereich: 0 - 64 MiB
                if region.end > 0 && region.start < 64 * 1024 * 1024 {
                    let aligned_end = region.end.min(64 * 1024 * 1024 - 1); // maybe -1 weg hier
                    let size = aligned_end - region.start;

                    if size > 0 {
                        self.add_free_block(region.start as usize, size as usize);
                    }
                }
            } else {
                // User-Speicherbereich: ab 64 MiB
                if region.end > 64 * 1024 * 1024 - 1 {
                    let aligned_start = region.start.max(64 * 1024 * 1024); // Clamp start to 64 MiB
                    let size = region.end - aligned_start;

                    if size > 0 {
                        self.add_free_block(aligned_start as usize, size as usize);
                    }
                }
            }
        }
    }

    unsafe fn add_free_block(&mut self, addr: usize, size: usize) {
        // ensure that the freed block is capable of holding ListNode
        //assert_eq!(align_up(addr, mem::align_of::<ListNode>()), addr);
        assert!(size >= mem::size_of::<ListNode>());
        assert_eq!(addr % PAGE_FRAME_SIZE, 0);

        // create a new ListNode (on stack)
        let node = ListNode::new(size);

        // create a pointer to 'addr' of type ListNode
        let ptr_node_to_insert = addr as *mut ListNode;

        // copy content of new ListNode to 'addr'
        ptr_node_to_insert.write(node);

        let mut ptr_node_before_found_position: *mut ListNode = &mut self.head;

        // --------------------- Suche Position zum einfügen -----------------------------------------------------
        while let Some(ref mut ptr_node_after_found_position) =
            (*ptr_node_before_found_position).next
        {
            let addr_of_node_after_found_position =
                (*ptr_node_after_found_position as *const ListNode) as usize;
            if addr_of_node_after_found_position > addr {
                break; // Found insertion point
            }
            ptr_node_before_found_position = *ptr_node_after_found_position as *mut ListNode;
        }

        // --------------------- Debug: On/Off -----------------------------------------------------
        let debug = false;
        // --------------------- Debug: On/Off -----------------------------------------------------

        if debug {
            kprintln!(
                "block befor found position at 0x{:x} with size {} up to addr at 0x{:x}",
                (*ptr_node_before_found_position).start_addr(),
                (*ptr_node_before_found_position).size,
                (*ptr_node_before_found_position).end_addr()
            );
        }
        if debug {
            kprintln!(
                "block to insert at 0x{:x} with size {} up to addr at 0x{:x}",
                (*ptr_node_to_insert).start_addr(),
                (*ptr_node_to_insert).size,
                (*ptr_node_to_insert).end_addr()
            );
        }
        if debug {
            if let Some(ref mut ptr_node_after_found_position) =
                (*ptr_node_before_found_position).next
            {
                kprintln!(
                    "block after found position 0x{:x} with size {} up to addr at 0x{:x}",
                    (*ptr_node_after_found_position).start_addr(),
                    (*ptr_node_after_found_position).size,
                    (*ptr_node_after_found_position).end_addr()
                );
            } else {
                kprintln!("block after found position at: List End");
            }
        }

        // --------------------- Merge in 3 verschiedenen Fällen -----------------------------------------------------

        // Case 1: End of prior == Start of new && End of new == Start of next
        // alle drei vereinen => Block auf den ptr_node_before_found_position zeigt vergrößern
        // (*ptr_node_before_found_position).size erhöhen um size + (*ptr_node_after_found_position).size
        // (*ptr_node_before_found_position).next auf den node setzen auf den (*ptr_node_after_found_position).next zeigt
        // Case 2: End of new == Start of next
        // neuen und folgenden vereinen => Block auf den ptr_node_after_found_position zeigt vergrößern und auf neue start adresse setzen
        // (*ptr_node_after_found_position).size erhöhen um size
        // adresse auf die ptr_node_after_found_position zeigt auf ptr_node_to_insert ändern
        // Case 3: End of prior == Start of new
        // neuen und vorangehenden vereinen => Block auf den ptr_node_before_found_position zeigt vergrößern
        // (*ptr_node_before_found_position).size erhöhen um size
        // Case 4: No merge

        if let Some(ref mut ptr_node_after_found_position) = (*ptr_node_before_found_position).next
        {
            // Nicht am Ende einfügen => Case 1 - 4
            let case = if (*ptr_node_before_found_position).end_addr() == (*ptr_node_to_insert).start_addr() && (*ptr_node_to_insert).end_addr() == (*ptr_node_after_found_position).start_addr(){
                MergeCase::MergeAll                
            } else if (*ptr_node_to_insert).end_addr() == (*ptr_node_after_found_position).start_addr(){
                MergeCase::MergeWithNext
            } else if (*ptr_node_before_found_position).end_addr() == (*ptr_node_to_insert).start_addr(){
                MergeCase::MergeWithPrior
            } else {
                MergeCase::NoMerge
            };
            if debug {
                kprintln!("#####################################");
                kprintln!("#####################################");
                kprintln!("end_prior  == 0x{:x}", (*ptr_node_before_found_position).end_addr());
                kprintln!("start_new  == 0x{:x}", (*ptr_node_to_insert).start_addr());
                kprintln!("end_new    == 0x{:x}", (*ptr_node_to_insert).end_addr());
                kprintln!("start_next == 0x{:x}", (*ptr_node_after_found_position).start_addr());
            }
            match case {
                MergeCase::MergeAll => {
                    if debug {
                        kprintln!(".......................");
                        kprintln!("....add_free case 1....");
                        kprintln!(
                            "MergeAll: end_prior 0x{:x} == 0x{:x} start_new && end_new 0x{:x} == 0x{:x} start_next",
                            (*ptr_node_before_found_position).end_addr(),
                            (*ptr_node_to_insert).start_addr(),
                            (*ptr_node_to_insert).end_addr(),
                            (*ptr_node_after_found_position).start_addr()
                        );
                        kprintln!(".......................");
                    }
                    // case 1: Merged new block with prior one
                    let old_size = (*ptr_node_before_found_position).size;
                    (*ptr_node_before_found_position).size =
                        old_size + size + (*ptr_node_after_found_position).size;
                    (*ptr_node_before_found_position).next =
                        (*ptr_node_after_found_position).next.take();

                    if debug {
                        kprintln!(
                        "Case 1: Merged new block with prior and following one starting at 0x{:x}, old size: 0x{:x}, new size: 0x{:x}",
                        (*ptr_node_before_found_position).start_addr(), old_size, (*ptr_node_before_found_position).size
                    );
                    }
                }
                MergeCase::MergeWithNext => {
                    if debug {
                        kprintln!(".......................");
                        kprintln!("....add_free case 2....");
                        kprintln!(
                            "MergeWithNext: end_new 0x{:x} == 0x{:x} start_next",
                            (*ptr_node_to_insert).end_addr(),
                            (*ptr_node_after_found_position).start_addr()
                        );
                        kprintln!(".......................");
                    }
                    // case 2: Merged new block with prior one
                    let old_size = (*ptr_node_before_found_position).size;
                    (*ptr_node_before_found_position).size = old_size + size;

                    if debug {
                        kprintln!(
                        "Case 2: Merged new block with prior one starting at 0x{:x}, old size: 0x{:x}, new size: 0x{:x}",
                        (*ptr_node_before_found_position).start_addr(), old_size, (*ptr_node_before_found_position).size
                    );
                    }
                }
                MergeCase::MergeWithPrior => {
                    if debug {
                        kprintln!(".........................");
                        kprintln!("....add_free case 3.1....");
                        kprintln!(
                            "MergeWithPrior: end_prior 0x{:x} == 0x{:x} start_new",
                            (*ptr_node_before_found_position).end_addr(),
                            (*ptr_node_to_insert).start_addr()
                        );
                        kprintln!(".........................");
                    }
                    // case 3: Merged new block with post one
                    let old_size = (*ptr_node_after_found_position).size;
                    (*ptr_node_after_found_position).size = old_size + size;
                    ptr_node_after_found_position.set_start_addr(addr);

                    if debug {
                        kprintln!(
                        "Case 3: Merged new block with following one starting at 0x{:x}, old size: 0x{:x}, new size: 0x{:x}",
                        (*ptr_node_before_found_position).start_addr(), old_size, (*ptr_node_before_found_position).size
                    );
                    }
                }
                MergeCase::NoMerge => {
                    if debug {
                        kprintln!(".................................");
                        kprintln!("....add_free case 4.1 NoMerge....");
                        kprintln!(".................................");
                    }
                    // case 4: no merge
                    // Insert the new node
                    (*ptr_node_to_insert).next = (*ptr_node_before_found_position).next.take(); // Link the new node to the next node
                    (*ptr_node_before_found_position).next = Some(&mut *ptr_node_to_insert); // Link the current node to the new node
                    if debug {
                        kprintln!("no merge inserted between above mentioned blocks");
                    }
                }
            }
        } else {
            if debug {
                kprintln!("#####################################");
                kprintln!("#####################################");
                kprintln!("end_prior  == 0x{:x}", (*ptr_node_before_found_position).end_addr());
                kprintln!("start_new  == 0x{:x}", (*ptr_node_to_insert).start_addr());
                kprintln!("end_new    == 0x{:x}", (*ptr_node_to_insert).end_addr());
                kprintln!("start_next == List End");
            }
            // Am Ende einfügen => Case 3 und 4
            if (*ptr_node_before_found_position).end_addr() == (*ptr_node_to_insert).start_addr() {
                if debug {
                    kprintln!(".........................");
                    kprintln!("....add_free case 3.2....");
                    kprintln!(
                        "MergeWithPrior: end_prior 0x{:x} == 0x{:x} start_new",
                        (*ptr_node_before_found_position).end_addr(),
                        (*ptr_node_to_insert).start_addr()
                    );
                    kprintln!(".........................");
                }
                // case 2: Merged new block with prior one
                let old_size = (*ptr_node_before_found_position).size;
                (*ptr_node_before_found_position).size = old_size + size;

                if debug {
                    kprintln!(
                    "Case 3: Merged new block with prior one starting at 0x{:x}, old size: 0x{:x}, new size: 0x{:x}",
                    (*ptr_node_before_found_position).start_addr(), old_size, (*ptr_node_before_found_position).size
                );
                }
            } else {
                if debug {
                    kprintln!(".................................");
                    kprintln!("....add_free case 4.2 NoMerge....");
                    kprintln!(".................................");
                }
                // case 4: no merge
                // Insert the new node
                (*ptr_node_to_insert).next = (*ptr_node_before_found_position).next.take(); // Link the new node to the next node
                (*ptr_node_before_found_position).next = Some(&mut *ptr_node_to_insert); // Link the current node to the new node
                if debug {
                    kprintln!("no merge inserted between above mentioned blocks");
                }
            }
        }
        if debug {
            kprintln!("#####################################");
            kprintln!("#####################################");
        }
    }

    // Search a free block with the given size and alignment and remove
    // it from the free list.
    //
    // Return: 'ListNode' or 'None'
    fn find_free_block(&mut self, pf_count: usize) -> Option<&'static mut ListNode> {
        // 4KB times pf_count
        let size = pf_count * PAGE_FRAME_SIZE;
        // reference to current list node, updated for each iteration
        let mut current = &mut self.head;

        // search for a large enough memory block in the linked list
        // save next block in 'block' (may be 'None' -> use 'Some')
        while let Some(ref mut block) = current.next {
            // check if current 'block' is large enough
            if let Ok(alloc_start) = Self::check_block_for_alloc(&block, size) {
                let next = block.next.take(); // save successor of 'block'
                let ret = Some(current.next.take().unwrap()); // take 'block'
                current.next = next; // set 'next' to successor of 'block'
                return ret;
            } else {
                // block too small -> continue with next block
                current = current.next.as_mut().unwrap();
            }
        }
        // no suitable block found
        None
    }

    // Check if the given 'block' is large enough for an allocation with
    //
    // Return: OK(allocation start address) or Err
    fn check_block_for_alloc(block: &ListNode, size: usize) -> Result<usize, ()> {
        let alloc_start = block.start_addr(); //align_up(block.start_addr());

        let alloc_end = match alloc_start.checked_add(size) {
            Some(end) => end, // unused but required by compiler
            None => return Err(()),
        };

        // block too small?
        if alloc_end > block.end_addr() {
            return Err(());
        }

        // rest of block too small to hold a ListNode (required because the
        // allocation splits the block in a used and a free part)
        let remaining_block_size = block.end_addr() - alloc_end;
        if remaining_block_size > 0 && remaining_block_size < mem::size_of::<ListNode>() {
            return Err(());
        }

        // block suitable for allocation
        Ok(alloc_start)
    }

    // Dump free list
    pub fn dump_free_list(&mut self, input_string: String) {
        kprintln!(
            "Dumping free memory list PFListAllocator (including dummy element): {}",
            input_string
        );

        // reference to current list node, updated for each iteration
        let mut current = &mut self.head;

        // Walk through linked list
        while let Some(ref mut block) = current.next {
            kprintln!(
                "   Block start:  0x{:x}, block end: 0x{:x}, block size: 0x{:x}, 4kb block num: {}",
                block.start_addr(),
                block.start_addr() + block.size,
                block.size,
                block.size / PAGE_FRAME_SIZE
            );

            // continue with next block
            current = current.next.as_mut().unwrap();
        }
    }

    // soll pf_count viele 4kb blöcke allozieren
    // wird entweder auf dem Kernel Space oder User-Space aufgerufen
    pub unsafe fn alloc(&mut self, pf_count: usize) -> *mut u8 {
        // perform layout adjustments
        let ret_ptr: *mut u8;

        if let Some(block) = self.find_free_block(pf_count) {
            let alloc_end = block
                .start_addr()
                .checked_add(pf_count * PAGE_FRAME_SIZE)
                .expect("overflow");

            // the remaining memory will be inserted as new block
            // the size is large enough to store metadata; this is
            // checked in 'check_block_for_alloc' called by 'find_free_block'
            let remaining_block_size = block.end_addr() - alloc_end;
            if remaining_block_size > 0 {
                self.add_free_block(alloc_end, remaining_block_size);
                /*kprintln!(
                    "remaining block at addr=0x{:x} with size 0x{:x} ({} 4kb frames remaining)",
                    alloc_end,
                    remaining_block_size,
                    remaining_block_size / PAGE_FRAME_SIZE
                );*/
            }
            // gefunden freien block zürck geben
            ret_ptr = block.start_addr() as *mut u8;

            // Neuen Block mit 0en initialisieren (langsam)
            if !ret_ptr.is_null() {
                let slice = unsafe { core::slice::from_raw_parts_mut(ret_ptr, pf_count * PAGE_FRAME_SIZE) };
                slice.fill(0);
            }

            kprintln!(
                "allocated block from addr=0x{:x} till addr=0x{:x} with size 0x{:x} and {} Blocks",
                block.start_addr(),
                alloc_end - 1,
                pf_count * PAGE_FRAME_SIZE,
                pf_count
            );
        } else {
            // println!(", *** out of memory ***");
            ret_ptr = ptr::null_mut(); // out of memory
        }
        ret_ptr
    }

    pub unsafe fn dealloc(&mut self, ptr: *mut u8, pf_count: usize) {
        //kprintln!("   dealloc: size={}, align={}; not supported", layout.size(), layout.align());
        assert_eq!(ptr as usize % PAGE_FRAME_SIZE, 0);
        let size = pf_count * PAGE_FRAME_SIZE;
        self.add_free_block(ptr as usize, size)
    }
}

impl LinkedListAllocator {
    // Creates an empty LinkedListAllocator.
    //
    // Must be const because needs to be evaluated at compile time
    // because it will be used for initializing the ALLOCATOR static
    // see 'allocator.rs'
    pub const fn new() -> Self {
        Self {
            head: ListNode::new(0),
            heap_start: 0,
            heap_end: 0,
        }
    }

    // Initialize the allocator with the given heap bounds.
    //
    // This function is unsafe because the caller must guarantee that
    // the given heap bounds are valid. This method must be called only once.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.add_free_block(heap_start, heap_size);

        self.heap_start = heap_start;
        self.heap_end = heap_start + heap_size - 1;
    }

    // Adds the given free memory block 'addr' to the front of the free list.
    unsafe fn add_free_block(&mut self, addr: usize, size: usize) {
        // ensure that the freed block is capable of holding ListNode
        assert_eq!(align_up(addr, mem::align_of::<ListNode>()), addr);
        assert!(size >= mem::size_of::<ListNode>());

        // create a new ListNode (on stack)
        let mut node = ListNode::new(size);

        // set next ptr of new ListNode to existing 1st block
        node.next = self.head.next.take();

        // create a pointer to 'addr' of Type ListNode
        let ptr_node_to_insert = addr as *mut ListNode;

        // copy content of new ListeNode to 'addr'
        ptr_node_to_insert.write(node);

        // update ptr. to 1st block in global variable 'head'
        self.head.next = Some(&mut *ptr_node_to_insert);
    }

    // Search a free block with the given size and alignment and remove
    // it from the free list.
    //
    // Return: 'ListNode' or 'None'
    fn find_free_block(&mut self, size: usize, align: usize) -> Option<&'static mut ListNode> {
        // reference to current list node, updated for each iteration
        let mut current = &mut self.head;

        // search for a large enough memory block in the linked list
        // save next block in 'block' (may be 'None' -> use 'Some')
        while let Some(ref mut block) = current.next {
            // check if current 'block' is large enough
            if let Ok(alloc_start) = Self::check_block_for_alloc(&block, size, align) {
                let next = block.next.take(); // save successor of 'block'
                let ret = Some(current.next.take().unwrap()); // take 'block'
                current.next = next; // set 'next' to successor of 'block'
                return ret;
            } else {
                // block too small -> continue with next block
                current = current.next.as_mut().unwrap();
            }
        }
        // no suitable block found
        None
    }

    // Check if the given 'block' is large enough for an allocation with
    // 'size' and alignment 'align'
    //
    // Return: OK(allocation start address) or Err
    fn check_block_for_alloc(block: &ListNode, size: usize, align: usize) -> Result<usize, ()> {
        let alloc_start = align_up(block.start_addr(), align);

        let alloc_end = match alloc_start.checked_add(size) {
            Some(end) => end, // unused but required by compiler
            None => return Err(()),
        };

        // block too small?
        if alloc_end > block.end_addr() {
            return Err(());
        }

        // rest of block too small to hold a ListNode (required because the
        // allocation splits the block in a used and a free part)
        let remaining_block_size = block.end_addr() - alloc_end;
        if remaining_block_size > 0 && remaining_block_size < mem::size_of::<ListNode>() {
            return Err(());
        }

        // block suitable for allocation
        Ok(alloc_start)
    }

    // Adjust the given layout so that the resulting allocated memory
    // block is also capable of storing a `ListNode`.
    //
    // Returns the adjusted size and alignment as a (size, align) tuple.
    fn size_align(layout: Layout) -> (usize, usize) {
        let layout = layout
            .align_to(mem::align_of::<ListNode>())
            .expect("adjusting alignment failed")
            .pad_to_align();
        let size = layout.size().max(mem::size_of::<ListNode>());
        (size, layout.align())
    }

    // Dump free list
    pub fn dump_free_list(&mut self) {
        println!("Dumping free memory list (including dummy element)");
        println!(
            "   Heap start:   0x{:x}, heap end:  0x{:x}",
            self.heap_start, self.heap_end
        );

        // reference to current list node, updated for each iteration
        let mut current = &mut self.head;

        // Walk through linked list
        while let Some(ref mut block) = current.next {
            println!(
                "   Block start:  0x{:x}, block end: 0x{:x}, block size: {}",
                block.start_addr(),
                block.start_addr() + block.size - 1,
                block.size
            );

            // continue with next block
            current = current.next.as_mut().unwrap();
        }
    }

    pub unsafe fn alloc(&mut self, layout: Layout) -> *mut u8 {
        // kprint!("   alloc: size={}, align={}", layout.size(), layout.align());

        // perform layout adjustments
        let (size, align) = LinkedListAllocator::size_align(layout);
        let ret_ptr: *mut u8;

        if let Some(block) = self.find_free_block(size, align) {
            let alloc_end = block.start_addr().checked_add(size).expect("overflow");

            // the remaining memory will be inserted as new block
            // the size is large enough to store metadata; this is
            // checked in 'check_block_for_alloc' called by 'find_free_block'
            let remaining_block_size = block.end_addr() - alloc_end;
            if remaining_block_size > 0 {
                self.add_free_block(alloc_end, remaining_block_size);
            }
            ret_ptr = block.start_addr() as *mut u8;
            //   kprintln!(", returning addr=0x{:x}", block.start_addr());
        } else {
            // println!(", *** out of memory ***");
            ret_ptr = ptr::null_mut(); // out of memory
        }
        ret_ptr
    }

    pub unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        //kprintln!("   dealloc: size={}, align={}; not supported", layout.size(), layout.align());
        let (size, _) = LinkedListAllocator::size_align(layout);
        self.add_free_block(ptr as usize, size)
    }
}

// Trait required by the Rust runtime for heap allocations
unsafe impl GlobalAlloc for Locked<LinkedListAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.lock().alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.lock().dealloc(ptr, layout);
    }
}
