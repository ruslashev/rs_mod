LIN_DIR = build/linux
BB_DIR = build/busybox

MLIN = $(MAKE) -C linux O=../$(LIN_DIR) LLVM=1
MBB = $(MAKE) -C busybox O=../$(BB_DIR)

default:
	$(MAKE) submodules
	$(MAKE) rust
	$(MAKE) linux-config
	$(MAKE) build-linux
	$(MAKE) build-busybox
	$(MAKE) rootfs

submodules:
	git submodule update --init

rust:
	rustup override set $(shell linux/scripts/min-tool-version.sh rustc)
	rustup component add rust-src
	cargo install --locked --version $(shell linux/scripts/min-tool-version.sh bindgen) bindgen-cli
	$(MLIN) rustavailable

linux-config:
	mkdir -p $(LIN_DIR)
	$(MLIN) x86_64_defconfig
	linux/scripts/kconfig/merge_config.sh \
		-m \
		-O $(LIN_DIR) \
		$(LIN_DIR)/.config configs/linux_frag.config
	$(MLIN) olddefconfig

build-linux:
	$(MLIN) -j $(shell nproc)

build-busybox:
	mkdir -p $(BB_DIR)
	cp configs/busybox.config $(BB_DIR)/.config
	$(MBB) -j $(shell nproc)
	$(MBB) CONFIG_PREFIX=../rootfs install

rootfs:
	rm -f build/rootfs.img
	cp -rTu overlay build/rootfs
	mkdir -p build/rootfs/dev
	mkdir -p build/rootfs/proc
	mkdir -p build/rootfs/root
	mkdir -p build/rootfs/sys
	truncate --size=32M build/rootfs.img
	fakeroot /sbin/mkfs.ext4 -d build/rootfs build/rootfs.img -E root_owner=0:0,no_copy_xattrs

qemu:
	qemu-system-x86_64 \
		-kernel $(LIN_DIR)/arch/x86/boot/bzImage \
		-drive file=build/rootfs.img,if=virtio,format=raw \
		-append "rootwait rw root=/dev/vda console=ttyS0" \
		-M pc \
		-m 1G \
		-nographic

download/busybox:
	mkdir -p download
	wget https://www.busybox.net/downloads/binaries/1.35.0-x86_64-linux-musl/busybox -O $@

clean:
	rm -rf build

.PHONY: default submodules rust all qemu clean
