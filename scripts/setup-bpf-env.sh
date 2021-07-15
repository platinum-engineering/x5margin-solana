#!/bin/bash

export SDK_DIR="$HOME/.local/share/solana/install/active_release/solana-release/bin/sdk/bpf"
export LLVM_DIR="$SDK_DIR/dependencies/bpf-tools/llvm/bin"
export RUSTC_BIN="$SDK_DIR/dependencies/bpf-tools/rust/bin/rustc"

export CC="$LLVM_DIR/clang"
export AR="$LLVM_DIR/llvm-ar"
export OBJDUMP="$LLVM_DIR/llvm-objdump"
export OBJCOPY="$LLVM_DIR/llvm-objcopy"
export RUSTFLAGS=""