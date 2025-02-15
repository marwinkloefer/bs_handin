/* ╔═════════════════════════════════════════════════════════════════════════╗
   ║ Module: pit                                                             ║
   ╟─────────────────────────────────────────────────────────────────────────╢
   ║ Descr.: Programmable Interval Timer.                                    ║
   ╟─────────────────────────────────────────────────────────────────────────╢
   ║ Author:  Michael Schoettner, HHU, 15.6.2023                             ║
   ╚═════════════════════════════════════════════════════════════════════════╝
*/
#![allow(dead_code)]

use alloc::boxed::Box;
use core::ptr;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use crate::devices::cga;
use crate::kernel::cpu;
use crate::kernel::interrupts::int_dispatcher;
use crate::kernel::interrupts::isr;
use crate::kernel::interrupts::pic;
use crate::kernel::threads::scheduler;
use crate::kernel::threads::scheduler::SCHEDULER;
use crate::kernel::threads::thread;

// read systime
pub fn get_systime() -> u64 {
    SYS_TIME.load(Ordering::SeqCst)
}

// Ports
const PORT_CTRL: u16 = 0x43;
const PORT_DATA0: u16 = 0x40;

// system time ticks (each 10ms one incremented)
static SYS_TIME: AtomicU64 = AtomicU64::new(0);

// index for displaying spinner
static SYS_TIME_DISPLAY: AtomicUsize = AtomicUsize::new(0);

/**
  Description: Configure pit to fire an interrupt after `x` microseconds. \

*/
pub fn interval(x: u32) {
    let time_base = 838; /* ns */
    let duration: u32;
    let timer_interval = x;
    let duration = (x * 1000 + time_base / 2) / time_base;

    // Counter 0, Mode 3 (square wave), access mode lobyte/hibyte, 16-Bit binary format
    cpu::outb(PORT_CTRL, 0x36);
    cpu::outb(PORT_DATA0, (duration & 0xff) as u8);
    cpu::outb(PORT_DATA0, ((duration & 0xff00) >> 8) as u8);
}

/**
 Description: Configure pit using `interval` to fire an interrupt each 10ms.  \
              Then register `trigger` in interrupt dispatcher and allow the \
              timer IRQ in the PIC.

 Parameters: \
            `f` frequency of musical note \
            `d` duration in ms
*/
pub fn plugin() {
    interval(10000); // configure 10ms
    int_dispatcher::register(int_dispatcher::INT_VEC_TIMER, Box::new(PitISR));
    pic::allow(pic::IRQ_TIMER);
}

struct PitISR;

impl isr::ISR for PitISR {
    /**
     Description: ISR of the pit.
    */
    fn trigger(&self) {
        let spinner: [char; 4] = ['/', '-', '\\', '|'];

        // progress system time by one tick
        SYS_TIME.fetch_add(1, Ordering::SeqCst);

        // Rotate the spinner each 100 ticks. One tick is 10ms, so the spinner
        // rotates 360 degress in about 1s
        if get_systime() % 100 == 0 {
            let mut index = SYS_TIME_DISPLAY.load(Ordering::SeqCst);
            index = (index + 1) % 4;
            SYS_TIME_DISPLAY.store(index, Ordering::SeqCst);

            // 'Uhrzeiger' ausgeben; show aendert nicht die Cursorposition
            cga::show(79, 0, spinner[index], cga::Color::LightRed as u8);
        }

        // We try to switch to the next thread
        let (mut now, mut then) = (ptr::null_mut(), ptr::null_mut());
        {
            // Scheduler might be locked, in that case we give up preemption
            let maybe_guard = SCHEDULER.try_lock();
            if maybe_guard.is_some() {
                // check if we can switch, and if yes, 'prepare_preempt' will update
                // the status information of the scheduler
                (now, then) = maybe_guard.unwrap().prepare_preempt();
            }
        }
        if !now.is_null() && !then.is_null() {
         // everything worked, so now we switch
         thread::Thread::switch(now, then);
     }
}
}
