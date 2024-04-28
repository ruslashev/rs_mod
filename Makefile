LIN_DIR = build/linux
BB_DIR = build/busybox

MLIN = $(MAKE) -C linux O=../$(LIN_DIR) LLVM=1
MBB = $(MAKE) -C busybox O=../$(BB_DIR)

default:
	$(MAKE) submodules
	$(MAKE) rust
	$(MAKE) all

submodules:
	git submodule update --init

rust:
	rustup override set $(shell linux/scripts/min-tool-version.sh rustc)
	rustup component add rust-src
	cargo install --locked --version $(shell linux/scripts/min-tool-version.sh bindgen) bindgen-cli
	$(MLIN) rustavailable

all:
	mkdir -p $(LIN_DIR)
	$(MLIN) x86_64_defconfig
	linux/scripts/kconfig/merge_config.sh \
		-m \
		-O $(LIN_DIR) \
		$(LIN_DIR)/.config configs/linux_frag.config
	$(MLIN) olddefconfig
	$(MLIN) -j $(shell nproc)

build-busybox:
	mkdir -p $(BB_DIR)
	cp configs/busybox.config $(BB_DIR)/.config
	$(MBB) -j $(shell nproc)
	$(MBB) CONFIG_PREFIX=../rootfs install

initramfs:
	$(LIN_DIR)/usr/gen_init_cpio configs/initramfs.desc > build/initramfs.cpio

qemu:
	qemu-system-x86_64 \
		-kernel $(LIN_DIR)/arch/x86/boot/bzImage \
		-initrd build/initramfs.cpio \
		-append 'console=ttyS0' \
		-M pc \
		-m 1G \
		-nographic

download/busybox:
	mkdir -p download
	wget https://www.busybox.net/downloads/binaries/1.35.0-x86_64-linux-musl/busybox -O $@

clean:
	rm -rf build

.PHONY: default submodules rust all qemu clean
