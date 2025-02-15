/* ╔═════════════════════════════════════════════════════════════════════════╗
   ║ Module: scheduler                                                       ║
   ╟─────────────────────────────────────────────────────────────────────────╢
   ║ Descr.: A basic round-robin scheduler for cooperative threads.          ║
   ║         No priorties supported.                                         ║
   ╟─────────────────────────────────────────────────────────────────────────╢
   ║ Autor:  Michael Schoettner, HHU, 14.6.2024                              ║
   ╚═════════════════════════════════════════════════════════════════════════╝
*/
use alloc::boxed::Box;
use core::ptr;
use core::sync::atomic::AtomicUsize;
use spin::Mutex;

use crate::devices::cga;
use crate::kernel::cpu;
use crate::kernel::threads::thread;
use crate::mylib::queue;

static THREAD_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub fn next_thread_id() -> usize {
    THREAD_ID_COUNTER.fetch_add(1, core::sync::atomic::Ordering::SeqCst)
}

pub static SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::new());

/**
 Description: Return callers thread ID
*/
pub fn get_active_tid() -> usize {
    thread::Thread::get_tid(SCHEDULER.lock().active)
}

/**
 Description: Get active thread (used before calling 'block')
*/
pub fn get_active() -> Box<thread::Thread> {
    let act;

    let irq = cpu::disable_int_nested();
    unsafe {
        let a = SCHEDULER.lock().active;
        act = Box::from_raw(a);
    }
    cpu::enable_int_nested(irq);
    act
}

/**
 Description: Set initialized flag
*/
pub fn set_initialized() {
    SCHEDULER.lock().initialized = true;
}

pub struct Scheduler {
    active: *mut thread::Thread,
    ready_queue: queue::Queue<Box<thread::Thread>>, // auf die CPU wartende Threads
    next_thread_id: u64,
    initialized: bool,
}

// Notwendig, da sonst der Compiler 'SCHEDULER' als nicht akzeptiert
unsafe impl Send for Scheduler {}

impl Scheduler {
    // Scheduler mit Ready-Queue anlegen
    pub const fn new() -> Self {
        Scheduler {
            active: ptr::null_mut(),
            next_thread_id: 0,
            ready_queue: queue::Queue::new(),
            initialized: false,
        }
    }

    /**
     Description: Start the scheduler. Called only once from 'startup'
    */
    pub fn schedule() {
        let next_thread = SCHEDULER.lock().ready_queue.dequeue();
        if let Some(that) = next_thread {
            // convert 'next_thread' into raw pointer.
            // Prevents Rust from deleting it too early but we need to manually call 'drop' later
            let raw = Box::into_raw(that);

            // set active reference in SCHEDULER
            SCHEDULER.lock().active = raw;

            // and start this thread
            thread::Thread::start(raw);
        } else {
            panic!("Panic: no thread, cannot start scheduler");
        }
    }

    /**
        Description: Register new thread in ready queue

        Parameters: \
               `that` thread to be registered
    */
    pub fn ready(that: Box<thread::Thread>) {
        SCHEDULER.lock().ready_queue.enqueue(that);
    }

    /**
        Description: Calling thread terminates. Scheduler switches to next thread.
                     (The thread terminating is not in the ready queue.)
    */
    pub fn exit() {
        // Get next thread from ready queue
        let next = SCHEDULER.lock().ready_queue.dequeue();
        if next.is_none() {
            panic!("Cannot exit thread as there is no other thread to run!");
        }

        // Start next thread
        if let Some(nx) = next {
            let raw = Box::into_raw(nx);
            SCHEDULER.lock().active = raw;
            thread::Thread::start(raw);
        }
    }

    /**
        Description: Yield cpu and switch to next thread
    */
    pub fn yield_cpu() {
        // Get next thread from ready queue
        let next = SCHEDULER.lock().ready_queue.dequeue();
        if next.is_none() {
            return;
        }

        let that = SCHEDULER.lock().active;

        // Re-insert current thread into ready queue
        let bx;
        unsafe {
            // convert raw-Pointer back to Box<Thread>
            bx = Box::from_raw(that);
        }
        SCHEDULER.lock().ready_queue.enqueue(bx);

        // Switch thread
        if let Some(nx) = next {
            let raw = Box::into_raw(nx);
            SCHEDULER.lock().active = raw;
            thread::Thread::switch(that, raw);
        }
    }

    /**
        Description: This function is only called from the ISR of the PIT. \
                     Check if we can switch from the current running thread to another one. \
                     If doable prepare everything and return raw pointers to current and next thread. \
                     The switching of threads is done from within the ISR of the PIT, in order to \
                     release the lock of the scheduler.

        Return: \
               `(current,next)` current thread, next thread (to switch to)
    */
    pub fn prepare_preempt(&mut self) -> (*mut thread::Thread, *mut thread::Thread) {
        // If the scheduler is not initialized, we abort
        if self.initialized == false {
            return (ptr::null_mut(), ptr::null_mut());
        }

        // Check if there is a thread in the ready queue, if not we abort
        let next = self.ready_queue.dequeue();
        if next.is_none() {
            return (ptr::null_mut(), ptr::null_mut());
        }

        // If we are here, we can preempt

        // Insert the current running thread into the ready qeueue
        let current = self.active;
        unsafe {
            self.ready_queue.enqueue(Box::from_raw(current));
        }

        // Set active thread in scheduler and return (current, next)
        if let Some(nx) = next {
            let raw_next = Box::into_raw(nx);
            self.active = raw_next;
            (current, raw_next)
        } else {
            panic!("prepare_preempt failed.");
        }

        // Interrupts werden in Thread_switch in thread.asm wieder zugelassen
        //
    }

}
