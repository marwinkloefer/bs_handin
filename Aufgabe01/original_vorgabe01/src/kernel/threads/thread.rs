/* ╔═════════════════════════════════════════════════════════════════════════╗
   ║ Module: thread                                                          ║
   ╟─────────────────────────────────────────────────────────────────────────╢
   ║ Descr.: Functions for creating, starting, switching and ending threads. ║
   ╟─────────────────────────────────────────────────────────────────────────╢
   ║ Autor:  Michael Schoettner, 11.06.2024                                  ║
   ╚═════════════════════════════════════════════════════════════════════════╝
*/
use alloc::boxed::Box;
use core::fmt;

use crate::consts;
use crate::devices::cga;
use crate::kernel::cpu;
use crate::kernel::threads::scheduler;
use crate::kernel::threads::stack;
use crate::mylib::queue::Link;

// Diese Funktionen sind in 'thread.asm'
extern "C" {
    fn _thread_kernel_start(old_rsp0: u64);
    fn _thread_user_start(old_rsp0: u64);
    fn _thread_switch(now_rsp0: *mut u64, then_rsp0: u64, then_rsp0_end: u64);
}

// Diese Funktion (setzt den Kernel-Stack im TSS) ist in 'boot.asm'
extern "C" {
    fn _tss_set_rsp0(old_rsp0: u64);
}

// Verwaltungsstruktur fuer einen Thread
#[repr(C)]
pub struct Thread {
    tid: usize,
    is_kernel_thread: bool,
    old_rsp0: u64, // letzter genutzter Stackeintrag im Kernel-Stack
    // der User-Stack-Ptr. wird auto. durch die Hardware gesichert
    
    
    // Speicher fuer den User-Stack
    /*
        Hier muss Code eingefuegt werden
    */

    
    kernel_stack: Box<stack::Stack>, // Speicher fuer den Kernel-Stack
    entry: extern "C" fn(),
}

impl Thread {
    // Neuen Thread anlegen
    pub fn new(my_tid: usize, myentry: extern "C" fn(), kernel_thread: bool) -> Box<Thread> {

        // Speicher fuer die Stacks anlegen
        let my_kernel_stack = stack::Stack::new(consts::STACK_SIZE);


        /*
           Hier muss Code eingefuegt werden
        */

        // Thread-Objekt anlegen
        let mut threadobj = Box::new(Thread {
            tid: my_tid,
            is_kernel_thread: kernel_thread,
            old_rsp0: 0,
            kernel_stack: my_kernel_stack,
            entry: myentry,
        });

        threadobj.prepare_kernel_stack();

        threadobj
    }

    // Starten des 1. Kernel-Threads (rsp0 zeigt auf den praeparierten Stack)
    // Wird vom Scheduler gerufen, wenn dieser gestartet wird.
    // Alle anderen Threads werden mit 'switch' angestossen
    pub fn start(now: *mut Thread) {
        unsafe {
            kprintln!("thread start, kernel-stack = {:x}", (*now).old_rsp0);
            _thread_kernel_start((*now).old_rsp0);
        }
    }

    // Umschalten von Thread 'now' auf Thread 'then'
    pub fn switch(now: *mut Thread, then: *mut Thread) {
        unsafe {
            kprint!(
                "preempt: tid={}, old_rsp0={:x}",
                Thread::get_tid(now),
                (*now).old_rsp0
            );
            kprintln!(
                " and switch to tid={}, old_rsp0={:x}",
                Thread::get_tid(then),
                (*then).old_rsp0
            );
            _thread_switch(
                &mut (*now).old_rsp0,
                (*then).old_rsp0,
                (*then).kernel_stack.stack_end() as u64,
            );
        }
    }

