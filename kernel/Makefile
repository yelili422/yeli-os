target := riscv64gc-unknown-none-elf
mode := debug
kernel := target/$(target)/$(mode)/yeli-os
bin := target/$(target)/$(mode)/kernel.bin

objdump := rust-objdump --arch-name=riscv64
objcopy := rust-objcopy --binary-architecture=riscv64

.PHONY: kernel build clean qemu run env

env:
	cargo install cargo-binutils
	rustup component add llvm-tools-preview rustfmt
	rustup target add $(target)

kernel:
	cargo build

$(bin): kernel
	$(objcopy) $(kernel) --strip-all -O binary $@

asm:
	$(objdump) -d $(kernel) | less

build: $(bin)

clean:
	cargo clean


# FIXME:
# The laster version can't load the kernel.
# See more at: https://github.com/riscv/opensbi/issues/118
# and https://github.com/rcore-os/rCore-Tutorial/issues/136
qemu:
	qemu-system-riscv64 \
		-machine virt \
		-nographic \
		-bios ../bootloader/opensbi-0.6-rv64-qemu.bin \
		-device loader,file=$(bin),addr=0x80200000 \

run: build qemu
