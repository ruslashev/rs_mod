LIN_CMDLINE = rootwait rw root=/dev/vda console=ttyS0

LIN_DIR = build/linux
LIN_CFG = $(LIN_DIR)/.config
LIN_IMG = $(LIN_DIR)/arch/x86/boot/bzImage
BB_DIR = build/busybox
BB_CFG = $(BB_DIR)/.config
BB_BIN = $(BB_DIR)/busybox
BB_INS = $(RFS_DIR)/bin/busybox
RFS_DIR = build/rootfs
RFS_IMG = build/rootfs.img
MOD_DIR = $(RFS_DIR)/lib/modules

MLIN = $(MAKE) -C linux O=../$(LIN_DIR) LLVM=1 CLIPPY=1
MBB = $(MAKE) -C busybox O=../$(BB_DIR)

default: qemu

all:
	$(MAKE) submodules
	$(MAKE) rust
	$(MAKE) build-linux
	$(MAKE) rootfs

submodules:
	git submodule update --init

rust:
	rustup override set $(shell linux/scripts/min-tool-version.sh rustc)
	rustup component add rust-src
	cargo install --locked --version $(shell linux/scripts/min-tool-version.sh bindgen) bindgen-cli
	$(MLIN) rustavailable

linux-config: $(LIN_CFG)

$(LIN_CFG): configs/linux_frag.config
	mkdir -p $(LIN_DIR)
	$(MLIN) x86_64_defconfig
	linux/scripts/kconfig/merge_config.sh -m -O $(LIN_DIR) $@ $^
	$(MLIN) olddefconfig

build-linux: $(LIN_IMG)

$(LIN_IMG): $(LIN_CFG)
	$(MLIN) -j $(shell nproc)
	touch $@

modules: $(MOD_DIR)

$(MOD_DIR): $(LIN_IMG) $(wildcard src/**/*)
	$(MLIN) M=../src
	$(MLIN) M=../src modules_install INSTALL_MOD_PATH=../rootfs
	touch $(MOD_DIR)

busybox-config: $(BB_CFG)

$(BB_CFG): configs/busybox.config
	mkdir -p $(BB_DIR)
	cp $^ $@

build-busybox: $(BB_BIN)

$(BB_BIN): $(BB_CFG)
	$(MBB) -j $(shell nproc)

install-busybox: $(BB_INS)

$(BB_INS): $(BB_BIN)
	mkdir -p $(RFS_DIR)
	$(MBB) CONFIG_PREFIX=../rootfs install

rootfs: $(RFS_IMG)

$(RFS_IMG): $(BB_INS) $(MOD_DIR) $(wildcard overlay/**/*)
	rm -f $@
	cp -rT --update=all --preserve=mode overlay $(RFS_DIR)
	mkdir -p $(RFS_DIR)/dev
	mkdir -p $(RFS_DIR)/proc
	mkdir -p $(RFS_DIR)/root
	mkdir -p $(RFS_DIR)/sys
	truncate --size=32M $@
	fakeroot /sbin/mkfs.ext4 -d $(RFS_DIR) $@ -E root_owner=0:0,no_copy_xattrs

qemu: $(LIN_IMG) $(RFS_IMG)
	qemu-system-x86_64 \
		-kernel $(LIN_IMG) \
		-drive file=$(RFS_IMG),if=virtio,format=raw \
		-append "$(LIN_CMDLINE)" \
		-M pc \
		-m 1G \
		-nographic \
		$(QEMU_EXTRA_FLAGS)

gdb_start: LIN_CMDLINE+=nokaslr
gdb_start: QEMU_EXTRA_FLAGS=-s -S
gdb_start: qemu

gdb_connect:
	cd $(LIN_DIR) && gdb vmlinux -ex 'target remote :1234'

rust-analyzer:
	$(MLIN) M=../src rust-analyzer
	mv build/src/rust-project.json .

rustdoc:
	$(MLIN) rustdoc
	xdg-open $(LIN_DIR)/Documentation/output/rust/rustdoc/kernel/index.html

clean:
	rm -rf build

clean-linux:
	rm -rf $(LIN_DIR)

clean-busybox:
	rm -rf $(BB_DIR)

clean-rootfs:
	rm -rf $(RFS_DIR) $(RFS_IMG)

.PHONY: \
	default \
	all \
	submodules \
	rust \
	linux-config \
	build-linux \
	modules \
	busybox-config \
	build-busybox \
	install-busybox \
	rootfs \
	qemu \
	gdb_start \
	gdb_connect \
	rust-analyzer \
	rustdoc \
	clean \
	clean-linux \
	clean-busybox \
	clean-rootfs

MAKEFLAGS += --no-print-directory
