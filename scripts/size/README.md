# Size attribution tools (0122)

For the D020 budget (gzip of the shipped module within 1.5x quickjs-wasi's). With the
older tools in scripts/: `wasm-size.py` (sections, code by crate/path/function),
`wasm-funcs.py`, `wasm-dwarf-lines.py` (code bytes by source line, inlining included),
`native-sizes.sh` (generated natives), `wasm-size-history.sh` (size per commit), and
`cargo run -p djs-prelude --example image-size` (the base image by cell kind and role).

| script | what |
| --- | --- |
| `sections.py MODULE [BASE.img]` | gzip of module, code, data, base image and the rest of the data; the ratio to quickjs-wasi |
| `image-kinds.py BASE.img` | the base image by cell kind, gzip per kind, and in its order vs regrouped |
| `families.py MODULE` | code per generic family (monomorphization) |
| `budget.py MODULE` | the gate's budget check (TESTING.md) |
| `datamap.py MAP.txt MODULE` | the data section by symbol and category from a wasm-ld link map (0156) |
| `sandbox-memory.ts [MODULE] [N]` | a fresh Sandbox's wasm memory: pages, arena length and capacity, RSS per sandbox (0156; node --expose-gc) |
| `vec-sites.py NAMES [--check\|--update scripts/size/vec-sites.txt]` | inlined container glue (push, drop, with_capacity, reserve) per function, against a baseline (0161, D2521) |
| `patches/onepath.py` | variant: no `@native`, `@fast` or `run_hot` (`ONEPATH=native,fast,hot`) (0156) |
| `patches/optsize.py` | variant: `#[optimize(size)]` on cold modules or `OPTSIZE=all` (0156) |

Builds: `scripts/build-wasm.sh` (shipped, stripped); for names
`CARGO_PROFILE_WASM_STRIP=debuginfo cargo build -p djs-wasm --target wasm32-unknown-unknown
--profile wasm`. The base image is `target/.../build/djs-prelude-*/out/base.img`.
