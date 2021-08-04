SCRIPT_DIR=$(dirname "$0")
$SCRIPT_DIR/build.sh

cd $SCRIPT_DIR/..
cargo run --package bpf-disassembler -- target/bpfel-unknown-unknown/bpf-release/x5margin.so > disassembled.txt