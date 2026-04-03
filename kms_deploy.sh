#!/bin/bash

# Exit immediately if a command fails
set -euo pipefail

APP_NAME="winit-run"
TARGET_PATH="/home/admin"
TARGET_TRIPLE="aarch64-unknown-linux-gnu"
SOCKET="/tmp/ssh-socket-%r@%h:%p"

RUN_IN_BACKGROUND=false

echo "--- Building $APP_NAME ---"
CROSS_CONTAINER_OPTS="--platform linux/amd64" cross build \
    --package "$APP_NAME" \
    --target "$TARGET_TRIPLE" \
    --release

# 2. Open Master Connection (Asks for password now)
echo "--- Opening Master Connection to $TARGET_HOST ---"
ssh -M -f -N -o ControlPersist=600 -S "$SOCKET" "$TARGET_HOST"

# Trap to ensure the background connection is closed on exit/interrupt
trap 'ssh -S "$SOCKET" -O exit "$TARGET_HOST" 2>/dev/null; echo -e "\n--- Connection closed ---"; exit' INT TERM EXIT

# 3. Deploy (Uses existing socket)
echo "--- Deploying to $TARGET_HOST ---"
scp -o "ControlPath=$SOCKET" "target/$TARGET_TRIPLE/release/$APP_NAME" "$TARGET_HOST:$TARGET_PATH/"

echo "--- Starting application ---"
if [ "$RUN_IN_BACKGROUND" = true ]; then
    echo "Running in BACKGROUND (nohup)..."
    # -f tells SSH to background itself; redirects ensure it detaches fully
    ssh -S "$SOCKET" -f "$TARGET_HOST" "chmod +x $TARGET_PATH/$APP_NAME && sudo SLINT_BACKEND=linuxkms nohup $TARGET_PATH/$APP_NAME > /dev/null 2>&1 &"
else
    echo "Running INTERACTIVELY (attached)..."
    # -t is required for interactive sudo/TTY behavior
    ssh -S "$SOCKET" -t "$TARGET_HOST" "chmod +x $TARGET_PATH/$APP_NAME && sudo SLINT_BACKEND=linuxkms $TARGET_PATH/$APP_NAME"
fi