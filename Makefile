MLIN = $(MAKE) -C linux O=../build LLVM=1

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

all: download/busybox
	mkdir -p build
	$(MLIN) x86_64_defconfig
	linux/scripts/kconfig/merge_config.sh -m -O build build/.config configs/linux_frag.config
	$(MLIN) olddefconfig
	$(MLIN) -j $(shell nproc)

download/busybox:
	mkdir -p download
	wget https://www.busybox.net/downloads/binaries/1.35.0-x86_64-linux-musl/busybox -O $@

clean:
	rm -rf build

clean-all:
	rm -rf build download

.PHONY: default submodules rust all clean clean-all
