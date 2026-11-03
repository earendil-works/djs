// Linker flags for the wasm module (D1570).
//
// wasm-ld writes every relocated immediate (call targets, data addresses, the stack
// pointer global) as a padded 5-byte LEB so that it can patch it in place, and nothing
// compacts them afterwards: ~7% of the code section (0052). `--compress-relocations`
// writes minimal LEBs instead. wasm-ld rejects it when the output keeps debug info, so
// it is only passed for builds without (the `wasm` profile; not the gate's debug build,
// and not `scripts/build-wasm.sh --keep-names`, whose name section counts as debug info:
// its function sizes include the padding).
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=CARGO_PROFILE_WASM_STRIP");
    let wasm = std::env::var("CARGO_CFG_TARGET_ARCH").is_ok_and(|a| a == "wasm32");
    let debug = std::env::var("DEBUG").is_ok_and(|d| d != "false" && d != "0");
    let names = std::env::var("CARGO_PROFILE_WASM_STRIP").is_ok_and(|s| s == "false");
    if wasm && !debug && !names {
        println!("cargo:rustc-link-arg-cdylib=--compress-relocations");
    }
    // The shadow stack (D1941, D2540): 128 KiB instead of rustc's 1 MiB default. It is
    // part of every instance's linear memory from the start, and the parser's deepest
    // accepted nesting needs at most ~115 KiB (docs/compiler.md 2.4). DJS_WASM_STACK
    // overrides it for measurements.
    println!("cargo:rerun-if-env-changed=DJS_WASM_STACK");
    if wasm {
        let n = std::env::var("DJS_WASM_STACK").unwrap_or_else(|_| "131072".into());
        println!("cargo:rustc-link-arg-cdylib=-zstack-size={n}");
        // Exported so that a host can tell a stack overflow from other traps: the pointer
        // wraps below 0, past the end of memory (js/src/trap.ts, D2543).
        println!("cargo:rustc-link-arg-cdylib=--export=__stack_pointer");
    }
}
