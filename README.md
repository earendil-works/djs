# djs

djs is a small JavaScript and TypeScript engine written in `no_std` Rust and built mainly
for wasm32. It runs codemode scripts, model-written programs that call tools, process
their results and return data, and replaces QuickJS in pi's codemode.

- **One arena.** All VM state lives in one byte array addressed by `u32` offsets, so a
  new VM is a copy of a prebuilt image and the host reads results straight out of it.
- **Suspend anywhere.** The interpreter never recurses on the Rust stack for JS calls; it
  can stop between any two instructions and resume, also inside callbacks, getters and
  async functions.
- **Persistence as code.** A run waiting on its tools can be written out as a
  continuation dump: a plain JavaScript program that recreates it.
- **No AST.** A single pass compiles to stack bytecode; TypeScript is erased while
  parsing.
- **Deterministic.** Clock, randomness and weak references come from the host; the same
  inputs give byte-identical heaps, natively and in wasm.

It behaves like QuickJS, down to error messages (dquickts, a QuickJS fork, is the test
oracle), passes <!--n test262/passed-->38,637<!--/n--> of the <!--n test262/active-->38,682<!--/n--> test262 tests in its active group, and
the wasm module is <!--n wasm/brotli kb-->350 KB<!--/n--> brotli-compressed.

**Documentation:** [How djs works](https://earendil-works.github.io/djs/), the guide
(source in `docs/guide/`).

## Using it

```sh
npm install @earendil-works/djs
```

```ts
import { Sandbox } from "@earendil-works/djs";

const sb = await Sandbox.create({
  asyncFunctions: { search: (q) => mySearch(q as string) },
  memoryLimit: 64 << 20,
});
const n = await sb.runAsync("const hits = await search(q); return hits.length", { q: "djs" });
```

See `js/README.md` for the package and `js/pi/vm.ts` for pi's codemode adapter.

## Building and testing

Needs stable Rust (edition 2024) with the `wasm32-unknown-unknown` target and
`rust-src`, and node 22.6+. Oracle tests need dquickts' `qjs` (`$DJS_QJS`).

```sh
cargo build                                  # the workspace
cargo run -p djs-cli -- run file.js          # run a script (also: repl, disasm)
scripts/build-wasm.sh                        # the wasm module
cargo test                                   # unit and integration tests
cargo run -p djs-docs -- build               # the guide: target/docs-site/
```

`TESTING.md` covers test262, the differential corpus and the stress modes;
`scripts/gate.sh` is what every change to `main` must pass.

## Further reading

- `ARCH.md`: the architecture.
- `TESTING.md`: how djs is tested.
- `docs/decisions.md`: the decision log (`docs/decisions-index.md` by topic).
- `AGENTS.md`, `work/README.md`: how work is organized.
