TARGET ?= riscv64gc-unknown-none-elf
APP_DIR ?= src/bin

MODE ?= release

TARGET_DIR ?= target/$(TARGET)/$(MODE)
INSTALL_DIR ?=

APPS := $(wildcard $(APP_DIR)/*.rs)
ELFS := $(patsubst $(APP_DIR)/%.rs, $(TARGET_DIR)/%, $(APPS))
BINS := $(patsubst $(APP_DIR)/%.rs, $(TARGET_DIR)/%.bin, $(APPS))

OBJDUMP := rust-objdump --arch-name=riscv64
OBJCOPY := rust-objcopy --binary-architecture=riscv64

.PHONY: build
build: binary

.PHONY: elf
elf: $(APPS)
	@cargo build --release

.PHONY: binary
binary: elf
	$(foreach elf, $(ELFS), $(OBJCOPY) $(elf) --strip-all -O binary $(patsubst $(TARGET_DIR)/%, $(TARGET_DIR)/%.bin, $(elf));)

.PHONY: install
install: binary $(INSTALL_DIR)
	$(foreach bin, $(BINS), cp $(bin) $(INSTALL_DIR)/$(basename $(notdir $(bin)));)

$(INSTALL_DIR):
	@mkdir -p $(INSTALL_DIR)

.PHONY: clean
clean:
	@cargo clean
