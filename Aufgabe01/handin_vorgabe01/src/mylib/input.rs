use crate::devices::keyboard;

const KEY_LF: u8 = 10;
const KEY_CR: u8 = 13;
 

pub fn getch() -> u8 {
    let mut k: u8;

    loop {
        k = keyboard::get_lastkey();
        if k != 0 {
            break;
        }
    }
    k
}

pub fn wait_for_return() {
    loop {
        if keyboard::get_lastkey() == KEY_LF {
            break;
        }
    }
}
