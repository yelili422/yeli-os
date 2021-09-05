#!/bin/bash

set -e

# Guarantee the target path absolute.
target=$1
if [[ "$target" != /* ]]; then
    target=$CARGO_MANIFEST_DIR/$target
fi

suffix=".bin"
bin_file=$target$suffix

# Transform the output of ELF into binary format.
rust-objcopy $target \
    --strip-all \
    -O binary \
    $bin_file

# Run the binary file in qemu.
qemu-system-riscv64 \
    -machine virt \
    -nographic \
    -bios default \
    -kernel $bin_file \
    -device loader,file=$bin_file,addr=0x80200000
 