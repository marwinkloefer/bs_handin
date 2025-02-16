# Aufgabe 3

Richten Page-Frame-Allokator in der **list.rs** Datei ein und nutze diesen um in **frames.rs** die pageframes zu erstellen. Aufruf der init-Funktion erfolt in **kmain** mittels **frames::pf_init(&mut phys_mem);**. Hier wird dann der von multiboot erhaltene Speicher genutzt und aligned. abschließend nutzen wir die Frames für einen neuen heap: **let kernel_heap = frames::pf_alloc(KERNEL_HEAP_SIZE.div_ceil(PAGE_FRAME_SIZE), true);**, wobei wir neuen Kernel-Space-Frames allozieren.



Tests: **ohne User-Mode Threads und ohne Interrupts getestet werden**.

=> kein direkter Neustart ("nach dem Setzen des CR3-Registers sofort zu einem Neustart")