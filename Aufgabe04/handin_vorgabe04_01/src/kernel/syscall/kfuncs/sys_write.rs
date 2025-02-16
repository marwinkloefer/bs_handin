use crate::kernel::syscall::user_api::SYSNO_WRITE;
use crate::kernel::syscall::user_api::syscall0;

#[no_mangle]
pub extern "C" fn sys_write(buff: *const u8, len: u64) -> i64{
   // Lauf-Variable für die bereits ausgegebenen chars 
   let mut bytes_written: u64 = 0;

   for i in 0..len {
       let byte = unsafe { *buff.add(i as usize) };
       kprint!("{}", byte as char);
       bytes_written += 1;
   }
   kprint!("\n");
   // Was zurückgeben??? Anzahl geschriebener Bytes als Kontrolle???
   bytes_written as i64
}
