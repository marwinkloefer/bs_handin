# Aufgabe 1

In der Datei **"\*\*/Abgabe/Aufgabe01/handin_vorgabe01/src/user/hello_world_thread.rs"** wird ein Portbefehl ausgeführt. (siehe: **let val = cpu::inb(1);**)


In der Datei **"\*\*/Abgabe/Aufgabe01/handin_vorgabe01src/startup.rs"** ist der hello_world_tread als User-Level-Thread gesetzt. (siehe: 'kernel_thread = false')

=> User-Thread versucht Port-Befehl auszuführen => General Protection Fault => Läuft in Ring 3



"""

kickoff_kernel_thread, tid=0, old_rsp0 = 481f30, is_kernel_thread: true

preempt: tid=0, old_rsp0=481f30 and switch to tid=1, old_rsp0=581fa8

kickoff_kernel_thread, tid=1, old_rsp0 = 581fa8, is_kernel_thread: false

general protection fault: error_code = 0x0, cs:rip = 0x23:0x10b869

"""


==> Läuft, wenn **let val = cpu::inb(1);** auskommentiert wird