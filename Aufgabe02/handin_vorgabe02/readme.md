# Aufgabe 2

Rufe in **hello_world_thread_entry()** die geschriebene funktion **test_syscalls(call_id: u8)** auf die basierend auf den eingabe Zahlen die einzelnen Syscalls testet. AKtuell ist 0 gestetzt was dazu führt, dass **sys_hello_world()** getestet wird.




"""

kickoff_kernel_thread, tid=0, old_rsp0 = 481f30, is_kernel_thread: true

preempt: tid=0, old_rsp0=481f30 and switch to tid=1, old_rsp0=581fa8

kickoff_kernel_thread, tid=1, old_rsp0 = 581fa8, is_kernel_thread: false

preempt: tid=1, old_rsp0=581fa8 and switch to tid=0, old_rsp0=481a78

preempt: tid=0, old_rsp0=481a78 and switch to tid=1, old_rsp0=581be8

Hello World from user thread tid=1

preempt: tid=1, old_rsp0=581be8 and switch to tid=0, old_rsp0=481a78

preempt: tid=0, old_rsp0=481a78 and switch to tid=1, old_rsp0=581be8

preempt: tid=1, old_rsp0=581be8 and switch to tid=0, old_rsp0=481a78

"""