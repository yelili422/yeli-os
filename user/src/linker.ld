OUTPUT_ARCH(riscv)
ENTRY(_start)

BASE_ADDRESS = 0x0;

SECTIONS
{
    . = BASE_ADDRESS;
    .text : {
        *(.text.entry)
        *(.text .text.*)
    }
    .rodata : {
        . = ALIGN(16);
        *(.rodata .rodata.*)
        . = ALIGN(16);
        *(.srodata .srodata.*)
    }
    . = ALIGN(4096);
    .data : {
        . = ALIGN(16);
        *(.data .data.*)
        . = ALIGN(16);
        *(.sdata .sdata.*)
    }
    .bss : {
        . = ALIGN(16);
        *(.bss .bss.*)
        . = ALIGN(16);
        *(.sbss .sbss.*)
    }
    . = ALIGN(4096);
    PROVIDE(end = .);
    /DISCARD/ : {
        *(.eh_frame)
        *(.debug*)
    }
}
