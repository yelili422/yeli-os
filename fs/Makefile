MODE ?= release
TARGET ?= riscv64gc-unknown-none-elf
BUILD_ARGS ?=
IMG ?= rootfs.img
BINS ?= bin

ifeq ($(MODE), release)
  BUILD_ARGS += --release
endif

.PHONY: build
build:
	cargo build $(BUILD_ARGS) --all-targets

.PHONY: test
test:
	cargo test --all-targets

mkfs:
	cargo run $(BUILD_ARGS) --bin mkfs -- $(IMG) $(BINS)

.PHONY: clean
clean:
	cargo clean
