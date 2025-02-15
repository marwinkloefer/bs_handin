;*****************************************************************************
;*                                                                           *
;*                  t h r e a d . a s m                                      *
;*                                                                           *
;*---------------------------------------------------------------------------*
;* Beschreibung:    Assemblerfunktionen zum Starten eines Threads und zum    *
;*                  Umschalten zwischen Threads.                             *
;*                                                                           *
;* Autor:           Michael, Schoettner, HHU, 17.10.2023                     *
;*****************************************************************************


; EXPORTIERTE FUNKTIONEN
[GLOBAL _thread_kernel_start]
[GLOBAL _thread_user_start]
[GLOBAL _thread_switch]
[GLOBAL _thread_set_segment_register]


; IMPORTIERTE FUNKTIONEN

; Kernel-Stack im TSS setzen (beim Thread-Wechsel)
[EXTERN _tss_set_rsp0]


; IMPLEMENTIERUNG DER FUNKTIONEN

[SECTION .text]
[BITS 64]



;
; fn _thread_kernel_start (old_rsp0: u64); 
;
; Startet einen Thread im Ring 0
;
_thread_kernel_start:
    mov rsp, rdi                ; 1. Parameter -> load 'old_rsp0'
    pop rbp
    pop rdi                     ; Hier ist 'old_rsp0'
    pop rsi
    pop rdx
    pop rcx
    pop rbx
    pop rax
    pop r15
    pop r14
    pop r13
    pop r12
    pop r11
    pop r10
    pop r9
    pop r8
    popf                        
    retq


;
; fn _thread_switch (now_rsp0: *mut u64, then_rsp0: u64, then_rsp0_end: u64);
;    
; Umschalten zw. Threads
;
;       now_rsp0:      Dies ist ein Zeiger auf 'old_rsp0' in der Thread-Struct, 
;                      des Threads dem die CPU entzogen wird. Hier speichern wir RSP
;       then_rsp0:     Dies ist ein Zeiger auf 'old_rsp0' in der Thread-Struct, 
;                      des Threads der die CPU nun bekommt. Benötigen wir um den 0
;                      Stack umzuschalten
;       then_rsp0_end: Erste benutzerbare Adresse des Kernel-Stacks von 'then_rsp0'
;                      Wird benoetigt, um den RSP0-Eintrag im TSS zu aktualisieren          
_thread_switch:

    ; Register des aktuellen Threads auf dem Stack sichern
    pushf
    push r8
    push r9
    push r10
    push r11
    push r12
    push r13
    push r14
    push r15
    push rax
    push rbx
    push rcx
    push rdx
    push rsi
    push rdi
    push rbp
 
    ; sichere Stackpointer in 'now_rsp0' (1. Param)
    mov [rdi], rsp     

    ; aktualisiere RSP0 (Kernel-Stack) im TSS (3. Param, 'then_rsp0_end')
    mov rdi, rdx
    call _tss_set_rsp0

    ; Register des naechsten Threads laden

    ; Stack umschalten mithilfe von 'then_rsp0' (2. Param.)
    mov rsp, rsi
    pop rbp
    pop rdi
    pop rsi
    pop rdx
    pop rcx
    pop rbx
    pop rax
    pop r15
    pop r14
    pop r13
    pop r12
    pop r11
    pop r10
    pop r9
    pop r8
    popf 

	retq               ; Thread-Wechsel !



;
; fn _thread_user_start (old_rsp0: u64); 
;
; Schaltet den rufenden Thread in den Ring 3
; Wird nur 1x in 'kickoff_kernel_thread' in 'thread.rs' gerufen
_thread_user_start:
    mov rsp, rdi                ; 1. Parameter -> load 'old_rsp'
    pop rdi                     ; Hier ist 'object: *mut Thread'
    iretq                       ; Thread-Wechsel und Umschalten in den User-Mode!


;
; fn _thread_set_segment_register(); 
;
; Wird nach dem Starten eines Threads in Ring 3 benoetigt.
; Wir muessen noch das DS register setzen
_thread_set_segment_register:
   xor rax, rax
   mov rax, 43 ; User Data Segment; 5. Eintrag, RPL = 3
   mov ds, ax  
   mov es, ax 
   mov fs, ax 
   mov gs, ax 
   retq