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
// Muss mit NO_SYSCALLS in 'kernel/syscall/syscalls.asm' konsistent sein!
pub const NO_SYSCALLS: usize = 5;

// Funktionsnummern aller Systemaufrufe
pub const SYSNO_HELLO_WORLD: usize = 0;
pub const SYSNO_WRITE: usize = 1;
pub const SYSNO_READ: usize = 2;
pub const SYSNO_GETLASTKEY: usize = 3;
pub const SYSNO_GETTID: usize = 4;

/* 
 * Hier muss Code eingefuegt werden 
 */


pub fn usr_hello_world() {
   syscall0(SYSNO_HELLO_WORLD as u64);
}

pub fn usr_getlastkey() {
    syscall0(SYSNO_GETLASTKEY as u64);
}

pub fn usr_gettid() {
    syscall0(SYSNO_GETTID as u64);
}

pub fn usr_read(buff: *mut u8, len: u64) {
    syscall2(SYSNO_READ as u64, buff as u64, len);
}

pub fn usr_write(buff: *const u8, len: u64) {
    syscall2(SYSNO_WRITE as u64, buff as u64, len);
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
 *       x86_64-abi-0.99 Kapitel 3.2.3 Parameter Passing:
 *          - If the class is INTEGER, the next available register of the sequence %rdi, %rsi, %rdx, %rcx, %r8 and %r9 is used
 *              -> %rdi used to pass 1st argument to functions 
 *              -> %rsi used to pass 2nd argument to functions
 *              -> %rdx used to pass 3rd argument to functions; 2nd return register
 *              -> %rcx used to pass 4th integer argument to functions
 */

#[inline(always)]
#[allow(unused_mut)]
pub fn syscall1(arg0: u64, arg1: u64) -> u64 {
    let mut ret: u64;
    unsafe {
        asm!(
            "int 0x80",
            inlateout("rax") arg0 => ret,
            in("rdi") arg1,
            options(preserves_flags, nostack)
        );
    }
    ret
}

#[inline(always)]
#[allow(unused_mut)]
pub fn syscall2(arg0: u64, arg1: u64, arg2: u64) -> u64 {
    let mut ret: u64;
    unsafe {
        asm!(
            "int 0x80",
            inlateout("rax") arg0 => ret,
            in("rdi") arg1,
            in("rsi") arg2,
            options(preserves_flags, nostack)
        );
    }
    ret
}