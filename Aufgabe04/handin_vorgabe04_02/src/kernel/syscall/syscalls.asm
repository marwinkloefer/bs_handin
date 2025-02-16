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
; Muss mit NO_SYSCALLS in 'kernel/syscall/usr_api.rs' konsistent sein!
NO_SYSCALLS: equ 5

; Vektor fuer Systemaufrufe
SYSCALL_TRAPGATE: equ 0x80



;
; Trap-Gate fuer Systemaufrufe einrichten
;
_init_syscalls:
	;-----------------------------------------------------------------------------------------------------------------------------------------
	;Baltt2, 1. Aufgabe: Interrupt Descriptor Table (IDT) 
	;-----------------------------------------------------------------------------------------------------------------------------------------
	;    127-124  123-120  119-116  115-112|  111-108  107-104  103-100  99-96
	;    rrrr     rrrr     rrrr     rrrr   |  rrrr     rrrr     rrrr     rrrr
	;    /-------------0x0000-------------\|/-------------0x0000-------------\
	;
	;    95-92    91-88    87-84    83-80  |  79-76    75-72    71-68    67-64
	;    oooo     oooo     oooo     oooo   |  oooo     oooo     oooo     oooo
	;    /-------------0x0000-------------\|/-------------0x0000-------------\
	;    									  
	;    63-60    59-56    55-52    51-48  |  47-44    43-40    39-36    35-32
	;    oooo     oooo     oooo     oooo   |  pxx0     TYPE     0000     0IST		=> xx = DPL | p = Segment Present flag | IST = Interrupt Stack Table
	;	 oooo     oooo     oooo     oooo   |  1110     1111     0000     0IST		=> DPL = 11 | Present = 1| TYPE = 1111 (Table 3-2)
	;	 /-------------0x0000-------------\|/-------------0x0000-------------\
	;    
	;    31-28    27-24    23-20    19-16  |  15-12    11-8     7-4      3-0
	;    ssss     ssss     ssss     ssss   |  oooo     oooo     oooo     oooo		=> 0-1 = requested DPL (=0) | 2 = GDT(=0) / LDT(=1) | 3-15 = Index (=2)
	;    0000     0000     0001     0000   |  oooo     oooo     oooo     oooo
	;	 /-------------0x0000-------------\|/-------------0x0000-------------\

	; Schreibe in die idt den neuen trap-gate descriptor
	; idt an adresse _idt => ab offset 0x80 * 16

	; Lade die Addresse des _syscall_handler's in das Register rax
	mov rax, _syscall_handler 
	mov rcx, _idt + 128 * 16 ;// adresse des 8ten IDT-Eintrag
	; Setze die unteren 2 Byte (16 Bits) der _syscall_handler Address in die Bits 15-0, also das 0te und 1te Byte des 80ten IDT-Eintrag
	mov word [rcx], ax 				; Offset = _syscall_handler										[15-0]
	; Damit Bits richtig stehen und "genutzen" nicht erneut genutz werden, schiebe die _syscall_handler Address um 16 Bits nach rechts
	shr rax, 16 

	; Schreibe die Adresse des Kernel-Code-Segment (3ter Eintrage also Offset 2) an die Bits 31-16, 
	; also in das 2te und 3te Byte des 80ten IDT-Eintrag => 0000000000010000 = 0x00 0x10
	mov word [rcx + 2], 0x10		; Segment Selector												[31-16]
	
	; Schreibe Interrupt Stack Table, TYPE, DPL und Presnet Flag an die Bits 47-32, 
	; also in das 4te und 5te Byte des 80ten IDT-Eintrag => 11101111 00000000 = 0xEF 0x00
	mov byte [rcx + 4], 0x00		; IST (see 6.14.5 Interrupt Stack Table => gerade alles auf 0) 	[39-32]
	mov byte [rcx + 5], 0xEF		; TYPE = 1111, DPL = 11 und Presnet Flag = 1					[47-40]

	; Setze die nächsten 2 Byte (16 Bits) der _syscall_handler Address in die Bits 63-48, also das 6te und 7te Byte des 80ten IDT-Eintrag
	mov word [rcx + 6], ax			; Offset = _syscall_handler										[63-48]
	; Damit Bits richtig stehen und "genutzen" nicht erneut genutz werden, schiebe die _syscall_handler Address um 16 Bits nach rechts
	shr rax, 16  

	; Setze die nächste 4 Byte (32 Bits) der _syscall_handler Address in die Bits 95-64, also das 8te, 9te, 10te und 11te Byte des 80ten IDT-Eintrag
	mov dword [rcx + 8], eax 		; Offset = _syscall_handler										[95-64]

    ; Setze die letzen 4 Byte (32 Bits) des Base 80ten IDT-Eintrag auf 0, da reserved 
	mov dword [rcx + 12], 0  		; Reserved = 0													[127-96]

	ret



