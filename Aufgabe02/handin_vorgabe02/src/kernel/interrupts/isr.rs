/* ╔═════════════════════════════════════════════════════════════════════════╗
   ║ Module: isr                                                             ║
   ╟─────────────────────────────────────────────────────────────────────────╢
   ║ Descr.: Definition of the interface for an Interrupt Service Routine.   ║
   ║         Must be implemented by a device driver if it needs to handle    ║
   ║         interrupts. The ISR is registered using 'register' in           ║
   ║         'intdispatchter.rs'.                                            ║
   ╟─────────────────────────────────────────────────────────────────────────╢
   ║ Author: Michael Schoetter, Univ. Duesseldorf, 10.3.2022                 ║
   ╚═════════════════════════════════════════════════════════════════════════╝
*/

// Definition of Interrupt Service Routine
pub trait ISR {
    fn is_default_isr(&self) -> bool {
        return false;
    }
    fn trigger(&self);
}

// Default ISR needed by intdispatcher
#[derive(Copy, Clone)]
pub struct Default;

impl ISR for Default {
    fn is_default_isr(&self) -> bool {
        return true;
    }

    fn trigger(&self) {}
}
