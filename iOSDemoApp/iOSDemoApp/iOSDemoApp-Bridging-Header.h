#ifndef IOSDEMOAPP_BRIDGING_HEADER_H
#define IOSDEMOAPP_BRIDGING_HEADER_H
// Fallback bridging header: expose UniFFI low-level FFI definitions when Clang module import fails.
// This lets Swift see RustBuffer, ForeignBytes, RustCallStatus, and FFI function symbols.
#include "../generated/swift/ffi_runFFI.h"
#endif // IOSDEMOAPP_BRIDGING_HEADER_H
