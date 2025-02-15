use crate::kernel::syscall::user_api::SYSNO_READ;
use crate::kernel::syscall::user_api::syscall0;

#[no_mangle]
pub extern "C" fn sys_read(buff: *mut u8, len: u64) -> i64{
   //Teste das schreiben in 
   let mut bytes_read: u64 = 0;
   let text = b"read-text-successfully-stored-test";
   let text_len = text.len() as u64;

   if len > text_len{ //check ob überhaupt genug Platz
      for i in 0..text_len { // zu len ändern wenn Funktionalität kommt
         unsafe {
            *buff.add(i as usize) = text[i as usize];
         }
      bytes_read += 1;
      }
   }

   // Was zurückgeben??? Anzahl geschriebener Bytes als Kontrolle???
   bytes_read as i64
}
