
use crate::kernel::syscall::user_api::SYSNO_GETTID;
use crate::kernel::syscall::user_api::syscall0;
use crate::kernel::threads::scheduler;



#[no_mangle]
pub extern "C" fn sys_gettid() -> i64{
   let ret = scheduler::get_active_tid();
   //kprintln!("Current Thread ID = {}", ret);
   ret as i64
}