;  +---------------------------------------------+
;  | RAX ist 64-Bit-Register.                    |
;  | EAX ist untere 32-Bit-Register des RAX.     |
;  | AX  ist unterstes 16 Bits-Registersdes RAX. |
;  | AL sind unteren 8 Bits des AX-Registers.    |
;  | AH sind oberen 8 Bits des AX-Registers.     |
;  +---------------------------------------------+
;  | byte  sind  8 Bit oder 1 Byte				 |
;  | word  sind 16 Bit oder 2 Byte				 |
;  | dword sind 32 Bit oder 4 Byte				 |
;  +---------------------------------------------+
;
; Handler fuer Systemaufrufe 
;
_syscall_handler:
	; Only save registers marked as nessesary in 'Figure 3.4: Register Usage' in x86_64-abi-0.99

	; save registers
	push   	rbx
	push   	rbp
	push   	r12
	push   	r13
	push   	r14
	push   	r15

  	; DS und ES auf dem Stack sichern und danach Kernel-Data Segment in DS und ES setzen

	;xor 	rcx, rcx	;;// clear register => 0x0000
	mov 	rcx, 0
	mov 	cx, DS		;;// DS in ax speichern
	shl 	rcx, 16		;;// shiften um untere 16 bit von rax "frei" zu machen um ax zu schreiben
	mov 	cx, ES		;;// => AX hält ES, EAX hält DS + ES (untere 32 bit von RAX und obere 32 Bit = 0) ; bis hier keinen fehler
	push	rcx 		;;// lege DS und ES auf Stack

	;;// 0-1 = requested DPL (=0) | 2 = GDT(=0) / LDT(=1) | 3-15 = Index in GDT (=3)
	;;//0000000000011 0 00

	mov cx, 0x0018      			; Lädt den Wert 0x0018 (dezimal 24 | binär 0000000000011000) in die unteren 16 Bit des rcx-Registers
	;mov cx, 0000000000011000b      ; Problem mit binary value => b am ende vergessen ursprüglich, jetzt würde es auch so funktionieren => ohne b general protection fault: error_code = 0x2af8, cs:rip = 0x10:0x1001fa
	mov ds, cx
	mov es, cx 


	; Pruefen, ob die Funktionsnummer nicht zu gross ist
	cmp rax, NO_SYSCALLS
	jge syscall_abort   ; wirft eine Panic, kehrt nicht zurueck

	; Funktionsnummer ist OK -> Rust aufrufen
	call syscall_disp

 	; DS und ES wiederherstellen
	pop 	rcx			;;// DS + ES liegen auf Stack 
	mov 	ES, cx		;;// ES als letztes drauf => erstes runter
	shr 	rcx, 16		;;// shiften um an obere 16 bit von rax zu kommen durch ax
	mov 	DS, cx		;;// DS wiederherstellen

  	; Alle Register, marked as nessesary in 'Figure 3.4: Register Usage' in x86_64-abi-0.99, wiederherstellen
	pop    r15
	pop    r14
	pop    r13
	pop    r12
	pop    rbp
   	pop    rbx


	; done!
  	iretq
