#!/bin/bash

# Exit immediately if a command fails
set -euo pipefail

TARGET_PATH="/home/admin"
SOCKET="/tmp/ssh-socket-%r@%h:%p"

FEATURE_FLAG="${RENDER_MODE}"

RUN_IN_BACKGROUND=false

# Build inside an arm64 Debian container that matches the Pi (see kms.Dockerfile).
# Cargo's target dir lives in a Docker volume, not in the bind-mounted repo:
# on Docker Desktop for macOS the bind mount mixes second and nanosecond mtimes,
# which makes cargo think build outputs are stale and rebuild on every run.
# Only the finished binary is copied back to target/kms/ on the host.
RUST_VERSION="$(sed -n 's/^channel *= *"\(.*\)"/\1/p' rust-toolchain.toml)"
BUILDER_IMAGE="shashlik-kms-builder:${RUST_VERSION}"
HOST_OUT_DIR="target/kms"
BINARY="$HOST_OUT_DIR/$APP_NAME"

echo "--- Preparing build image $BUILDER_IMAGE ---"
# No build context needed: the Dockerfile is piped in on stdin.
docker build --platform linux/arm64 \
    --build-arg "RUST_VERSION=$RUST_VERSION" \
    -t "$BUILDER_IMAGE" - < kms.Dockerfile

echo "--- Building $APP_NAME ---"
docker run --rm --platform linux/arm64 \
    -v "$PWD:/src" \
    -v shashlik-kms-cargo-registry:/usr/local/cargo/registry \
    -v shashlik-kms-cargo-git:/usr/local/cargo/git \
    -v shashlik-kms-target:/target \
    -e CARGO_TARGET_DIR=/target \
    "$BUILDER_IMAGE" \
    sh -c 'cargo build --features "$1" --package "$2" --release \
        && mkdir -p "/src/$3" && cp "/target/release/$2" "/src/$3/"' \
    sh "$FEATURE_FLAG" "$APP_NAME" "$HOST_OUT_DIR"

# 2. Open Master Connection (Asks for password now)
echo "--- Opening Master Connection to $TARGET_HOST ---"
ssh -M -f -N -o ControlPersist=600 -S "$SOCKET" "$TARGET_HOST"

# Trap to ensure the background connection is closed on exit/interrupt
trap 'ssh -S "$SOCKET" -O exit "$TARGET_HOST" 2>/dev/null; echo -e "\n--- Connection closed ---"; exit' INT TERM EXIT

# 3. Deploy (Uses existing socket)
echo "--- Deploying to $TARGET_HOST ---"
scp -o "ControlPath=$SOCKET" "$BINARY" "$TARGET_HOST:$TARGET_PATH/"

echo "--- Starting application ---"
if [ "$RUN_IN_BACKGROUND" = true ]; then
    echo "Running in BACKGROUND (nohup)..."
    # -f tells SSH to background itself; redirects ensure it detaches fully
    ssh -S "$SOCKET" -f "$TARGET_HOST" "chmod +x $TARGET_PATH/$APP_NAME && sudo nohup $TARGET_PATH/$APP_NAME > /dev/null 2>&1 &"
else
    echo "Running INTERACTIVELY (attached)..."
    # -t is required for interactive sudo/TTY behavior
    ssh -S "$SOCKET" -t "$TARGET_HOST" "chmod +x $TARGET_PATH/$APP_NAME && sudo $TARGET_PATH/$APP_NAME"
fi
