    .section .text.entry
    .globl _entry
_entry:
    la sp, boot_stack_top
    call _start

    .section .bss.stack
    .global bootstack
boot_stack:
    .space 4096 * 4
    .global boot_stack_top
boot_stack_top:
