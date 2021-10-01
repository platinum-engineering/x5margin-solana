SCRIPT_DIR=$(dirname "$0")
$SCRIPT_DIR/build-locker.sh

cd $SCRIPT_DIR/..
cargo run --package bpf-disassembler --release -- locker-so/target/bpfel-unknown-unknown/release/locker.so > disassembled.txt