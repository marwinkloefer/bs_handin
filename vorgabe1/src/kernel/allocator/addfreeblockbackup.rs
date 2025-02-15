


/*
        
        // Kernel Heap einrichten, nach dem Kernel-Image
    kprintln!("__________alloc some space__________");
    let test0 = frames::pf_alloc(10, false);
    let test1 = frames::pf_alloc(10, false);
    let test2 = frames::pf_alloc(10, false);
    let test3 = frames::pf_alloc(10, false);
    let test4 = frames::pf_alloc(10, false);
    let test5 = frames::pf_alloc(10, false);
    let test6 = frames::pf_alloc(10, false);
    let test7 = frames::pf_alloc(10, false);
    let test10 = frames::pf_alloc(10, true);
    let test11 = frames::pf_alloc(10, true);
    let test12 = frames::pf_alloc(10, true);
    let test13 = frames::pf_alloc(10, true);
    let test14 = frames::pf_alloc(10, true);
    let test15 = frames::pf_alloc(10, true);
    let test16 = frames::pf_alloc(10, true);
    let test17 = frames::pf_alloc(10, true);
    kprintln!("__________free some space__________");
    frames::pf_free(test0, 10);
    frames::pf_free(test1, 10);
    frames::pf_free(test2, 10);
    frames::pf_free(test3, 10);
    frames::pf_free(test4, 10);
    frames::pf_free(test5, 10);
    frames::pf_free(test6, 10);
    frames::pf_free(test7, 10);
    frames::pf_free(test10, 10);
    frames::pf_free(test11, 10);
    frames::pf_free(test12, 10);
    frames::pf_free(test13, 10);
    frames::pf_free(test14, 10);
    frames::pf_free(test15, 10);
    frames::pf_free(test16, 10);
    frames::pf_free(test17, 10);
    //frames::pf_free(PhysAddr::new(test3.raw()+0x1000), 9);
    kprintln!("___________________________________");
      
    */













unsafe fn add_free_block(&mut self, addr: usize, size: usize) {
    // ensure that the freed block is capable of holding ListNode
    //assert_eq!(align_up(addr, mem::align_of::<ListNode>()), addr);
    assert!(size >= mem::size_of::<ListNode>());
    assert_eq!(addr % PAGE_FRAME_SIZE, 0);

    // create a new ListNode (on stack)
    let node = ListNode::new(size);

    // create a pointer to 'addr' of type ListNode
    let node_ptr = addr as *mut ListNode;

    // copy content of new ListNode to 'addr'
    node_ptr.write(node);

    // Start traversing from the head of the list
    let mut current_ptr: *mut ListNode = &mut self.head;

    // Zwischenspeicherung des vorherigen Blocks
    let mut prev_block_ptr: Option<*mut ListNode> = None; 

    // Traverse the list to find the correct position
    while let Some(ref mut next_node) = (*current_ptr).next {
        let next_addr = (*next_node as *const ListNode) as usize;
        if next_addr > addr {
            break; // Found insertion point
        }
        // Speichere den Block, der auf current zeigt
        prev_block_ptr = Some(current_ptr);
        // Move to the next node using a raw pointer
        current_ptr = *next_node as *mut ListNode;
    }
    
    // -------------------------- NEW LOGIC TO MERGE BLOCKS --------------------------
    //      current ist block nachdem neuer block eingefügt werden woll
    //      node_ptr ist block der eingefügt werden soll
    //      current_ptr.next ist block der nach dem neuen block kommen soll
    //      current -> node -> current.next
    // -------------------------------------------------------------------------------

    
    // Safely print the details of the blocks prior
    let prior_block_addr = (*current_ptr).start_addr();
    let prior_block_size = (*current_ptr).size;
    let prior_block_end = prior_block_addr + prior_block_size;
    kprintln!("block prior to new block at 0x{:x} with size {} up to addr at 0x{:x}", prior_block_addr, prior_block_size, prior_block_end);
    
    // Safely print the details of the blocks new
    let new_block_addr = (*node_ptr).start_addr();
    let new_block_size = (*node_ptr).size;
    let new_block_end = new_block_addr + new_block_size;
    kprintln!("               new block at 0x{:x} with size {} up to addr at 0x{:x}", new_block_addr, new_block_size, new_block_end);
    
    // Safely print the details of the blocks post
    if let Some(next_node) = (*current_ptr).next.take() {
        let next_block_addr = (*next_node).start_addr();
        let next_block_size = (*next_node).size;
        let next_block_end = next_block_addr + next_block_size;
        kprintln!("block after    new block at 0x{:x} with size {} up to addr at 0x{:x}", next_block_addr, next_block_size, next_block_end);
    } else {
        kprintln!("block after    new block: None");
    }

    
    // case 1: end_prio == start_new && end_new == start_next
    // case 2: end_new == start_next
    // case 3: end_prior == start_new 
    // case 4: no merge
    if let Some(next_node) = (*current_ptr).next.take() {
        let next_block_addr = (*next_node).start_addr();
        let next_block_size = (*next_node).size;
        let new_block_end = addr + size;
        if prior_block_end == addr && new_block_end == next_block_addr {
            // case 1: end_prior == start_new && end_new == start_next
            
            // speichere vorher den block, der auf prior block zeigt.
            // merge mit vorgänger
                // setz adress of new block auf adress prior block 
                // setz size of new block auf size_prior + size_new_block
            // merge mit nachfolger
                // setz size of new block auf size_next + size_new_block
            // setze next block vom vorgänger von prior auf den neuen block
            // setze next block vom neuen block auf den block auf den next_block.next gezeigt hat
            
        }
        else if new_block_end == next_block_addr {
            // case 2: end_new == start_next

            // merge mit nachfolger
                // setz size of new block auf size_next + size_new_block
            // setze next block vom neuen block auf den block auf den next_block.next gezeigt hat
        }
    }

    if prior_block_end == addr{
        // case 3: end_prior == start_new

        // speichere vorher den block, der auf prior block zeigt.
        // merge mit vorgänger
            // setz adress of new block auf adress prior block 
            // setz size of new block auf size_prior + size_new_block
        // setze next block vom vorgänger von prior auf den neuen block   
    }
    else{
        // case 4: no merge
        kprintln!("no merge inserted between above mentioned blocks");
        // Insert the new node
        (*node_ptr).next = (*current_ptr).next.take();               // Link the new node to the next node
        (*current_ptr).next = Some(&mut *node_ptr); // Link the current node to the new node
    }

}