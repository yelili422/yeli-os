/* Ref: https://sourceware.org/binutils/docs/ld/Scripts.html */

OUTPUT_ARCH(riscv)
ENTRY(_entry)

BASE_ADDRESS = 0x80200000;

SECTIONS
{
    . = BASE_ADDRESS;

    .text : {
        *(.text.entry)

        . = ALIGN(0x1000);
        *(.text .text.*)

        . = ALIGN(0x1000);
        _trampoline = .;
        *(trampoline)

        . = ALIGN(0x1000);
        ASSERT( . - _trampoline <= 0x1000, "trampoline larger then one page.")
        PROVIDE(etext = .);
    }

    .rodata : {
        . = ALIGN(0x1000);
        *(.rodata.*)
    }

    .data : {
        . = ALIGN(0x1000);
        *(.data*)
    }

    .bss : {
        . = ALIGN(0x1000);
        *(.sbss .bss .bss.*)
    }

    . = ALIGN(0x1000);
    PROVIDE(end = .);
}