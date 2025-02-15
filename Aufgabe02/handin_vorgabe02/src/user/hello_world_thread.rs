use crate::kernel::cpu;
use crate::kernel::syscall::user_api::{usr_getlastkey, usr_gettid, usr_hello_world, usr_read, usr_write};
use crate::kernel::threads::scheduler;

pub extern "C" fn hello_world_thread_entry() {
    //  let tid = scheduler::get_active_tid();
    //  println!("Hello World! thread-id = {}", tid);
    //  kprintln!("Hello World");
    //  let val = cpu::inb(1);
    
    /* Teste Syscalls */
    // tests 0 and 1 in one
    test_syscalls(0);

    loop {
        print!("U");

        let mut x: u64 = 0;
        loop {
            x = x + 1;
            if x > 100000000 {
                break;
            }
        }
    }
}

fn test_syscalls(call_id: u8) {
    match call_id {
        0 => usr_hello_world(), // teste Funktionsweise sys_hello_word aus Ring 3 heraus
        1 => usr_gettid(),      // teste Funktionsweise sys_gettid aus Ring 3 heraus
        2 => usr_getlastkey(),  // teste Funktionsweise sys_getlastkey aus Ring 3 heraus
        3 => {                  // teste Funktionsweise sys_write aus Ring 3 heraus
            // --------------------------- usr_write -------------------------- //
            const BUFFER_LENGTH_WRITE: usize = 64;
            let mut buffer: [u8; BUFFER_LENGTH_WRITE] = [0; BUFFER_LENGTH_WRITE];
            let message = "Buffer-to-output-write-test";
            for (i, &byte) in message.as_bytes().iter().enumerate() {
                buffer[i] = byte;
            }
            usr_write(buffer.as_ptr(), message.len() as u64);
            // --------------------------- usr_write -------------------------- //
        },
        4 => {                  // teste Funktionsweise sys_read aus Ring 3 heraus
            // --------------------------- usr_read --------------------------- //
            const BUFFER_LENGTH_READ: usize = 64; 
            let mut buffer: [u8; BUFFER_LENGTH_READ] = [0; BUFFER_LENGTH_READ];
            usr_read(buffer.as_mut_ptr(), BUFFER_LENGTH_READ as u64);
            usr_write(buffer.as_ptr(), BUFFER_LENGTH_READ as u64);
            // --------------------------- usr_read --------------------------- //
        },
        _ => {}
    }
}