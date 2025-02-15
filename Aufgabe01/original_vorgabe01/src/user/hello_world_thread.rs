use crate::kernel::cpu;
use crate::kernel::threads::scheduler;

pub extern "C" fn hello_world_thread_entry() {
    //    let tid = scheduler::get_active_tid();
    //    println!("Hello World! thread-id = {}", tid);
    // kprintln!("Hello World");
    //    let val = cpu::inb(1);
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
