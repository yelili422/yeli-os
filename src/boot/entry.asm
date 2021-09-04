    .section .text.entry
    .globl _start
_start:
    la sp, boot_stack_top
    call rust_main

    .section .bss.stack
    .global bootstack
boot_stack:
    .space 4096 * 4
    .global boot_stack_top
boot_stack_top:
