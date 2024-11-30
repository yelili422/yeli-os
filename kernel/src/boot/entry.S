# See more information of risc-v assembly at:
# https://github.com/riscv-non-isa/riscv-asm-manual/blob/master/riscv-asm.md

.section .text.entry
.globl _entry
_entry:
    la  sp, boot_stack_top
    call    _start
spin:
    j   spin

.section .bss.stack
.global boot_stack
boot_stack:
.space  1024 * 1024 * 16   # 16M
.global boot_stack_top
boot_stack_top:
