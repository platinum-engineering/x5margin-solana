# Platinum Engineering - Solana Projects

This is the central repository for projects on the Solana blockchain developed and maintained by Platinum Engineering.

This also serves as a test-bed for experimental approaches to smart-contract development on Solana.

Some highlights:
* Convenience macros for declaring and parsing program instructions.
* Efficient, zero-cost abstractions for working with the onchain Solana environment. Abstractions are checked with an in-house disassembler to make sure they compile to efficient and compact bytecode in BPF. This helps keep the size of the contracts small (cheap!), and leaves plenty of room for business logic code without unnecessary overhead.
* Experimental generation of smart contract bindings for WASM targets, for use in JS environments. This eliminates the need to maintain separate JS or TS bindings to work with the contract data structures and instructions, and improves code deduplication.
* Far fewer dependencies than in the Solana SDK. The eBPF on-chain target compiles with nearly no outside dependencies at all.

Overview:

* `solana-api-types`, A mirror reimplementation of core Solana data types, bridging the necessary types from the Solana SDK with our own extensions and adjustments. Primarily needed to make the projects compile under WASM, as well as making minor changes to the APIs.
* `jsonrpc-client`, an async-compatible client for the Solana JSON-RPC client.
* `solar` and `solar-macros`, an alternative to the official Solana SDK that aims to be a full replacement for it, prioritizing code deduplication and optimized codegen, taking lessons from past development of Solana smart contracts. Also contains various utilities and a more featured client than in `jsonrpc-client`.
* `locker`, a Token-locking smart-contract being developed for Unicrypt.
* `program`, in-house staking pool prototype.
* `wasm-client`, prototype client for WASM targets, exposing basic Solana types, as well as specific types from the two smart-contracts in this crate.
* `disassembler`, an utility tool for inspecting generated eBPF bytecode.