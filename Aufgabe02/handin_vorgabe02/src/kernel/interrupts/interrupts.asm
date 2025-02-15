;╔═════════════════════════════════════════════════════════════════════════╗
;║ Module: intdispatcher                                                   ║
;╟─────────────────────────────────────────────────────────────────────────╢
;║ Descr.: Here is everything related to the low-level handling of x86     ║
;║         interrupts: IDT, PIC initialization, interrupt handlers, and    ║
;║         invoking interrupt dispatching in Rust; 'int_disp' function     ║
;║         in 'intdispatcher.rs'.                                          ║
;╟─────────────────────────────────────────────────────────────────────────╢
;║ Author: Michael Schoetter, Univ. Duesseldorf, 8.6.2024                  ║
;╚═════════════════════════════════════════════════════════════════════════╝
[GLOBAL _init_interrupts]     ; export init function
[GLOBAL _idt]                 ; export, needed in 'syscalls.asm'

[EXTERN int_disp]             ; Funktion in Rust, welche Interrupts behandelt
[EXTERN int_gpf]              ; Funktion in Rust, welche GPF behandelt

[SECTION .text]
[BITS 64]

; Init the IDT and PIC
; This function should be called early during OS startup
_init_interrupts:
   call _setup_idt
   call _reprogram_pics
   ret


; Interrupt handlers
%macro _wrapper 1
_wrapper_%1:
   ; save registers
	  push   rax
	  push   rbx
	  push   rcx
	  push   rdx
	  push   rdi
	  push   rsi
	  push   r8
	  push   r9
	  push   r10
	  push   r11
   push   r12
   push   r13
   push   r14
   push   r15

   ; do we have a general protection fault?
			%if %1 == 13 
	     mov    rdi, [rsp+112] ; error code
	     mov    rdx, [rsp+120] ; rip
	     mov    rsi, [rsp+128] ; cs
	    call    int_gpf
   %else
   	  ; pass the vector as parameter 
	     xor rax, rax
						mov al, %1
	     mov    rdi, rax
	     call   int_disp
			%endif

	  ; Restore registers
   pop    r15
   pop    r14
   pop    r13
   pop    r12
	  pop    r11
	  pop    r10
	  pop    r9
	  pop    r8
	  pop    rsi
	  pop    rdi
	  pop    rdx
	  pop    rcx
   pop    rbx
	  pop    rax

	  ; done!
  	iretq
%endmacro

; create 256 interrupt handlers, one for each entry in the IDT
%assign i 0
   %rep 256
   _wrapper i
   %assign i i+1
%endrep


;
; Setup IDT
;
_setup_idt:
	  mov    rax, _wrapper_0

	  ; Bits 0..15 -> ax, 16..31 -> bx, 32..64 -> edx
	  mov    rbx, rax
	  mov    rdx, rax
	  shr    rdx, 32
	  shr    rbx, 16

	  mov    r10, _idt  ; pointer to the interrupt gate
	  mov    rcx, 255   ; counter
_loop:
	  add    [r10+0], ax
	  adc    [r10+6], bx
	  adc    [r10+8], edx
	  add    r10, 16
	  dec    rcx
	  jge    _loop

	  lidt   [_idt_descr]
	  ret

;
; Reprogramming the Programmable Interrupt Controllers (PICs) 
; so that all 15 hardware interrupts lie sequentially in the IDT
;
_reprogram_pics:
   mov    al, 0x11   ; ICW1: 8086-Modus with ICW4
	  out    0x20, al
	  call   _delay
	  out    0xa0, al
	  call   _delay
	  mov    al, 0x20   ; ICW2 Master: IRQ # Offset (32)
	  out    0x21, al
	  call   _delay
	  mov    al, 0x28   ; ICW2 Slave: IRQ # Offset (40)
	  out    0xa1, al
	  call   _delay
	  mov    al, 0x04   ; ICW3 Master: slaves use IRQs
	  out    0x21, al
	  call   _delay
	  mov    al, 0x02   ; ICW3 Slave: connected through IRQ2 of master
	  out    0xa1, al
	  call   _delay
	  mov    al, 0x03   ; ICW4: 8086-Modus and automatic EOI
	  out    0x21, al
	  call   _delay
	  out    0xa1, al
	  call   _delay

	  mov    al, 0xff   ; Mask all hardware interrupts
	  out    0xa1, al   ; Except IRQ2 is allowed
	  call   _delay     
	  mov    al, 0xfb   ; used for cascading
	  out    0x21, al

	  ret

;
; Short delay, required for some in/out commands
;
_delay:
   jmp    _L2
_L2:
   ret


[SECTION .data]

;
; Interrupt Descriptor Table with 256 entries
;
_idt:
%macro _idt_entry 1
   dw  (_wrapper_%1 - _wrapper_0) & 0xffff ; offset 0 .. 15
   dw  0x0000 | 0x8 * 2 ; selector references the 64 bit code segment descriptor in the GDT, see 'boot.asm'
  	dw  0x8e00 ; 8 -> interrupt is present, e -> 80386 64 bit interrupt gate
   dw  ((_wrapper_%1 - _wrapper_0) & 0xffff0000) >> 16 ; offset 16 .. 31
   dd  ((_wrapper_%1 - _wrapper_0) & 0xffffffff00000000) >> 32 ; offset 32..63
   dd  0x00000000 ; reserved
%endmacro

%assign i 0
%rep 256
_idt_entry i
%assign i i+1
%endrep

; needed for LIDT instruction, see 'setup_idt'
_idt_descr:
   dw  256*16 - 1    ; 256 entries
   dq _idt
