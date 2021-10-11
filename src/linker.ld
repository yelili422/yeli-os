/* Ref: https://sourceware.org/binutils/docs/ld/Scripts.html */

OUTPUT_ARCH(riscv)

ENTRY(_entry)

BASE_ADDRESS = 0xffffffff80200000;

SECTIONS
{
    /* . is Location Counter */
    . = BASE_ADDRESS;

    kernel_start = .;

    . = ALIGN(4K);
    text_start = .;

    .text : {
        *(.text.entry)
        *(.text .text.*)
    }

    . = ALIGN(4K);
    rodata_start = .;

    .rodata : {
        *(.rodata .rodata.*)
    }

    . = ALIGN(4K);
    data_start = .;

    .data : {
        *(.data .data.*)
    }

    . = ALIGN(4K);
    bss_start = .;

    .bss : {
        *(.sbss .bss .bss.*)
    }

    . = ALIGN(4K);
    kernel_end = .;
}