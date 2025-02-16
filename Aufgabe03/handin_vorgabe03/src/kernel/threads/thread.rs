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

// Füge diesen Import hinzu
use core::arch::asm;

use crate::consts;
use crate::devices::cga;
use crate::kernel::cpu;
use crate::kernel::threads::scheduler;
use crate::kernel::threads::stack;
use crate::mylib::queue::Link;

// Test import to get Thread in Ring 3 running
use crate::hello_world_thread::hello_world_thread_entry;

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
    
    
//----Aufgabe 3 Blatt 1: User-Stack eingebaut----------------------------------------------------------------------------------------------
    // User-Level-Threads (laufen im Ring 3) benötigen immer zwei Stacks, einen für den User- und einen für
    // den Kernel-Mode. Der Einfachheit halber allozieren wir immer zwei Stacks, auch für reine KernelThreads.
    user_stack: Box<stack::Stack>, /// Speicher fuer den User-Stack
//-----------------------------------------------------------------------------------------------------------------------------------------

    
    kernel_stack: Box<stack::Stack>, // Speicher fuer den Kernel-Stack
    entry: extern "C" fn(),
}

impl Thread {
    // Neuen Thread anlegen
    pub fn new(my_tid: usize, myentry: extern "C" fn(), kernel_thread: bool) -> Box<Thread> {

        // Speicher fuer die Stacks anlegen
        let my_kernel_stack = stack::Stack::new(consts::STACK_SIZE);

//----Aufgabe 3 Blatt 1: User-Stack eingebaut----------------------------------------------------------------------------------------------
        
        let my_user_stack = stack::Stack::new(consts::STACK_SIZE);
//-----------------------------------------------------------------------------------------------------------------------------------------

        // Thread-Objekt anlegen
        let mut threadobj = Box::new(Thread {
            tid: my_tid,
            is_kernel_thread: kernel_thread,
            old_rsp0: 0,
            user_stack: my_user_stack,
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
        // Adresse in 'kickoff_kernel_addr' nur ein unsinniger Wert eingetragen
        // werden. Die Funktion 'kickoff_kernel_addr' muss daher dafuer sorgen,
        // dass diese Adresse nie benoetigt, sie darf also nicht zurueckspringen,
        // sonst kracht's.
        unsafe {
            *sp0 = 0xDEADDEADDEADDEAD as u64; // dummy Ruecksprungadresse

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
            // Eintrag auf dem Stack in 'rsp0'. Darüber gelangen wir in
            // _thread_kernel_start an die noetigen Register
            self.old_rsp0 = (sp0 as u64) - (8 * 17); // aktuellen Stack-Zeiger speichern
        }
    }


    //-----------------------------------------------------------------------------------------------------------------------------------------
    // START: 1. Blatt, 3. Aufgabe: Threads im Ring 3 starten 
    //-----------------------------------------------------------------------------------------------------------------------------------------
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
        // Interrupt-Stackframe bauen
        // Stackframe gebaut werden, wie er bei einem Interrupt mit Privilegienwechsel
        // Intel User Guide: In 64-bit mode, the size of interrupt stack-frame pushes is fixed at eight bytes.
        //
        // Table 6-8. Stack Usage with Privilege-Level Change
        // IA-32e Mode:
        //     +--------------------------------------------------------------------------------------------------------+
        //     | SS         |   +40   | Stack Segment => Verweist auf Datensegment-Deskriptor in GDT                    |
        //     | RSP        |   +32   | Register Stack Pointer => zeigt auf oberste Adresse des Stacks                  |
        //     | RFLAGS     |   +24   | Flags-Register => siehe https://en.wikipedia.org/wiki/FLAGS_register            |  
        //     | CS         |   +16   | Code Segment => Verweist auf Codesegment-Deskriptor in GDT                      |
        //     | RIP        |    +8   | Instruction Pointer Register => Adresse der nächsten auszuführenden Instruktion |
        //     | Error Code |     0   | Optional                                                                        |
        //     +--------------------------------------------------------------------------------------------------------+
        //             < 8 Bytes | 64 Bit > => 16 hex zahlen

        //Adresse der nächsten auszuführenden Instruktion im User-Mode wird die kickoff_user_thread Funktion
        let kickoff_user_addr = kickoff_user_thread as *const (); 
        //Pointer um _thread_user_start später das benötigte Objekt zu übergeben
        let object: *const Thread = self; 
        // sp0 zeigt ans Ende des Speicherblocks des Kernel-Stacks
        let sp0: *mut u64 = self.kernel_stack.stack_end(); //0xdead im Speicher aus prepare_kernel_stack
        unsafe{
        // xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx Interrupt Stack Start xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx \\
            // ------------------------------ SS = Stack Segment  (see 3.4.2 Segment Selectors) ------------------------------ \\
            // Bit 0-1: Specifies the privilege level of the selector 
            // Bit   3: Specifies the descriptor table to use (0 = GDT, 1 = LDT)
            // Bit 4-x: Selects one of the descriptors in the GDT or LDT
            // Datensegment-Deskriptor 6ter Eintrag in GDT => 101 | GDT => 0 | Ring 3 Usermode => 11 =>> 101011 => 2B 
            // check gdb with "x /g sp0-1"
            *sp0.offset(-1) = 0x000000000000002B; 
            // ------------------------------ RSP = Register Stack Pointer --------------------------------------------------- \\
            // setze RSP auf das Ende des Stacks 
            // check gdb with "x /g sp0-2"
            *sp0.offset(-2) = self.user_stack.stack_end() as u64;
            // ------------------------------ RFLAGS => Flags Register (see https://en.wikipedia.org/wiki/FLAGS_register) ---- \\
            // Top 32-Bit reserved
            // Bit 1 => Reserved, always = 1 | Bit 9 => Interrupt enable flag = 1 => 10 0000 0010 => 202 
            *sp0.offset(-3) = 0x0000000000000202;
            // ------------------------------ CS = Code Segment  (see 3.4.2 Segment Selectors) ------------------------------- \\
            // Bit 0-1: Specifies the privilege level of the selector 
            // Bit   3: Specifies the descriptor table to use (0 = GDT, 1 = LDT)
            // Bit 4-x: Selects one of the descriptors in the GDT or LDT
            // Codesegment-Deskriptor 5ter Eintrag in GDT => 100 | GDT => 0 | Ring 3 Usermode => 11 =>> 100011 => 23
            *sp0.offset(-4) = 0x0000000000000023;
            // ------------------------------ RIP = Instruction Pointer Register --------------------------------------------- \\
            // Wollen laut Aufgabe in 'kickoff_user_thread' „landen“ 
            // => Adresse der nächsten auszuführenden Instruktion kickoff_user_thread setzen
            *sp0.offset(-5) = kickoff_user_addr as u64;
            //*sp0.offset(-6) = 0x0000000000000000; //Error Code
        // xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx Interrupt Stack Ende xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx \\
            // _thread_user_start() erwartet noch ein Objekt, welches es von Stack nimmt 
            *sp0.offset(-6) = object as u64;
            //update old_rsp0 indem wir die 6 gepusht Adressen mit je 8 Byte abziehen
            //self.old_rsp0 = (sp0 as u64) - 6 * 8;
            // In den Ring 3 schalten -> Aufruf von '_thread_user_start' in thread.asm und Aufruf von iretq
            _thread_user_start((sp0 as u64) - 6 * 8); 
        } 
    }



/*
            let oldest_of_old_rsp0: u64 = self.old_rsp0; 
            let sp0_minus_8_mal_6: u64 = (sp0 as u64) - 6 * 8;
            let kernel_stack_end_to_be_sure: u64 = self.kernel_stack.stack_end() as u64;
            let user_stack_end_to_be_sure: u64 = self.user_stack.stack_end() as u64;
            let address_of_user_kick_off: u64 = kickoff_user_addr as u64;
            let sp0alsnormal = sp0;
            let sp0alsderefferenz = *sp0;
*/


    //-----------------------------------------------------------------------------------------------------------------------------------------
    // ENDE: 1. Blatt, 3. Aufgabe: Threads im Ring 3 starten 
    //-----------------------------------------------------------------------------------------------------------------------------------------

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
    unsafe {
        ((*object).entry)();
    }
    loop {}
}
