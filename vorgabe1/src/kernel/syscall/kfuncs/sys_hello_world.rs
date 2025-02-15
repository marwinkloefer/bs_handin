
use crate::kernel::threads::scheduler;
use crate::kernel::syscall::user_api::SYSNO_HELLO_WORLD;
use crate::kernel::syscall::user_api::syscall0;



#[no_mangle]
pub extern "C" fn sys_hello_world() {
   kprintln!("Hello World from user thread tid={}", scheduler::get_active_tid() );
}
