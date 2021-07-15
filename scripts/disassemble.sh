SCRIPT_DIR=$(dirname "$0")
$SCRIPT_DIR/build.sh
rbpf -u disassembler -e $SCRIPT_DIR/../target/bpfel-unknown-unknown/bpf-release/platinum_trade.so > $SCRIPT_DIR/../disassembled.txt