use crate::kernel::paging::frames::PhysAddr;
use crate::kernel::paging::pages;


// Diese Funktionen sind in 'thread.asm'
extern "C" {
    fn _thread_kernel_start(old_rsp0: u64);
    fn _thread_user_start(old_rsp0: u64);
    fn _thread_switch(now_rsp0: *mut u64, then_rsp0: u64, then_rsp0_end: u64, then_pml4: PhysAddr);
    fn _thread_set_segment_register();
}


// Verwaltungsstruktur fuer einen Thread
#[repr(C)]
pub struct Thread {
    tid: u64,
    is_kernel_thread: bool,
    pml4_addr: PhysAddr, // Einstieg in die Seitentabellen
    old_rsp0: u64,       // letzter genutzter Stackeintrag, Kernel-Stack
    // der User-Stack-Ptr. wird auto. durch die Hardware gesichert
    user_stack: Box<stack::Stack>,   // Speicher fuer den User-Stack
    kernel_stack: Box<stack::Stack>, // Speicher fuer den Kernel-Stack
    entry: extern "C" fn(),
}

impl Thread {

    // Neuen Thread anlegen
    pub fn new(myentry: extern "C" fn(), kernel_thread: bool) -> Box<Thread> {
        let mytid = scheduler::get_next_tid();

        // Page-Tables anlegen
        let new_pml4_addr = pages::pg_init_kernel_tables();


        // ...
    }
    
    
    // Starten des 1. Kernel-Threads (rsp0 zeigt auf den praeparierten Stack)
    // Wird vom Scheduler gerufen, wenn dieser gestartet wird.
    // Alle anderen Threads werden mit 'switch' angestossen
    pub fn start(now: *mut Thread) {
        unsafe {
            pages::pg_set_cr3(now.as_ref().unwrap().pml4_addr); // Adressraum setzen
            _thread_kernel_start((*now).old_rsp0);
        }
    }

    // Umschalten von Thread 'now' auf Thread 'then'
    pub fn switch(now: *mut Thread, then: *mut Thread) {
        /* 
         *   Hier muss Code geaendert werden
         */
    }
}
