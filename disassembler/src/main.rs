use solana_rbpf::{
    ebpf,
    elf::EBpfElf,
    static_analysis::Analysis,
    user_error::UserError,
    vm::{Config, DefaultInstructionMeter},
};

fn csize(opc: u8) -> u32 {
    match opc {
        ebpf::BPF_W => 32,
        ebpf::BPF_H => 16,
        ebpf::BPF_DW => 64,
        ebpf::BPF_B => 8,
        _ => 0,
    }
}

fn alu_op(op: u8) -> &'static str {
    match op {
        ebpf::BPF_ADD => "+=",
        ebpf::BPF_SUB => "-=",
        ebpf::BPF_MUL => "*=",
        ebpf::BPF_DIV => "/=",
        ebpf::BPF_OR => "|=",
        ebpf::BPF_AND => "&=",
        ebpf::BPF_LSH => "<<=",
        ebpf::BPF_RSH => ">>=",
        ebpf::BPF_MOD => "%=",
        ebpf::BPF_XOR => "^=",
        ebpf::BPF_MOV => "=",
        ebpf::BPF_ARSH => ">>=",
        _ => "?=?",
    }
}

fn jmp_op(op: u8) -> &'static str {
    match op {
        ebpf::BPF_JEQ => "==",
        ebpf::BPF_JSET => "&",
        ebpf::BPF_JNE => "!=",

        ebpf::BPF_JGT => ">",
        ebpf::BPF_JGE => ">=",
        ebpf::BPF_JSGT => ">",
        ebpf::BPF_JSGE => ">=",

        ebpf::BPF_JLT => "<",
        ebpf::BPF_JLE => "<=",
        ebpf::BPF_JSLT => "<",
        ebpf::BPF_JSLE => "<=",

        _ => "???",
    }
}

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let file = &args[1];
    let file = std::fs::read(&file).unwrap();

    let config = Config::default();
    let exec = EBpfElf::<UserError, DefaultInstructionMeter>::load(config, &file).unwrap();

    let analysis = Analysis::from_executable(&exec);
    let mut sizes = vec![];

    let mut current_fn = "<N/A>".to_string();
    let mut current_fn_size = 0;
    for insn in &analysis.instructions {
        let pc = insn.ptr;
        if let Some(cfg_node) = analysis.cfg_nodes.get(&pc) {
            let is_function = analysis.functions.contains_key(&pc);
            if is_function {
                if current_fn_size > 0 {
                    sizes.push((current_fn, current_fn_size));
                }

                current_fn = cfg_node.label.clone();
                current_fn_size = 0;
                println!();
            }

            if !cfg_node.sources.is_empty() || is_function {
                // println!(
                //     "{}: (src={}, dest={})",
                //     cfg_node.label,
                //     cfg_node.sources.len(),
                //     cfg_node.destinations.len()
                // );
                println!("{}:", cfg_node.label,);
            }
        }

        let class = insn.opc & 0b0000_0111;
        print!("    ");

        if class == ebpf::BPF_LD {
            let size = insn.opc & 0b0001_1000;
            let mode = insn.opc & 0b1110_0000;

            if mode == ebpf::BPF_ABS {
                print!("[{:2}] r0 = *({:#x})", csize(size), insn.imm);
            } else if mode == ebpf::BPF_IND {
                print!("[{:2}] r0 = *r{}", csize(size), insn.src);
            } else if mode == ebpf::BPF_IMM {
                print!("[64] r{} = {:#x}", insn.dst, insn.imm)
            } else if mode == ebpf::BPF_MEM {
                panic!("unknown opcode");
            }

            // print!("ld s={:x} m={:x}", size, mode);
            println!();
        } else if class == ebpf::BPF_LDX {
            let size = insn.opc & 0b0001_1000;

            if insn.off != 0 {
                println!(
                    "[{:2}] r{} = *(r{} + {:#x})",
                    csize(size),
                    insn.dst,
                    insn.src,
                    insn.off,
                );
            } else {
                println!("[{:2}] r{} = *r{}", csize(size), insn.dst, insn.src,);
            }
        } else if class == ebpf::BPF_ST {
            let size = insn.opc & 0b0001_1000;

            if insn.off != 0 {
                println!(
                    "[{:2}] *(r{} + {:#x}) = {}",
                    csize(size),
                    insn.dst,
                    insn.off,
                    insn.imm
                );
            } else {
                println!("[{:2}] *r{}: = {}", csize(size), insn.dst, insn.imm);
            }
        } else if class == ebpf::BPF_STX {
            let size = insn.opc & 0b0001_1000;

            if insn.off != 0 {
                println!(
                    "[{:2}] *(r{} + {:#x}) = r{}",
                    csize(size),
                    insn.dst,
                    insn.off,
                    insn.src
                );
            } else {
                println!("[{:2}] *r{} = r{}", csize(size), insn.dst, insn.src);
            }
        } else if class == ebpf::BPF_ALU || class == ebpf::BPF_ALU64 {
            let is_immediate = insn.opc & 0b1000 == 0;
            let width = if class == ebpf::BPF_ALU64 { 64 } else { 32 };
            let op = insn.opc & 0b1111_0000;

            print!("[{:2}] ", width);

            let arg = if is_immediate {
                format!("{:#x}", insn.imm)
            } else {
                format!("r{}", insn.src)
            };

            if op == ebpf::BPF_NEG {
                print!("r{} = {}", insn.dst, arg);
            } else if op == ebpf::BPF_END {
                if is_immediate {
                    print!("r{} = to_le<{}>(r{})", insn.dst, insn.imm, insn.dst);
                } else {
                    print!("r{} = to_be<{}>(r{})", insn.dst, insn.imm, insn.dst);
                }
            } else {
                print!("r{} {} {}", insn.dst, alu_op(op), arg);
            }

            println!();
        } else if class == ebpf::BPF_JMP {
            let is_immediate = insn.opc & 0b1000 == 0;
            let op = insn.opc & 0b1111_0000;

            let arg = if is_immediate {
                format!("{:#x}", insn.imm)
            } else {
                format!("r{}", insn.src)
            };

            if op == ebpf::BPF_CALL {
                if insn.opc == ebpf::CALL_IMM {
                    if let Some(syscall) = analysis.syscalls.get(&(insn.imm as u32)) {
                        if syscall == "abort" {
                            println!("abort");
                        } else {
                            println!("syscall r0 = {}(r1, r2, r3, r4, r5)", syscall);
                        }
                    } else {
                        let label = analysis
                            .executable
                            .lookup_bpf_function(insn.imm as u32)
                            .and_then(|pc| analysis.cfg_nodes.get(&pc))
                            .map(|node| node.label.as_str())
                            .unwrap_or("[unknown]");

                        println!("call {}", label);
                    }
                } else if insn.opc == ebpf::CALL_REG {
                    println!("callx {:#x}", insn.imm)
                }
            } else if op == ebpf::BPF_EXIT {
                println!("exit");
            } else if op == ebpf::BPF_JA {
                let target = analysis
                    .cfg_nodes
                    .get(&((pc as isize + insn.off as isize + 1) as usize))
                    .expect("invalid jump destination");

                println!("goto {}", target.label)
            } else {
                let target = analysis
                    .cfg_nodes
                    .get(&((pc as isize + insn.off as isize + 1) as usize))
                    .expect("invalid jump destination");

                println!(
                    "if r{} {} {} {{ goto {} }}",
                    insn.dst,
                    jmp_op(op),
                    arg,
                    target.label
                )
            }
        }

        current_fn_size += 1;
    }

    let total_size: usize = sizes.iter().map(|(_, s)| *s).sum();

    sizes.sort_by_key(|(_, s)| *s);
    sizes.reverse();
    println!();
    println!();
    println!("function sizes:");

    for (label, size) in sizes {
        let part = (size as f64 / total_size as f64) * 100.0;
        println!("[{:.1}%] {}: {}", part, label, size);
    }
}
