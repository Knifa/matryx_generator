PI_ARCH := aarch64-unknown-linux-gnu

.PHONY: build-pi
build-pi:
	cross build --target aarch64-unknown-linux-gnu --release

.PHONY: copy-to-pi
copy-to-pi:
	scp target/$(PI_ARCH)/release/matryx_generator matryx-pi:/tmp
