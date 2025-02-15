/*****************************************************************************
 *                                                                           *
 *                  u s e r _ a p i                                          *
 *                                                                           *
 *---------------------------------------------------------------------------*
 * Beschreibung:    Alle Systemaufrufe landen vom Assembler-Coder hier und   *
 *                  werden anhand der Funktionsnummerund der Funktions-      *
 *                  tabelle weitergeleitet.                                  *
 *                                                                           *
 * Autor:           Stefan Lankes, RWTH Aachen University                    *
 *                  Licensed under the Apache License, Version 2.0 or        *
 *                  the MIT license, at your option.                         *
 *                                                                           *
 *                  Michael Schoettner, 14.9.2023, modifiziert               * 
 *****************************************************************************/

use core::arch::asm;



// Anzahl an Systemaufrufen
// Muss mit NO_SYSCALLS in 'kernel/interrupts/interrupts.asm' konsistent sein!
pub const NO_SYSCALLS: usize = 1;

// Funktionsnummern aller Systemaufrufe
pub const SYSNO_HELLO_WORLD: usize = 0;
/* 
 * Hier muss Code eingefuegt werden 
 */


pub fn usr_hello_world() {
   syscall0(SYSNO_HELLO_WORLD as u64);
}

/* 
 * Hier muss Code eingefuegt werden 
 */



#[inline(always)]
#[allow(unused_mut)]
pub fn syscall0(arg0: u64) -> u64 {
    let mut ret: u64;
    unsafe {
        asm!("int 0x80",
            inlateout("rax") arg0 => ret,
            options(preserves_flags, nostack)
        );
    }
    ret
}


/* 
 * Hier muss Code eingefuegt werden 
 */
