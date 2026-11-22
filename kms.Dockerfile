# Build image for the Raspberry Pi (Linux KMS) binaries.
#
# The Pi runs 64-bit Debian 13 (trixie). On an Apple Silicon Mac this image is
# arm64 too, so cargo builds natively for the Pi: no cross-compiler, no sysroot,
# no emulation. Keep the Debian release in sync with the Pi so the binary never
# links against a newer glibc than the device has.
#
# kms_deploy.sh builds this image and passes RUST_VERSION from rust-toolchain.toml.
ARG RUST_VERSION
FROM --platform=linux/arm64 rust:${RUST_VERSION}-trixie

# -dev packages for the system libraries the linuxkms backend links against.
# OpenSSL is vendored (built from source by openssl-src), so no libssl-dev.
RUN apt-get update \
    && apt-get install --assume-yes --no-install-recommends \
        libxkbcommon-dev libwayland-dev libx11-dev libfontconfig1-dev \
        libudev-dev libinput-dev libgbm-dev \
        clang libclang-dev cmake pkg-config \
    && rm -rf /var/lib/apt/lists/*

# rust-toolchain.toml also lists the Android target; install it up front so
# rustup doesn't download it on every build.
ARG RUST_VERSION
RUN rustup target add aarch64-linux-android --toolchain "${RUST_VERSION}"

WORKDIR /src
