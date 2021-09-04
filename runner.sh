#!/bin/bash

set -e

target=$1
project_dir=$CARGO_MANIFEST_DIR

suffix=".bin"
bin_file=$project_dir/$target$suffix

rust-objcopy $project_dir/$target \
    --strip-all \
    -O binary \
    $bin_file

qemu-system-riscv64 \
    -machine virt \
    -nographic \
    -bios default \
    -kernel $bin_file \
    -device loader,file=$bin_file,addr=0x80200000
 