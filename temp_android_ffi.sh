#!/usr/bin/env bash
script_location=$(dirname "$(realpath $0)")
echo "UniFFI script location: ${script_location}"
set -e
cd "$script_location"/ffi-run
cargo ndk -t arm64-v8a -o ./../AndroidDemoApp/app/src/main/jniLibs build
cargo run --bin uniffi-bindgen generate --library ../AndroidDemoApp/app/src/main/jniLibs/arm64-v8a/libffi_run.so --language kotlin --out-dir ../AndroidDemoApp/app/src/main/java/