/* Ref: https://sourceware.org/binutils/docs/ld/Scripts.html */

OUTPUT_ARCH("riscv")
ENTRY(_entry)

BASE_ADDRESS = 0x80200000;

SECTIONS
{
    . = BASE_ADDRESS;
    __kernel_start = .;

    .text : {
        . = ALIGN(4096);
        __text_start = .;
        *(.text.entry)

        . = ALIGN(4096);
        __trampoline = .;
        *(.text.trampoline)

        . = ALIGN(4096);
        *(.text .text.*)

        . = ALIGN(4096);
        __text_end = .;
    }

    .rodata : {
        . = ALIGN(4096);
        __rodata_start = .;
        *(.rodata.*)

        . = ALIGN(4096);
        __rodata_end = .;
    }

    .data : {
        . = ALIGN(4096);
        __data_start = .;
        *(.data*)

        . = ALIGN(4096);
        __data_end = .;
    }

    .bss : {
        . = ALIGN(4096);
        __bss_start = .;
        *(.sbss .bss .bss.*)

        . = ALIGN(4096);
        __bss_end = .;
    }

    __kernel_end = .;
}