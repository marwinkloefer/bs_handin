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
use crate::kernel::cpu;
use alloc::{
    alloc::{GlobalAlloc, Layout},
    string::String,
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
}


/**
 Description: Metadata of the list allocator
*/
pub struct LinkedListAllocator {
    head: ListNode,
    heap_start: usize,
    heap_end: usize,
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
        self.heap_end   = heap_start + heap_size - 1;
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
        let node_ptr = addr as *mut ListNode;   
        
         // copy content of new ListeNode to 'addr'
        node_ptr.write(node); 
        
        // update ptr. to 1st block in global variable 'head'
        self.head.next = Some(&mut *node_ptr); 
    }
    
    
    // Search a free block with the given size and alignment and remove
    // it from the free list.
    //
    // Return: 'ListNode' or 'None'
    fn find_free_block(&mut self, size: usize, align: usize)
        -> Option<&'static mut ListNode>
    {
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
    fn check_block_for_alloc(block: &ListNode, size: usize, align: usize)
        -> Result<usize, ()>
    {
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
        if remaining_block_size > 0 && 
           remaining_block_size < mem::size_of::<ListNode>() 
        {
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
		println!("   Heap start:   0x{:x}, heap end:  0x{:x}", 
		            self.heap_start, self.heap_end );
		
        // reference to current list node, updated for each iteration
        let mut current = &mut self.head;
        
        // Walk through linked list
        while let Some(ref mut block) = current.next {
		    println!("   Block start:  0x{:x}, block end: 0x{:x}, block size: {}", 
		                block.start_addr(), 
		                block.start_addr() + block.size - 1, 
		                block.size );
            
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
