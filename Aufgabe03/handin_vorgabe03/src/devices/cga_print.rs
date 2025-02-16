/* ╔═════════════════════════════════════════════════════════════════════════╗
   ║ Module: cga_print                                                       ║
   ╟─────────────────────────────────────────────────────────────────────────╢
   ║ Descr.: Implements the macros print! and println! using cga. The macro  ║
   ║         implementation uses a mutex, so they should not be used within  ║
   ║         an interrupt handler!                                           ║
   ╟─────────────────────────────────────────────────────────────────────────╢
   ║ Author: Philipp Oppermann, see here:                                    ║
   ║            https://os.phil-opp.com/vga-text-mode/                       ║
   ╚═════════════════════════════════════════════════════════════════════════╝
*/
use crate::devices::cga;
use core::fmt;
use core::fmt::Write;
use spin::Mutex;

// The global writer that can used as an interface from other modules
// It is threadsafe by using 'Mutex'
pub static WRITER: Mutex<Writer> = Mutex::new(Writer { x: 0, y: 0 });

// Defining a Writer for writing formatted strings to the CGA screen
pub struct Writer {
    x: u64,
    y: u64,
}

// Implementation of the 'core::fmt::Write' trait for our Writer
// Required to output formatted strings
// Requires only one function 'write_str'
impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            match byte {
                // printable ASCII byte or newline
                0x20..=0x7e | b'\n' => (self.x, self.y) = cga::print_byte(self.x, self.y, byte),

                // not part of printable ASCII range
                _ => (self.x, self.y) = cga::print_byte(self.x, self.y, 0xfe),
            }
        }
        Ok(())
    }
}

// Provide macros like in the 'io' module of Rust
// The $crate variable ensures that the macro also works
// from outside the 'std' crate.
macro_rules! print {
    ($($arg:tt)*) => ({
        $crate::cga_print::print(format_args!($($arg)*));
    });
}

macro_rules! println {
    ($fmt:expr) => (print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (print!(concat!($fmt, "\n"), $($arg)*));
}

// Helper function of print macros (must be public)
pub fn print(args: fmt::Arguments) {
    WRITER.lock().write_fmt(args).unwrap();
}
