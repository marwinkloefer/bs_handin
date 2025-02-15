/* ╔═════════════════════════════════════════════════════════════════════════╗
   ║ Module: cpu                                                             ║
   ╟─────────────────────────────────────────────────────────────────────────╢
   ║ Descr.: Different cpu functions are implemented here.                   ║
   ╟─────────────────────────────────────────────────────────────────────────╢
   ║ Author: Michael Schoetter, Univ. Duesseldorf, 9.6.2024                  ║
   ╚═════════════════════════════════════════════════════════════════════════╝
*/

use core::arch::asm;

/**
 Description:
    Write one byte to a port

 Parameters: \
   `port` port address, 16 bit \
   `data` data to be written, 8 bit
*/
#[inline]
pub fn outb(port: u16, data: u8) {
   unsafe {
      asm!(
         "out dx, al",
         in("dx") port,
         in("al") data,
      );
   }
}


/**
 Description: Read one byte from a port

 Parameters: \
    `port` port address, 16 bit \
 Return: \
   `data` data read, 8 bit
*/
pub fn inb(port: u16) -> u8 {
   let ret: u8;
   unsafe {
      asm!(
         "in al, dx",
         in("dx") port,
         out("al") ret,
      );
   }
   ret
}

/**
 Description: Check if IE bit is set in RFLAGS \

 Return: \
   `true` if IE is set, `false` otherwise
*/
#[inline]
pub fn is_int_enabled() -> bool {
	let rflags: u64;

	unsafe { asm!("pushf; pop {}", lateout(reg) rflags, options(nomem, nostack, preserves_flags)) };
	if (rflags & (1u64 << 9)) != 0 {
		return true;
	}
	false
}


/**
 Description: clear IE bit in RFLAGS \

 Return: \
   `true` if IE was set already, `false` otherwise
*/
#[inline]
pub fn disable_int_nested() -> bool {
	let was_enabled = is_int_enabled();
	disable_int();
	was_enabled
}

/**
 Description: set IE bit in RFLAGS only iff `was_enabled` 
   is `true` otherwise do nothing.
*/
#[inline]
pub fn enable_int_nested(was_enabled: bool) {
	if was_enabled == true {
		enable_int();
	}
}

/**
 Description: set IE bit in RFLAGS
*/
#[inline]
pub fn enable_int () {
   unsafe { asm!( "sti" ); }
}
    
    
/**
 Description: clear IE bit in RFLAGS
*/
#[inline]
pub fn disable_int () {
   unsafe { asm!( "cli" ); }
}


/**
 Description: stop CPU, will be waked up by next interrupt
*/
#[inline]
pub fn halt () {
   loop {
      unsafe { asm!( "hlt" ); }
   }
}

/**
 Description: return RFLAGS
*/
#[inline]
pub fn getflags () -> u64 {
   let rflags: u64;
   unsafe {
       asm! ("pushfq; pop {}", out(reg) rflags, options(nomem, preserves_flags));
   }
   rflags  
}

/**
 Description: spin loop hint
*/
pub fn pause() {
    unsafe {
        asm!("pause", options(nomem, nostack));
    }
}
