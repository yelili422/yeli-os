
MODE ?= release
TARGET ?= riscv64gc-unknown-none-elf
KERNEL_NAME ?= yeli-os
ROOTFS_NAME ?= rootfs.img

SUBDIRS := kernel user fs
TARGET_DIR := target

QEMU = qemu-system-riscv64
KERNNEL_IMG ?= $(TARGET_DIR)/$(KERNEL_NAME).img
ROOTFS ?= $(TARGET_DIR)/$(ROOTFS_NAME)

.PHONY: all clean qemu

all: $(KERNNEL_IMG) $(TARGET_DIR)/bin/% $(ROOTFS)

$(TARGET_DIR):
	@mkdir -p $(TARGET_DIR)

$(KERNNEL_IMG): $(TARGET_DIR)
	$(MAKE) -C kernel MODE=$(MODE) TARGET=$(TARGET) KERNEL_NAME=$(KERNEL_NAME)
	@cp kernel/$(TARGET_DIR)/$(TARGET)/$(MODE)/$(KERNEL_NAME).img $(KERNNEL_IMG)

$(TARGET_DIR)/bin/%: $(TARGET_DIR)
	$(MAKE) -C user install MODE=$(MODE) TARGET=$(TARGET) INSTALL_DIR=../$(TARGET_DIR)/bin

$(ROOTFS): $(KERNNEL_IMG) $(TARGET_DIR)/bin/%
	$(MAKE) -C fs mkfs MODE=$(MODE) TARGET=$(TARGET) IMG=$(ROOTFS) BINS=../$(TARGET_DIR)/bin
	@cp fs/$(ROOTFS) $(TARGET_DIR)/

qemu: all
	$(QEMU) \
		-machine virt \
		-nographic \
		-bios default \
		-kernel $(KERNNEL_IMG) \
		-device loader,file=$(KERNNEL_IMG),addr=0x80200000 \
		-drive file=$(ROOTFS),format=raw,if=none,id=mmio0 \
		-device virtio-blk-device,drive=mmio0,bus=virtio-mmio-bus.0

clean:
	@for dir in $(SUBDIRS); do \
		$(MAKE) -C $$dir clean; \
	done
	@rm -rf $(TARGET_DIR)
