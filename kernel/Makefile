KERNEL_NAME = yeli-os

MODE ?= debug
TARGET ?= riscv64gc-unknown-none-elf
BUILD_ARGS ?=

ifeq ($(MODE), release)
  BUILD_ARGS += --release
endif

KERNEL_ELF =
KERNEL_BIN = $(KERNEL_ELF).img

ifeq ($(firstword $(MAKECMDGOALS)), run)
  # use the rest as arguments for "run"
  RUN_ARGS = $(wordlist 2, $(words $(MAKECMDGOALS)), $(MAKECMDGOALS))
  KERNEL_ELF = $(firstword $(RUN_ARGS))

  # ...and turn them into do-nothing targets
  $(eval $(RUN_ARGS):;@:)
endif

ifeq ($(KERNEL_ELF),)
  BUILD_PATH = target/$(TARGET)/$(MODE)
  KERNEL_ELF = $(BUILD_PATH)/$(KERNEL_NAME)
endif

.PHONY: build
build: fmt $(KERNEL_BIN)

$(KERNEL_ELF):
	cargo build $(BUILD_ARGS)

$(KERNEL_BIN): $(KERNEL_ELF)
	@# Transform the output of ELF into binary format
	@# and discord metadata to ensure machine finds the first instruction
	rust-objcopy $(KERNEL_ELF) --strip-all -O binary $(KERNEL_BIN)

QEMU = qemu-system-riscv64
# In virt platform, the physical address starts at 0x8000_0000,
# the default memory size is 128MiB.
# Hance the bootloader will be loaded to 0x8000_0000, and the
# os bin file will be loaded to 0x8020_0000. We need to ensure
# that the first instruction of the kernel is located at
# physical address 0x8020_0000. We did it in linker.ld file.
QEMU_ARGS = \
	-machine virt \
	-nographic \
	-bios default \
	-kernel $(KERNEL_BIN) \
	-device loader,file=$(KERNEL_BIN),addr=0x80200000

.PHONY: run
run: $(KERNEL_BIN)
	$(QEMU) $(QEMU_ARGS)

.PHONY: clean
clean:
	@cargo clean

.PHONY: fmt
fmt:
	@cargo fmt

.PHONY: test
test: $(KERNEL_BIN)
	@cargo test --lib

GDB = riscv64-elf-gdb
DGB_CLIENT_ARGS = \
	-ex 'file $(KERNEL_ELF)' \
	-ex 'set arch riscv:rv64' \
	-ex 'target remote localhost:1234'

.PHONY: gdb
gdb: $(KERNEL_BIN)
	$(QEMU) $(QEMU_ARGS) -s -S

.PHONY: gdb_client
gdb_client:
	$(GDB) $(DGB_CLIENT_ARGS)