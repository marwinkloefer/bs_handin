/* ╔═════════════════════════════════════════════════════════════════════════╗
   ║ Module: cga                                                             ║
   ╟─────────────────────────────────────────────────────────────────────────╢
   ║ Descr.: This module provides functions for doing output on the CGA text ║
   ║         screen. It also supports a text cursor position stored in the   ║
   ║         hardware using ports.                                           ║
   ╟─────────────────────────────────────────────────────────────────────────╢
   ║ Author: Michael Schoetter, Univ. Duesseldorf, 6.2.2024                  ║
   ╚═════════════════════════════════════════════════════════════════════════╝
*/
use crate::kernel::cpu;

// make type comparable, printable and enable copy semantics
#[allow(dead_code)] // avoid warnings for unused colors
#[repr(u8)] // store each enum variant as an u8
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Pink = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    LightPink = 13,
    Yellow = 14,
    White = 15,
}

pub const CGA_STD_ATTR: u8 = (Color::Black as u8) << 4 | (Color::Green as u8);
pub const CGA_BASE_ADDR: u64 = 0xb8000;

const CGA_ROWS: u64 = 25;
const CGA_COLUMNS: u64 = 80;

/**
 Description: Display the `character` at the given position `x`,`y` with attribute `attrib`
*/
pub fn show(x: u64, y: u64, character: char, attrib: u8) {
    let pos: u64;

    if x > CGA_COLUMNS || y > CGA_ROWS {
        return;
    }

    pos = (y * CGA_COLUMNS + x) * 2;

    unsafe {
        *((CGA_BASE_ADDR + pos) as *mut u8) = character as u8;
        *((CGA_BASE_ADDR + pos + 1) as *mut u8) = attrib;
    }
}

/**
 Description: Print byte `b` at actual position cursor position `x`,`y`
*/
pub fn print_byte(mut x: u64, mut y: u64, b: u8) -> (u64, u64) {
    //let (mut x, mut y) = getpos();

    if b == ('\n' as u8) {
        x = 0;
        y += 1;
        if y >= CGA_ROWS {
            scrollup();
            y -= 1;
        }
    } else {
        show(x, y, b as char, CGA_STD_ATTR);
        x += 1;
        if x >= CGA_COLUMNS {
            x = 0;
            y += 1;
            if y >= CGA_ROWS {
                scrollup();
                y -= 1;
            }
        }
    }
    (x, y)
}

/**
 Description: Scroll text lines by one to the top.
*/
pub fn scrollup() {
    let mut dst_off: u64 = 0;
    let mut src_off: u64 = CGA_COLUMNS * 2;
    let mut counter: u64;

    counter = (CGA_ROWS - 1) * (CGA_COLUMNS * 2);

    // Zeilen nach oben schieben
    while counter > 0 {
        unsafe {
            *((CGA_BASE_ADDR + dst_off) as *mut u8) = *((CGA_BASE_ADDR + src_off) as *mut u8);
        }
        counter -= 1;
        dst_off += 1;
        src_off += 1;
    }

    // untere Zeile mit Leerzeichen fuellen
    for x in 0..80 {
        show(x, 24, ' ', CGA_STD_ATTR);
    }
}

/**
 Description: Helper function returning an attribute byte for the given
              parameters `bg`, `fg`, and `blink`
*/
pub fn attribute(bg: Color, fg: Color, blink: bool) -> u8 {
    let mut ret = (((bg as u8) & 0x7) << 4) | ((fg as u8) & 0xf);

    if blink == true {
        ret |= 0x80
    }
    ret
}