    //
    // Kernel-Stack praeparieren, fuer das Starten eines Threads im Ring 0
    // (wird in '_thread_kernel_start' und '_thread_switch' genutzt)
    // Im Wesentlichen wird hiermit der Stack umgeschaltet und durch
    // einen Ruecksprung die Funktion 'kickoff_kernel_thread' angesprungen.
    //
    // Die Interrupt werden nicht aktiviert.
    //
    fn prepare_kernel_stack(&mut self) {
        let kickoff_kernel_addr = kickoff_kernel_thread as *const ();
        let object: *const Thread = self;

        // sp0 zeigt ans Ende des Speicherblocks, passt somit
        let sp0: *mut u64 = self.kernel_stack.stack_end();

        // Stack initialisieren. Es soll so aussehen, als waere soeben die
        // die Funktion '_thread_kernel_start' aufgerufen worden. Diese
        // Funktion hat als Parameter den Zeiger "object" erhalten.
        // Da der Aufruf "simuliert" wird, kann fuer die Ruecksprung-
        // adresse in 'kickoff_kernel_addr' nur ein unsinniger Wert eingetragen
        // werden. Die Funktion 'kickoff_kernel_addr' muss daher dafuer sorgen,
        // dass diese Adresse nie benoetigt, sie darf also nicht zurueckspringen,
        // sonst kracht's.
        unsafe {
            *sp0 = 0x00DEAD00 as u64; // dummy Ruecksprungadresse

            *sp0.offset(-1) = kickoff_kernel_addr as u64; // Adresse von 'kickoff_kernel_thread'

            // Nun sichern wir noch alle Register auf dem Stack
            *sp0.offset(-2) = 2; // rflags (IOPL=0, IE=0)
            *sp0.offset(-3) = 0; // r8
            *sp0.offset(-4) = 0; // r9
            *sp0.offset(-5) = 0; // r10
            *sp0.offset(-6) = 0; // r11
            *sp0.offset(-7) = 0; // r12
            *sp0.offset(-8) = 0; // r13
            *sp0.offset(-9) = 0; // r14
            *sp0.offset(-10) = 0; // r15

            *sp0.offset(-11) = 0; // rax
            *sp0.offset(-12) = 0; // rbx
            *sp0.offset(-13) = 0; // rcx
            *sp0.offset(-14) = 0; // rdx

            *sp0.offset(-15) = 0; // rsi
            *sp0.offset(-16) = object as u64; // rdi -> 1. Param. fuer 'kickoff_kernel_thread'
            *sp0.offset(-17) = 0; // rbp

            // Zum Schluss speichern wir den Zeiger auf den zuletzt belegten
            // Eintrag auf dem Stack in 'rsp0'. Daruber gelangen wir in
            // _thread_kernel_start an die noetigen Register
            self.old_rsp0 = (sp0 as u64) - (8 * 17); // aktuellen Stack-Zeiger speichern
        }
    }


    

 
    //
    // Diese Funktion wird verwendet, um einen Thread vom Ring 0 in den
    // Ring 3 zu versetzen. Dies erfolgt wieder mit einem praeparierten Stack.
    // Hier wird ein Interrupt-Stack-Frame gebaut, sodass beim Ruecksprung
    // mit 'iretq' die Privilegstufe gewechselt wird. Wenn alles klappt
    // landen wir in der Funktion 'kickoff_user_thread' und sind dann im Ring 3
    // 
    // In den Selektoren RPL = 3, RFLAGS = IOPL=0, IE=1
    //
    // Die Interrupt werden durch den 'iretq' aktiviert.
    //
    fn switch_to_usermode(&mut self) {

        /*
            Hier muss Code eingefuegt werden
        */

        // Interrupt-Stackframe bauen

        // In den Ring 3 schalten -> Aufruf von '_thread_user_start'
    }

    pub fn get_tid(thread_object: *const Thread) -> usize {
        unsafe { (*thread_object).tid }
    }

    pub fn get_raw_pointer(&mut self) -> *mut Thread {
        self
    }
}

// Notwendig, für die Queue-Implementierung im Scheduler
impl PartialEq for Thread {
    fn eq(&self, other: &Self) -> bool {
        self.tid == other.tid
    }
}

// Notwendig, falls wir die Ready-Queue ausgeben moechten
impl fmt::Display for Thread {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.tid)
    }
}

//
// Dies ist die erste Rust-Funktion, die aufgerufen wird, wenn
// ein neuer Thread startet (im Ring 0). Falls dies ein User-Thread
// ist, so wird von hier aus 'switch_to_usermode' gerufen.
//
// Hier sind die Interrupts noch gesperrt.
//
#[no_mangle]
pub extern "C" fn kickoff_kernel_thread(object: *mut Thread) {
    unsafe {
        kprintln!(
            "kickoff_kernel_thread, tid={}, old_rsp0 = {:x}, is_kernel_thread: {}",
            (*object).tid,
            (*object).old_rsp0,
            (*object).is_kernel_thread
        );
    }

    // Setzen von rsp0 im TSS
    unsafe {
        _tss_set_rsp0((*object).kernel_stack.stack_end() as u64);
    }

    // Falls dies ein User-Thread ist, schalten wir nun in den User-Mode
    // Der Aufruf kehrt nicht zurueck, schaltet aber IE = 1
    // Es geht anschliessend in 'kickoff_user_thread' weiter
    unsafe {
        if (*object).is_kernel_thread == false {
            (*object).switch_to_usermode();
        } else {
            // Interrupts wieder zulassen
            cpu::enable_int();
            ((*object).entry)();
        }
    }
    loop {}
}

//
// Dies ist die  Rust-Funktion, die aufgerufen wird, wenn ein
// Kernel-Thread (Ring 0) in den Ring 3 versetzt wird
//
#[no_mangle]
pub extern "C" fn kickoff_user_thread(object: *mut Thread) {
    // Einstiegsfunktion des Threads aufrufen

    /*
        Hier muss Code eingefuegt werden
    */

    loop {}
}
