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
    // The Unicode tables as custom sections (D3350): array literals in `pub` statics
    // without `#[used]`, as djs-prelude does for `djs_base` (through `include_bytes!` the
    // bytes would stay in the data segment too).
    if std::env::var_os("CARGO_FEATURE_COLD_UNICODE").is_some() {
        let mut out = String::new();
        for (section, file) in [("djs_props", "props.bin"), ("djs_norm", "norm.bin")] {
            let path = format!("../djs-core/src/unicode/{file}");
            println!("cargo:rerun-if-changed={path}");
            let bytes = std::fs::read(&path).expect("cannot read the Unicode tables");
            let list: Vec<String> = bytes.iter().map(|b| b.to_string()).collect();
            out += &format!(
                "#[unsafe(link_section = \"{section}\")]\npub static {}: [u8; {}] = [{}];\n",
                section.to_uppercase(),
                bytes.len(),
                list.join(",")
            );
        }
        let dir = std::env::var("OUT_DIR").unwrap();
        std::fs::write(std::path::Path::new(&dir).join("tables_sections.rs"), out)
            .expect("cannot write the table sections");
    }
    println!("cargo:rerun-if-env-changed=CARGO_PROFILE_WASM_STRIP");
    let wasm = std::env::var("CARGO_CFG_TARGET_ARCH").is_ok_and(|a| a == "wasm32");
    let debug = std::env::var("DEBUG").is_ok_and(|d| d != "false" && d != "0");
    let names = std::env::var("CARGO_PROFILE_WASM_STRIP").is_ok_and(|s| s == "false");
    if wasm && !debug && !names {
        println!("cargo:rustc-link-arg-cdylib=--compress-relocations");
    }
    // The shadow stack (D1941, D2540, D3280). It is part of every instance's linear memory
    // from the start. The release modules link 64 KiB: compiles nest to the small stack's
    // limits there, and a host moves the stack to a 128 KiB block for one that nests
    // deeper (the elastic stack). Debug builds (larger frames) and modules with snapshots
    // (whose loads compile the scripts without the host) keep 128 KiB, where the parser's
    // deepest accepted nesting needs at most ~115 KiB (docs/compiler.md 2.4).
    // DJS_WASM_STACK overrides it for measurements.
    println!("cargo:rerun-if-env-changed=DJS_WASM_STACK");
    if wasm {
        let snapshot = std::env::var("CARGO_FEATURE_SNAPSHOT").is_ok();
        let small = if debug || snapshot { "131072" } else { "65536" };
        let n = std::env::var("DJS_WASM_STACK").unwrap_or_else(|_| small.into());
        println!("cargo:rustc-link-arg-cdylib=-zstack-size={n}");
        // Exported so that a host can tell a stack overflow from other traps: the pointer
        // wraps below 0, past the end of memory (js/src/trap.ts, D2543).
        println!("cargo:rustc-link-arg-cdylib=--export=__stack_pointer");
    }
}
