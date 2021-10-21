# See more information of risc-v assembly at: 
# https://github.com/riscv-non-isa/riscv-asm-manual/blob/master/riscv-asm.md

    .section .text.entry
    .globl _entry
_entry:
    la  sp, boot_stack_top
    call    _start

    .section .bss.stack
    .global boot_stack
boot_stack:
    .space  4096 * 4    # 16K
    .global boot_stack_top
boot_stack_top:
