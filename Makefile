PI_ARCH := armv7-unknown-linux-gnueabihf

.PHONY: build-pi
build-pi:
	cross build --target armv7-unknown-linux-gnueabihf --release

.PHONY: copy-to-pi
copy-to-pi:
	scp target/$(PI_ARCH)/release/matryx_generator pi@raspberrypi.mora-jazz.ts.net:/tmp
