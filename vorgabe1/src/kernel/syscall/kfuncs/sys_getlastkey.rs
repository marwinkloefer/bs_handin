
use crate::devices::keyboard;
use crate::devices::keyboard::get_lastkey;
use crate::kernel::syscall::user_api::SYSNO_GETLASTKEY;
use crate::kernel::syscall::user_api::syscall0;
use crate::mylib::input::getch;



#[no_mangle]
pub extern "C" fn sys_getlastkey() -> u64{
   //let ret = keyboard::get_lastkey();
   let ret = getch();
   kprintln!("Last Key = {}", ret as char);
   ret as u64
}
