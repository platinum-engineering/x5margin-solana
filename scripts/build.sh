#!/bin/bash

SCRIPT_DIR=$(dirname "$0")
source $SCRIPT_DIR/setup-bpf-env.sh
cargo +bpf build -Z unstable-options --target bpfel-unknown-unknown --profile bpf-release --package x5margin-program-so
