// Linker flags for the wasm module (D1570).
//
// wasm-ld writes every relocated immediate (call targets, data addresses, the stack
// pointer global) as a padded 5-byte LEB so that it can patch it in place, and nothing
// compacts them afterwards: ~7% of the code section (0052). `--compress-relocations`
// writes minimal LEBs instead. wasm-ld rejects it when the output keeps debug info, so
// it is only passed for builds without (the `wasm` profile; not the gate's debug build).
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let wasm = std::env::var("CARGO_CFG_TARGET_ARCH").is_ok_and(|a| a == "wasm32");
    let debug = std::env::var("DEBUG").is_ok_and(|d| d != "false" && d != "0");
    if wasm && !debug {
        println!("cargo:rustc-link-arg-cdylib=--compress-relocations");
    }
    // The shadow stack (D1941): 256 KiB instead of rustc's 1 MiB default. It is part of
    // every instance's linear memory from the start, and the parser's deepest accepted
    // nesting needs at most ~170 KiB (docs/compiler.md 2.4). DJS_WASM_STACK overrides it
    // for measurements.
    println!("cargo:rerun-if-env-changed=DJS_WASM_STACK");
    if wasm {
        let n = std::env::var("DJS_WASM_STACK").unwrap_or_else(|_| "262144".into());
        println!("cargo:rustc-link-arg-cdylib=-zstack-size={n}");
    }
}
