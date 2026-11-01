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

Builds: `scripts/build-wasm.sh` (shipped, stripped); for names
`CARGO_PROFILE_WASM_STRIP=debuginfo cargo build -p djs-wasm --target wasm32-unknown-unknown
--profile wasm`. The base image is `target/.../build/djs-prelude-*/out/base.img`.
