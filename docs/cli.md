# djs-cli: the `djs` command

```sh
cargo run -p djs-cli -- run [--body] [--ts] [--experimental-decorators] [--emit-decorator-metadata]
    [--gc-stress] [--suspend-every N] [--backend bytecode|native] file.js
cargo run -p djs-cli -- disasm [--body] file.js
cargo run -p djs-cli -- heap [--json] [--top N] [--only SECTIONS] [--diff FILE|base|none]
    [--reference FILE] dump
cargo run -p djs-cli -- strip-types file.ts
cargo run -p djs-cli -- repl [--ts] [--gc-stress] [--no-history]
```

`run` runs a Script (or, with `--body`, a codemode async body) and serves `console.log`;
`--experimental-decorators` and `--emit-decorator-metadata` (which implies it) select
TypeScript's legacy decorators (D1651). `--backend` runs the `@native` builtins as their
generated Rust state machines (`native`, the default) or as bytecode (D1635).
`disasm` prints the bytecode. `strip-types` erases TypeScript types like ts-blank-space
(TESTING.md). `heap` analyzes a heap dump (below). This file documents `heap` and the
REPL.

## `djs heap`

What is in a heap dump (D2264): a raw image (`.snapshot` of the REPL, `Vm::bytes`), a
snapshot container (`Sandbox.bytes()`, pi's `snapshot()`, D2260) or an older DJSS/DJPI
snapshot. Sections (`--only summary,kinds,...` picks some; `--json` prints one object with
a key per section):

| section | shows |
| --- | --- |
| `summary` | format, build, image size, live vs garbage (and what a collection leaves), old region (`OLD_END`) vs young |
| `kinds` | cells and bytes per cell kind (garbage, old) and per object class |
| `diff` | against the reference image: unchanged cells in place or moved, changed, new (grouped into subtrees with their path and the new bytes only they keep alive), gone; how much of the image sits at the reference's offsets. This is what a snapshot compressed against the reference costs |
| `largest` | the largest cells and the largest retained sizes (dominator tree), with paths like `globalThis.cache[3].body` |
| `strings` | flat strings and ropes, duplicated contents, the largest strings |
| `shapes` | shapes, objects with a shape or in dictionary mode, the most common key lists |
| `functions` | templates and bytecode per script, closures, builtin closures by intrinsic, boxes |
| `session` | run state, queued jobs, the inbox, pending host calls, promises, suspended async functions and generators with their locals |

The diff compares with a container's reference by default, or with this build's base
for a raw image of this build; `--diff FILE` picks another image, `--diff none` skips it.
A container compressed against the base decompresses with this build's base when the
hashes match. Anything else needs its reference: `node scripts/heap/reference.ts base
[--wasm MODULE] -o base.img` writes a module's base, and `node scripts/heap/reference.ts
pi-prelude --data START.json -o prelude.img` pi's prelude image for a session's start
data (tools, globals, store, memory limit, deterministic mode;
scripts/snapshot-research/wasm.ts writes its cases' as `start.json`). Then `djs heap
--reference prelude.img session.djsn`.

Paths read like JavaScript where the heap allows: `.name` for properties (from the
object's shape or table), `[i]` for elements, `~name` for captured variables, `<frame>.x`
for a suspended function's locals, `<proto>`, `handle[n]` for the embedder's handles,
and the layout's field names in brackets for internal cells (`<atoms><entries>`).

## `djs repl`

An interactive REPL for debugging the engine and scripts (issue 0064), built only on the
public `Vm` API. djs's own features are commands: fuel, suspension, heap images, bytecode
and the GC.

```text
> let o = {a: [1, 2], m: new Map([[1, 'x']])}
> o
{ a: [ 1, 2 ], m: Map(1) { 1 => 'x' } }
> function spin(n) { let s = 0; for (let i = 0; i < n; i++) s += i; return s }
> .suspend-every 300
suspending every 300 fuel
> spin(50)
[suspended, 300 fuel] at spin (repl:1:55)
[suspended, 600 fuel] at spin (repl:1:59)
[suspended, 900 fuel] at spin (repl:1:49)
1225
```

### Commands

| command | does |
| --- | --- |
| `.help`, `.exit` | the list; leave (Ctrl-D on an empty line leaves too) |
| `.load <file>` | runs a file as one input: a Script, or a top-level-await body. `.ts` files are TypeScript |
| `.ts [on\|off]` | parse inputs as TypeScript (`--ts` at start) |
| `.decorators [tc39\|legacy\|metadata]` | TypeScript decorators: the standard ones (the default), TypeScript's `experimentalDecorators`, or those with `emitDecoratorMetadata` (D1651). Legacy decorators need TypeScript mode |
| `.disasm <code>` | if `code` is an expression that evaluates to a function (a bound one is followed to its target), that function's bytecode with its nested functions. Otherwise the bytecode of `code` itself, compiled and not run. The expression *is* evaluated |
| `.fuel [n\|off]` | without an argument: the fuel the last input used, and the limit. With `n`: each input may use `n` fuel, then it is stopped (`vm.abort()`, the globals stay) |
| `.suspend-every [n\|off]` | run inputs in slices of `n` fuel. At every stop, print the fuel so far and the innermost frame (`Vm::backtrace`), then continue in a new VM restored from the heap image, as `djs run --suspend-every` does |
| `.step [n\|off]` | pause a running input every `n` fuel (default 1000) at a nested `step>` prompt that shows the innermost frame: Enter (or `s`/`n`) runs the next slice, `c` runs to the end, `bt` prints all frames (`Vm::backtrace`), `fuel` the fuel so far, `q` (or Ctrl-C) stops the input (`vm.abort()`, the globals stay). In line mode the prompt reads the next input lines |
| `.complete <text>` | the Tab completions for `text` (what Tab offers in the editor) |
| `.time [on\|off]` | after each input: wall time (compile and run) and fuel |
| `.gc` | collect; the arena size before and after |
| `.heap` | arena bytes used, allocated and the limit, handles, atoms, then cells per kind and objects per class (count and bytes), largest first |
| `.snapshot <file>` | collect, then write the heap image (`vm.bytes()`): the whole VM |
| `.restore <file>` | replace the VM with `Vm::from_bytes` of a file. It must come from the same build (the build hash is checked). Images from `djs repl` and `djs run` both work: `console` is host function 1 in both |
| `.clear` | a fresh VM |

### Input

- **Multi-line input.** A line that leaves the input incomplete (the compile error is at
  its end, or it is an unterminated template or comment) prompts `... ` for more. Ctrl-C
  drops the unfinished input.
- **Persistence.** Inputs are Scripts (D004): top-level `let`/`const`/`class` live in the
  global lexical scope and `var`/functions on the global object, so later inputs see
  them. Redeclaring a `let` is a SyntaxError, as between two Scripts.
- **Results.** A Script prints its completion value (issue 0058, D1581), except the
  `undefined` of an input that starts with a declaration (`let`, `function`, `class`,
  TypeScript's `enum`/`type`/..., ...). Two kinds of input run as the async body
  `return [(input\n)];` instead: an expression a Script cannot run (`await f()`), and an
  input that starts with `{`, which prints as an object literal as in node (`{a: 1}` →
  `{ a: 1 }`; a block that is not an expression still runs as a Script). The array keeps
  the body from adopting a promise value, so `Promise.resolve(1)` prints as a promise.
  Runtime errors in those inputs are 9 columns off on their first line.
  `SCRIPT_COMPLETION_VALUES` in `repl/input.rs` was the switch from the interim
  (expressions only) to completion values.
- **Top-level `await`** (D1641). An input that is not a valid Script but contains `await`
  is rewritten into an async body whose declarations are globals:
  - `let`/`const`/`var` → `0,` padded to the keyword's width: `let {a} = await f()`
    becomes the destructuring assignment `0,  {a} = await f()`. The names are declared
    first by a Script (`let a;`, `var` for `var`), so a redeclaration fails before the
    input runs, and closures in the body see the globals.
  - `class C {}` → `C = class C {};`, declared with `let C`.
  - `function f() {}` stays (hoisted in the body), followed by `;globalThis.f = f;`.
  - In TypeScript mode the types are erased first (`strip_types`); inputs with enums or
  namespaces are rewritten as they are.

  Gaps: `const` becomes `let` (assignable later). A statement with `await` prints no
  value (an async body has no completion value). A name declared before the input throws stays declared (`undefined`). The scan
  that finds top-level declarations is token based (a regex is told from a division by
  the previous token), so exotic ASI cases can mislead it.
- **Ctrl-C** while an input runs stops it at the end of the current fuel slice (1M fuel,
  a few ms): `vm.abort()`, then `Interrupted`. The globals stay.
- **History** goes to `$DJS_HISTORY`, else `~/.djs_history` (the last 1000 lines, one per
  physical line). `--no-history` turns it off.

### Output

Results print with a host-side printer (`repl/inspect.rs`) that reads the heap directly:
no JS runs, so getters show as `[Getter]` and Proxies as `Proxy [ target, handler ]`
without calling traps. The style follows node's `util.inspect` (D1100: matching qjs is not
a goal): strings quoted, `-0`, `10n`, `Symbol(x)`, `[ 1, <1 empty item> ]`,
`Foo { x: 1 }`, `[Object: null prototype] {}`, `[Function: f]`, `[class B extends A]`,
`[AsyncGeneratorFunction: g]`, `Map(1) { k => v }`, `Set`, `Uint8Array(2) [ 1, 2 ]`,
`ArrayBuffer { [Uint8Contents]: <..> }`, `Promise { <pending> }` / `{ <rejected> .. }`,
dates in ISO form (UTC), `/re/g`, boxed primitives, cycles as `<ref *1>` and
`[Circular *1]`. Nesting deeper than 3 prints `[Object]`; more than 100 elements print
`... n more items`; a value wider than 72 columns breaks into one entry per line. An
uncaught error prints as `Uncaught Name: message` with its stack. `console.log` (and
`info`/`warn`/`error`/`debug`) prints strings raw and everything else the same way.

### Line mode and tests

When stdin is not a terminal, the REPL reads lines with no prompt, banner or editing,
and prints only results. `crates/djs-cli/tests/repl.rs` drives it that way.

### The line editor

Hand-rolled, without dependencies (D1640). The terminal is put in non-canonical mode
without echo or signal keys through `stty`, so Ctrl-C arrives as a key, and a thread reads
stdin into a channel that the REPL checks between fuel slices. Keys: Left/Right,
Home/End, Up/Down (history), Backspace/Delete, Ctrl-A/E/B/F/K/U/W/L/P/N, Alt-B/F.
Characters count as one column each. Long lines that wrap are redrawn imperfectly.

**Tab** completes the name before the cursor (0102, D2011): a global (Script `let`/
`const`/`class` bindings and the global object's properties along its prototype chain) or,
after a dotted path such as `foo.bar.`, a property of the object the path names (own and
inherited string keys, non-enumerable ones too, as node does). The path is followed
through data properties only, read from the heap: no getter, proxy trap or other JS runs,
so a path through an accessor or a proxy completes nothing. One candidate is inserted; with
several, their common prefix, or, if that adds nothing, the list is printed under the
line. `.complete <text>` shows the same list. Without `stty` (not a Unix terminal) the REPL falls back to line
mode.
