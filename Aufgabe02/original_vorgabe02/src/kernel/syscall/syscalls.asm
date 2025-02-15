;******************************************************************************
;*                                                                            *
;*                  s y s c a l l s . a s m                                   *
;*                                                                            *
;*----------------------------------------------------------------------------*
;* Beschreibung:    Hier befindet sich alles rund um die low-level Behandlung *
;*                  von Systemaufrufen sowie die Weiterleitung an Rust.       *
;*                                                                            *
;*                  Achtung: '_init_syscalls' muss nach der Initialisieriung  *
;*                  der IDT aufgerufen werden!                                *
;*                                                                            *
;* Autor:           Michael Schoettner, 23.8.2023                             *
;******************************************************************************

[GLOBAL _init_syscalls]       ; Funktion exportieren

[EXTERN _idt]                 ; IDT in 'interrupts.asm' 
[EXTERN syscall_disp]         ; Funktion in Rust, die Syscalls behandelt
[EXTERN syscall_abort]        ; Funktion in Rust, die abbricht, 
                              ; falls der Systemaufruf nicht existiert

[SECTION .text]
[BITS 64]

; Hoechste Funktionsnummer für den System-Aufruf-Dispatcher
; Muss mit NO_SYSCALLS in 'kernel/syscall/mod.rs' konsistent sein!
NO_SYSCALLS: equ 1

; Vektor fuer Systemaufrufe
SYSCALL_TRAPGATE: equ 0x80



;
; Trap-Gate fuer Systemaufrufe einrichten
;
_init_syscalls:
	
	; 
	; Hier muss Code eingefuegt werden
	;
	
	


;
; Handler fuer Systemaufrufe 
;
_syscall_handler:
  ; Alle Register sichern

	; 
	; Hier muss Code eingefuegt werden
	;



  ; DS und ES auf dem Stack sichern und danach Kernel-Data Segment in DS und ES setzen

	; 
	; Hier muss Code eingefuegt werden
	;


  ; Pruefen, ob die Funktionsnummer nicht zu gross ist
  cmp rax, NO_SYSCALLS
  jge syscall_abort   ; wirft eine Panic, kehrt nicht zurueck

  ; Funktionsnummer ist OK -> Rust aufrufen
  call syscall_disp

  ; DS und ES wiederherstellen
  
	; 
	; Hier muss Code eingefuegt werden
	;
  
  

  ; Alle Register wiederherstellen

	; 
	; Hier muss Code eingefuegt werden
	;

  iretq
