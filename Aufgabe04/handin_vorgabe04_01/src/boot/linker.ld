ENTRY(_start)

SECTIONS {
    . = 1M;   /* load at address 1MB */

    ___KERNEL_DATA_START__ = .;

  .boot :
    {
        /* ensure that the multiboot header is at the beginning */
        *(_multiboot_header)
    }

    .text :
    {
        *(.text*)
    }

   .bss : 
    {
      ___BSS_START__ = .;
      *(".bss")
      *(".bss.*")
      ___BSS_END__ = .;
    }

    ___KERNEL_DATA_END__ = .;

}
