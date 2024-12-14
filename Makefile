
MODE ?= release
TARGET ?= riscv64gc-unknown-none-elf
KERNEL_NAME ?= yeli-os
ROOTFS_NAME ?= rootfs.img

SUBDIRS := kernel user fs
TARGET_DIR := target

QEMU = qemu-system-riscv64
GDB = riscv64-elf-gdb
KERNEL_ELF ?= $(TARGET_DIR)/$(KERNEL_NAME)
KERNEL_IMG ?= $(TARGET_DIR)/$(KERNEL_NAME).img
ROOTFS ?= $(TARGET_DIR)/$(ROOTFS_NAME)

.PHONY: all clean qemu gdb

all: $(KERNEL_IMG) $(TARGET_DIR)/bin/% $(ROOTFS)

$(TARGET_DIR):
	@mkdir -p $(TARGET_DIR)

$(KERNEL_IMG): $(TARGET_DIR)
	$(MAKE) -C kernel MODE=$(MODE) TARGET=$(TARGET) KERNEL_NAME=$(KERNEL_NAME)
	@cp kernel/$(TARGET_DIR)/$(TARGET)/$(MODE)/$(KERNEL_NAME) $(KERNEL_ELF)
	@cp kernel/$(TARGET_DIR)/$(TARGET)/$(MODE)/$(KERNEL_NAME).img $(KERNEL_IMG)

$(TARGET_DIR)/bin/%: $(TARGET_DIR)
	$(MAKE) -C user install MODE=$(MODE) TARGET=$(TARGET) INSTALL_DIR=../$(TARGET_DIR)/bin

$(ROOTFS): $(KERNEL_IMG) $(TARGET_DIR)/bin/%
	$(MAKE) -C fs mkfs MODE=$(MODE) TARGET=$(TARGET) IMG=$(ROOTFS) BINS=../$(TARGET_DIR)/bin
	@cp fs/$(ROOTFS) $(TARGET_DIR)/


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
	-kernel $(KERNEL_IMG) \
	-device loader,file=$(KERNEL_IMG),addr=0x80200000 \
	-drive file=$(ROOTFS),format=raw,if=none,id=x0 \
	-global virtio-mmio.force-legacy=false \
	-device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0

qemu: all
	$(QEMU) $(QEMU_ARGS)

gdb:
	$(MAKE) MODE=debug all
	$(QEMU) $(QEMU_ARGS) -s -S

gdb_client:
	$(GDB) \
	-ex 'file $(KERNEL_ELF)' \
	-ex 'set arch riscv:rv64' \
	-ex 'target remote localhost:1234'

clean:
	@for dir in $(SUBDIRS); do \
		$(MAKE) -C $$dir clean; \
	done
	@rm -rf $(TARGET_DIR)
