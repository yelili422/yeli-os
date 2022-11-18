
MODE ?= debug
ARCH := riscv64gc-unknown-none-elf

build_path := target/$(ARCH)/$(MODE)
kernel_name := yeli-os

ELF ?= $(build_path)/$(kernel_name)
IMAGE := $(ELF).img

build_args :=

qemu := qemu-system-riscv64
qemu_args := \
	-machine virt \
	-nographic \
	-bios default \
	-kernel $(IMAGE) \
	-device loader,file=$(IMAGE),addr=0x80200000

gdb := riscv64-unknown-elf-gdb
gdb_client_args := \
	-ex 'file $(ELF)' \
	-ex 'set arch riscv:rv64' \
	-ex 'target remote localhost:1234'

ifeq ($(MODE), release)
build_args += --release
endif

.PHONY: build
build:
	cargo build --$(build_args)

.PHONY: run
run: build
	@# Transform the output of ELF into binary format
	@# and discord metadata to ensure machine finds the first instruction
	rust-objcopy $(ELF) --strip-all -O binary $(IMAGE)

	$(qemu) $(qemu_args)

.PHONY: gdb
gdb: build
	$(qemu) $(qemu_args) -s -S

.PHONY: gdb_client
gdb_client:
	$(gdb) $(gdb_client_args)

.PHONY: clean
clean:
	@cargo clean

.PHONY: test
test: build
	@cargo test
