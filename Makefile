
MODE ?= debug
TARGET := riscv64gc-unknown-none-elf

BUILD_PATH := target/$(TARGET)/$(MODE)
KERNEL_NAME := yeli-os

KERNEL_ELF ?= $(BUILD_PATH)/$(KERNEL_NAME)
KERNEL_BIN := $(KERNEL_ELF).img

BUILD_ARGS :=

QEMU := qemu-system-riscv64
QEMU_ARGS := \
	-machine virt \
	-nographic \
	-bios default \
	-kernel $(KERNEL_BIN) \
	-device loader,file=$(KERNEL_BIN),addr=0x80200000

GDB := riscv64-unknown-elf-gdb
DGB_CLIENT_ARGS := \
	-ex 'file $(KERNEL_ELF)' \
	-ex 'set arch riscv:rv64' \
	-ex 'target remote localhost:1234'

ifeq ($(MODE), release)
BUILD_ARGS += --release
endif

.PHONY: build
build:
	cargo build --$(BUILD_ARGS)

.PHONY: run
run: build
	@# Transform the output of ELF into binary format
	@# and discord metadata to ensure machine finds the first instruction
	rust-objcopy $(KERNEL_ELF) --strip-all -O binary $(KERNEL_BIN)

	$(QEMU) $(QEMU_ARGS)

.PHONY: gdb
gdb: build
	$(qemu) $(QEMU_ARGS) -s -S

.PHONY: gdb_client
gdb_client:
	$(GDB) $(DGB_CLIENT_ARGS)

.PHONY: clean
clean:
	@cargo clean

.PHONY: test
test: build
	@cargo test
