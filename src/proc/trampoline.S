# Code to switch between user and kernel space.
#
# This code is mapped at the same virtual address
# (TRAMPOLINE) ins user and kernel space so that
# it containes to work when it switches page tables.
#
# linker.ld causes this to be aligned to a page
# boundary.

.global trampoline
trampoline:
