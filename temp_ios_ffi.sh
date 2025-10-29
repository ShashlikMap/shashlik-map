#!/usr/bin/env bash
set -euo pipefail
script_dir=$(dirname "$(realpath "$0")")
crate_dir="$script_dir/ffi-run"
cd "$crate_dir"

# Ensure Rust toolchain (cargo) is available when invoked from Xcode's limited PATH
if [[ -f "$HOME/.cargo/env" ]]; then
  # shellcheck disable=SC1090
  source "$HOME/.cargo/env"
fi
export PATH="$HOME/.cargo/bin:$PATH"

# Set up environment variables to help with library linking
XCODE_TOOLCHAIN_PATH=$(xcode-select --print-path)/Toolchains/XcodeDefault.xctoolchain/usr
export LIBRARY_PATH="${XCODE_TOOLCHAIN_PATH}/lib:${LIBRARY_PATH:-}"
export DYLD_LIBRARY_PATH="${XCODE_TOOLCHAIN_PATH}/lib:${DYLD_LIBRARY_PATH:-}"

if ! command -v cargo >/dev/null 2>&1; then
  echo "[error] 'cargo' not found in PATH. Install Rust via: curl https://sh.rustup.rs -sSf | sh" >&2
  echo "        After installation, restart Xcode or ensure PATH includes $HOME/.cargo/bin" >&2
  exit 10
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "[error] 'jq' not found (needed for cargo metadata parsing). Install via Homebrew: brew install jq" >&2
  exit 11
fi

# Build (debug profile) for arm64 iOS device & simulator.
# Required targets (run once if missing):
#   rustup target add aarch64-apple-ios aarch64-apple-ios-sim

echo "[info] Building ffi-run (debug) for iOS device + simulator"
cargo build --target aarch64-apple-ios
cargo build --target aarch64-apple-ios-sim

# Create a universal (lipo) static lib output directory
OUT_ROOT="$script_dir/iOSDemoApp/generated"
LIB_OUT="$OUT_ROOT/lib"
SWIFT_OUT="$OUT_ROOT/swift"
mkdir -p "$LIB_OUT" "$SWIFT_OUT"

# Incremental skip: if artifacts newer than src and not forced
if [[ -z "${FORCE_UNIFFI_GEN:-}" ]]; then
  if [[ -d "$LIB_OUT/ffi_run.xcframework" && -f "$SWIFT_OUT/ffi_run.swift" ]]; then
    newest_src_ts=$(find "$crate_dir/src" -type f -maxdepth 1 -name '*.rs' -exec stat -f %m {} + 2>/dev/null | sort -nr | head -n1 || echo 0)
    xc_ts=$(stat -f %m "$LIB_OUT/ffi_run.xcframework/Info.plist" 2>/dev/null || echo 0)
    swift_ts=$(stat -f %m "$SWIFT_OUT/ffi_run.swift" 2>/dev/null || echo 0)
    if [[ $xc_ts -ge $newest_src_ts && $swift_ts -ge $newest_src_ts ]]; then
      echo "[info] Skipping regeneration (artifacts newer than sources). Set FORCE_UNIFFI_GEN=1 to override." 
      exit 0
    fi
  fi
fi

WORKSPACE_TARGET_DIR="$(cargo metadata --format-version 1 --no-deps | jq -r '.target_directory')"
DEVICE_LIB="$WORKSPACE_TARGET_DIR/aarch64-apple-ios/debug/libffi_run.a"
SIM_LIB_ARM="$WORKSPACE_TARGET_DIR/aarch64-apple-ios-sim/debug/libffi_run.a"
UNIVERSAL_LIB="$LIB_OUT/libffi_run_universal.a"

if [[ ! -f "$DEVICE_LIB" ]]; then
  echo "[error] Device lib not found: $DEVICE_LIB" >&2
  exit 2
fi

if [[ -f "$SIM_LIB_ARM" ]]; then
  # Device and sim are both arm64: use an xcframework instead of lipo fat (would duplicate arch)
  XCFRAMEWORK_DIR="$LIB_OUT/ffi_run.xcframework"
  rm -rf "$XCFRAMEWORK_DIR"
  xcodebuild -create-xcframework \
    -library "$DEVICE_LIB" \
    -library "$SIM_LIB_ARM" \
    -output "$XCFRAMEWORK_DIR"
  echo "Created XCFramework: $XCFRAMEWORK_DIR"
  UNIVERSAL_LIB="$XCFRAMEWORK_DIR" # For downstream usage variable still referenced
else
  echo "No simulator library built" >&2
  exit 1
fi

# Generate Swift bindings via uniffi-bindgen using the produced cdylib archive.
# uniffi-bindgen expects a dynamic lib by default; we can point to the static archive by providing the .a path.
# NOTE: If any UDL files exist we would specify them; current code uses macro-based scaffolding so generate from lib.

if [[ -d "$UNIVERSAL_LIB" ]]; then
  DEV_STATIC=$(find "$UNIVERSAL_LIB" -type f -path "*ios-arm64*" -name libffi_run.a | head -n 1)
  if [[ -z "$DEV_STATIC" ]]; then
    DEV_STATIC=$(find "$UNIVERSAL_LIB" -type f -name libffi_run.a | head -n 1)
  fi
  echo "[info] Using static lib for uniffi-bindgen: $DEV_STATIC"
  # Use prebuild Swift bindings if they already exist and no force flag
  if [[ -z "${FORCE_UNIFFI_GEN:-}" ]] && [[ -f "$SWIFT_OUT/ffi_run.swift" ]]; then
    echo "[info] Using existing Swift bindings; set FORCE_UNIFFI_GEN=1 to regenerate"
  else
    # Try using cargo with extra environment variables
    echo "[info] Generating Swift bindings from $DEV_STATIC"
    RUSTFLAGS="-L /usr/lib" \
    cargo run --bin uniffi-bindgen generate --library "$DEV_STATIC" --language swift --out-dir "$SWIFT_OUT" || {
      echo "[warn] Failed to generate Swift bindings. Copying from backup if available."
      if [[ -f "$script_dir/ffi-run/bindings/ffi_run.swift" ]]; then
        cp "$script_dir/ffi-run/bindings/ffi_run.swift" "$SWIFT_OUT/"
        cp "$script_dir/ffi-run/bindings/ffi_runFFI.h" "$SWIFT_OUT/"
        cp "$script_dir/ffi-run/bindings/ffi_runFFI.modulemap" "$SWIFT_OUT/"
        echo "[info] Copied Swift bindings from backup"
      else
        echo "[error] No backup bindings available" >&2
        exit 7
      fi
    }
  fi
else
  echo "[info] Using static lib for uniffi-bindgen: $DEVICE_LIB"
  # Try using cargo with extra environment variables
  RUSTFLAGS="-L /usr/lib" \
  cargo run --bin uniffi-bindgen generate --library "$DEVICE_LIB" --language swift --out-dir "$SWIFT_OUT" || {
    echo "[warn] Failed to generate Swift bindings. Copying from backup if available."
    if [[ -f "$script_dir/ffi-run/bindings/ffi_run.swift" ]]; then
      cp "$script_dir/ffi-run/bindings/ffi_run.swift" "$SWIFT_OUT/"
      cp "$script_dir/ffi-run/bindings/ffi_runFFI.h" "$SWIFT_OUT/"
      cp "$script_dir/ffi-run/bindings/ffi_runFFI.modulemap" "$SWIFT_OUT/"
      echo "[info] Copied Swift bindings from backup"
    else
      echo "[error] No backup bindings available" >&2
      exit 7
    fi
  }
fi

echo "[done] Generated Swift bindings in: $SWIFT_OUT"

# Sanity: ensure XCFramework exists where Xcode expects
if [[ ! -d "$LIB_OUT/ffi_run.xcframework" ]]; then
  echo "[warn] XCFramework directory missing: $LIB_OUT/ffi_run.xcframework" >&2
  exit 5
fi
if [[ ! -f "$SWIFT_OUT/ffi_run.swift" ]]; then
  echo "[warn] Swift binding file missing: $SWIFT_OUT/ffi_run.swift" >&2
  exit 6
fi

