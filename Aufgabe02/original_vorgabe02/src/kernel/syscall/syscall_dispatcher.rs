/*****************************************************************************
 *                                                                           *
 *                  s y s c a l l _ d i s p a t c h e r                      *
 *                                                                           *
 *---------------------------------------------------------------------------*
 * Beschreibung:    Alle Systemaufrufe landen vom Assembler-Coder hier und   *
 *                  werden anhand der Funktionsnummerund der Funktions-      *
 *                  tabelle weitergeleitet.                                  *
 *                                                                           *
 * Autor:           Stefan Lankes, RWTH Aachen                               *
 *                  Michael Schoettner, 23.10.2024, modifiziert              *
 *****************************************************************************/
use core::arch::{asm, naked_asm};

use crate::kernel::syscall;
use crate::kernel::syscall::sys_funcs::sys_getlastkey::sys_getlastkey;
use crate::kernel::syscall::sys_funcs::sys_gettid::sys_gettid;
use crate::kernel::syscall::sys_funcs::sys_hello_world::sys_hello_world;
use crate::kernel::syscall::sys_funcs::sys_read::sys_read;
use crate::kernel::syscall::sys_funcs::sys_write::sys_write;
use crate::kernel::syscall::user_api;

extern "C" {
    fn _init_syscalls();
}

// IDT-Eintrag fuer Systemaufrufe einrichten (in 'syscalls.asm')
pub fn init() {
    unsafe {
        _init_syscalls();
    }
}

#[no_mangle]
pub static SYSCALL_FUNCTABLE: SyscallFuncTable = SyscallFuncTable::new();

#[repr(align(64))]
#[repr(C)]
pub struct SyscallFuncTable {
    handle: [*const usize; user_api::NO_SYSCALLS],
}

impl SyscallFuncTable {
    pub const fn new() -> Self {
        SyscallFuncTable {
            handle: [
                sys_hello_world as *const _,
                sys_write as *const _,
                sys_read as *const _,
                sys_getlastkey as *const _,
                sys_gettid as *const _,
            ],
        }
    }
}

unsafe impl Send for SyscallFuncTable {}
unsafe impl Sync for SyscallFuncTable {}

/*****************************************************************************
 * Funktion:        syscall_disp                                             *
 *---------------------------------------------------------------------------*
 * Beschreibung:    Wenn ein System-Aufruf ueber int 0x80 ausgeloest wurde   *
 *                  ruft der Assembler-Handler '_syscall_handler' diese      *
 *                  Rust-Funktion auf. Das Sichern und Wiederherstellen der  *
 *                  Register wird schon in Assembler erledigt.               *
 *****************************************************************************/
 #[naked]
 #[no_mangle]
pub unsafe extern "C" fn syscall_disp() {
    naked_asm!(
						"call [{syscall_functable}+8*rax]",
						"ret",
      syscall_functable = sym SYSCALL_FUNCTABLE);
//    		options(noreturn));
}

/*****************************************************************************
 * Funktion:        syscall_abort                                            *
 *---------------------------------------------------------------------------*
 * Beschreibung:    Falls eine unbekannte Funktionsnummer verwendet wurde,   *
 *                  ruft der Assembler-Code diese Funktion auf, um eine      *
 *                  panic auszuloesen.                                       *
 *****************************************************************************/
 #[no_mangle]
pub unsafe extern "C" fn syscall_abort() {
    let sys_no: u64;

    asm!(
        "mov {}, rax", out(reg) sys_no
    );

    panic!("Systemaufruf mit Nummer {} existiert nicht!", sys_no);
}
