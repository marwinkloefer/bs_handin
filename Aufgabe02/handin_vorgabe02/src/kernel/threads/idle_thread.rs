use crate::kernel::threads::scheduler;

pub extern "C" fn idle_thread_entry() {
    scheduler::set_initialized();
    loop {
        print!("I");

        let mut x: u64 = 0;
        loop {
            x = x + 1;
            if x > 100000000 {
                break;
            }
        }
    }
}
