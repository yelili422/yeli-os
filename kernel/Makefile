KERNEL_NAME = yeli-os

MODE ?= debug
TARGET ?= riscv64gc-unknown-none-elf
BUILD_ARGS ?=

ifeq ($(MODE), release)
  BUILD_ARGS += --release
endif

KERNEL_ELF =
KERNEL_BIN = $(KERNEL_ELF).img

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

.PHONY: clean
clean:
	@cargo clean

.PHONY: fmt
fmt:
	@cargo fmt

.PHONY: test
test: $(KERNEL_BIN)
	@cargo test --lib
