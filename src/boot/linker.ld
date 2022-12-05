/* Ref: https://sourceware.org/binutils/docs/ld/Scripts.html */

OUTPUT_ARCH(riscv)
ENTRY(_entry)

BASE_ADDRESS = 0x80200000;

SECTIONS
{
    . = BASE_ADDRESS;

    .text : {
        *(.text.entry)

        . = ALIGN(4096);
        *(.text.trampoline)

        . = ALIGN(4096);
        *(.text .text.*)

        . = ALIGN(4096);
        PROVIDE(etext = .);
    }

    .rodata : {
        . = ALIGN(4096);
        *(.rodata.*)
    }

    .data : {
        . = ALIGN(4096);
        *(.data*)
    }

    .bss : {
        . = ALIGN(4096);
        *(.sbss .bss .bss.*)
    }

    . = ALIGN(4096);
    PROVIDE(end = .);
}