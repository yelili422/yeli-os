# See more informations of risc-v assembly at: 
# https://github.com/riscv-non-isa/riscv-asm-manual/blob/master/riscv-asm.md

    .section .text.entry
    .globl _entry
_entry:
    # The layout of satp :
    # Mode * 4, Address Space ID * 16, PPN of Root Page * 44
    # Get physical page number of boot_page_table.
    lui t0, %hi(boot_page_table)
    li  t1, 0xffffffff00000000
    sub t0, t0, t1
    srli    t0, t0, 12
    li  t1, (8 <<60)    # 8 << 60 represents Sv39 mode
    or  t0, t0, t1
    # Write to satp and update TLB.
    csrw    satp, t0
    sfence.vma

    # Load the virtual address of boot stack.
    lui sp, %hi(boot_stack_top)
    addi    sp, sp, %lo(boot_stack_top)

    # Jump to the rust code.
    lui t0, %hi(_start)
    addi    t0, t0, %lo(_start)
    jr  t0

    .section .bss.stack
    .global boot_stack
boot_stack:
    .space  4096 * 4    # 16K
    .global boot_stack_top
boot_stack_top:

    .section .data
    .align  12
    .global boot_page_table
boot_page_table:
    # In the Sv39 mode, a virtual address represented by 64 bits:
    # Reserved * 10, PPN[2] * 26, PPN[1] * 9, PPN[2] * 9, RSW * 2, Flags * 8
    .8byte  0
    .8byte  0
    # 0xcf = 0x1100_1111
    .8byte  (0x80000 << 10) | 0xcf  # 0x8000_0000 -> 0x8000_0000
    .zero   507 * 8
    .8byte  (0x80000 << 10) | 0xcf  # 0xffff_ffff_8000_0000 -> 0x8000_0000
    .8byte  0
