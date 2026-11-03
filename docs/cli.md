# djs-cli: the `djs` command

```sh
cargo run -p djs-cli -- run [--body] [--ts] [--experimental-decorators] [--emit-decorator-metadata]
    [--gc-stress] [--suspend-every N] [--backend bytecode|native] [--tools answers.json] file.js
cargo run -p djs-cli -- disasm [--body] file.js
cargo run -p djs-cli -- heap [--json] [--top N] [--only SECTIONS] [--diff FILE|base|none]
    [--reference FILE] [--all] dump
cargo run -p djs-cli -- strip-types file.ts
cargo run -p djs-cli -- resume [--explain] [--tools answers.json] [--gc-stress] [--suspend-every N] dump.js
cargo run -p djs-cli -- repl [--ts] [--gc-stress] [--no-history] [--tools answers.json]
```

`run` runs a Script (or, with `--body`, a codemode async body) and serves `console.log`;
`--experimental-decorators` and `--emit-decorator-metadata` (which implies it) select
TypeScript's legacy decorators (D1651). `--backend` runs the `@native` builtins as their
generated Rust state machines (`native`, the default) or as bytecode (D1635).
`disasm` prints the bytecode. `strip-types` erases TypeScript types like ts-blank-space
(TESTING.md). `heap` analyzes a heap dump (below). This file documents `heap`, the
REPL and tools.

`run` defines three host globals (D2602): `tools` (also passed as `tools` to a
`--body`), whose calls are async host calls (below); `later(v, rejected)`, an async host
call answered with `v` (rejected with it if `rejected`), the corpus's idle point; and
`print(...)`, which prints like `console.log`. Whenever the run is idle (no jobs, only
host calls open), the oldest open call that has an answer gets it, then the run
continues: `later` calls always, tool calls from `--tools answers.json`. A call with no
answer ends the run with `djs: no answer for #4 tools.read({"path":"x"})` on stderr and
exit code 1.

`resume dump.js` (D2603) loads a dump (format 2, starting with `"djs-dump 2";`, or
version 1, `// djs-dump 1`; `Vm::load_dump`) into a fresh VM with the same globals and
runs it on to its end like `run`: the open calls the dump recreates keep their ids and
are answered the same way. A dump that does not load is exit code 2 with the loader's
reason. The loader's hand-written fixtures (tests/dump/v2/) resume to the rest of their
programs' output.

`resume --explain dump.js` (D2641) only loads it and prints what it holds in flight,
since a dump has no comments: the open calls, each with the activations waiting on it,
then the other suspended activations (a generator at a `yield`, an async function
waiting on another's result), each with where it stopped in the dump, the source line
with a caret under the `await`/`yield`, and its bindings; handles name the activation
whose promise they are. Nothing is answered; exit code 0, or 2 if it does not load.
Loading runs the dump's code, as `resume` does.

```text
$ djs resume --explain reads.dump.js
reads.dump.js: format 2; 1 open call, 1 suspended activation; handles: run
#2 tools.read({"path":"a"})
  async function main at dump.js:3:13 (run)
      const a = await tools.read({ path: "a" });
                ^
    a = <uninitialized>, b = <uninitialized>
```

Idle points are numbered from 1 (D2604): every stop with only host calls open, before the
run answers one. `run --dump-after K OUT file.js` writes the run as a format 2 dump to
`OUT` at idle point K and stops (exit code 0, with what the run printed so far); `djs
resume OUT` (with the same `--tools`) prints the rest, so the two together print what the
uninterrupted run prints. A run that ends first says `djs: --dump-after K: the run ended
after N idle points` on stderr; a dump the exporter refuses is exit code 1 with the
refusal (`cannot dump:` and one `path: reason` line per value). `run --dump-every-idle`
dumps at every idle point and continues in a fresh VM that loaded the dump. An async
body's promise (`--body`) is the dump's handle `run`.

A `--body` script suspended in its own top level dumps as the async function it runs as
(D2617). Known gap: a body whose parameter (`tools`, `console`) a closure captures is
refused ("a codemode body whose frame at await#1 differs from the async function the dump
writes for it"), reported to the exporter.

## `djs heap`

What is in a heap dump (D2264): a raw image (`.snapshot --raw` of the REPL, `Vm::bytes`), a
session image (D2066) or a snapshot container (`Sandbox.bytes()`, pi's `snapshot()`, the
REPL's `.snapshot`, D2260). Sections (`--only summary,kinds,...` picks some; `--json` prints one object with
a key per section):

| section | shows |
| --- | --- |
| `summary` | format, build, image size, live vs garbage (and what a collection leaves), old region (`OLD_END`) vs young, the session image's size (what a container stores before compression, D2066) |
| `kinds` | cells and bytes per cell kind (garbage, old) and per object class |
| `prefix` | the words of the builtin prefix that differ from this build's base, by cell and builtin path (`%Array.prototype%`): the builtins a script changed (D2066) |
| `diff` | against the reference image: unchanged cells in place or moved, changed, new (grouped into subtrees with their path and the new bytes only they keep alive), gone; how much of the image sits at the reference's offsets. This is what a snapshot compressed against the reference costs. A changed cell counts its differing words; a regrown one compares over the words both have, plus its nonzero new words |
| `largest` | the largest cells and the largest retained sizes (dominator tree), with paths like `globalThis.cache[3].body`; in the session view, retained sizes count the session's cells only |
| `strings` | flat strings and ropes, duplicated contents, the largest strings |
| `shapes` | shapes, objects with a shape or in dictionary mode, the most common key lists |
| `functions` | templates and bytecode per script, closures, builtin closures by intrinsic, boxes |
| `session` | run state, queued jobs, the inbox, pending host calls, promises, suspended async functions and generators with their locals |

`largest`, `strings`, `shapes` and `functions` describe the session's own part when there
is a reference: the cells new or changed against it (D2432). Each starts with one line of
the reference's totals (`unchanged from the reference: ...`); `--all` shows the whole
image instead, as does `--diff none`. In the session view, a string the reference already
holds counts whole as an extra copy. In JSON these sections carry `scope` (`session` or
`all`) and, in the session view, `reference_part` with the totals.

The diff compares with a container's reference by default, or with this build's base
for a raw image of this build; `--diff FILE` picks another image, `--diff none` skips it.
A container compressed against the base decompresses with this build's base when the
hashes match. Anything else needs its reference: `node scripts/heap/reference.ts base
[--wasm MODULE] -o base.img` writes a module's base, and `node scripts/heap/reference.ts
pi-prelude --data START.json -o prelude.img` pi's prelude image for a session's start
data (tools, globals, store, memory limit, deterministic mode;
scripts/snapshot-research/wasm.ts writes its cases' as `start.json`). Then `djs heap
--reference prelude.img session.djsn`.

Builtins are named by their registry paths (D2371, tests/builtins/registry.tsv):
`%Array.prototype.map%`, `%get Map.prototype.size%`, `%ArrayIteratorPrototype%.next`;
`globalThis` is the global object. Paths read like JavaScript where the heap allows: `.name` for properties (from the
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
| `.heap [sections]` | arena bytes used, allocated and the limit, handles, atoms; then `djs heap`'s sections over the live VM against the base image (default `summary,diff`: the session's own cells and the base cells it changed, apart from the base realm every VM starts with; `all`, or a list such as `kinds,largest`) (D2340) |
| `.snapshot [--raw] <file>` | collect, then write a snapshot container (app 3, `repl`) compressed against the base image: about 16 KB for a small session instead of its ~330 KB image. `--raw` writes the image itself (`vm.bytes()`) |
| `.dump <file\|-> [names]` | write the session's globals (or the named ones, plus the globals their functions read) as a JavaScript dump (docs/functions-as-code.md): functions as their source text, closures that share variables made by one factory, data as literals and constructor calls, binary data as base64, builtins referenced by name and changed builtins as code. With something in flight it is format 2: open calls, suspended activations and pending promises, with the waiting inputs as handles (below). What a dump cannot write is refused with its path. `-` prints it |
| `.restore <file>` | replace the VM with one restored from a container (any app; decompressed with this build's base) or a raw image. It must come from the same build (the build hash is checked). Images from `djs repl` and `djs run` both work: `console` is host function 1 in both. A file starting with `"djs-dump ` (format 2) or `// djs-dump` (version 1) is a dump instead: a fresh VM loads it (`Vm::load_dump`), from any build; a format 2 dump's waiting run and open calls come back (below) |
| `.dump-every [on\|off]` | at every idle point, dump the session and continue in a fresh VM that loaded the dump, printing `[dumped N bytes]`; a refused dump prints `[not dumped]` with the reasons and turns it off |
| `.clear` | a fresh VM |
| `.pending` | the open tool calls: `#2 tools.read({"path":"a.ts"})  input 1, at repl:1:11`, with the input (or `.resolve`/answer) that made the call and where the innermost activation waiting on it stopped |
| `.resolve <id> <json>` | answers call `#id` with a JSON value (`null` without one), then runs what that unblocks |
| `.reject <id> <message>` | answers call `#id` with `new Error(message)` |
| `.bt <id\|all>` | the suspended activations waiting on call `#id`, innermost first, through other activations, `then` reactions and `Promise.all`-style combinators; or every suspended activation (after a collection). Each with where it stopped, the source line with a caret under the `await`/`yield`, and its named bindings: parameters (named from the function's source) and locals |

### Tools and waiting inputs

The REPL defines a global `tools` (0174, D2600). `tools.<name>(...args)` is an async host
call: it returns a promise at once and the call stays open until it is answered, by hand
with `.resolve`/`.reject`, or from `--tools answers.json`. An input that waits on a call
does not block the prompt: the open calls print as `[waiting] #2 tools.read(...)` (or
`[input 3 waiting]` when it waits on calls made earlier), later inputs run, and when the
input settles its value prints as `[input 1] value` (an expression input; a statement
prints nothing) or `[input 1] Uncaught ...`.

```text
> const r = await tools.read({ path: "a.ts" })
[waiting] #2 tools.read({"path":"a.ts"})
> .pending
#2 tools.read({"path":"a.ts"})  input 1, at repl:1:11
> .resolve 2 "export const x = 1;"
> r.length
19
```

Call ids are the VM's host call ids, which count every host call: the first read of a
tool name makes its function through a host call of its own (`tools` is a Proxy that
creates an async host function per name and keeps it), and `console.log` calls count
too. Answers are applied through the VM's inbox, one macrotask each, so the order of
answers is what the script sees. With `--tools`, answered calls print as
`[answered] #2 tools.read(...)`; `later(v)` calls are answered at once, quietly.
`.clear` and `.restore` of an image forget the waiting inputs (the calls a restored image
still has open are listed). `.restore` of a format 2 dump restores them (D2603): a fresh
VM loads it, its open calls keep their ids (a tool named in the dump gets the id its
function has afterwards), and the values it names for the host (`$dump.handle("run",
p)`) become waiting inputs labelled by name, printed as `[run] value` when they settle:

```text
> .restore reads.dump.js
loaded reads.dump.js into a fresh VM
waiting: #2 tools.read({"path":"a"})
> .bt 2
#2 tools.read({"path":"a"})
  async function main at dump.js:3:13 (run)
      const a = await tools.read({ path: "a" });
                ^
    a = <uninitialized>, b = <uninitialized>
> .resolve 2 "AA"
[waiting] #4 tools.read({"path":"b"})
```

`.dump FILE` of a waiting session writes format 2 (D2604): the waiting inputs are its
handles, under their labels (`input 2`, or a restored dump's names), so `.restore` brings
them back as waiting inputs, and it reports what is in flight:

```text
> let out = []; get('a').then(v => out.push(v)); 0
[waiting] #2 tools.read({"path":"a"})
0
> .dump s.js
wrote 596 bytes to s.js (1 activation, 1 pending call)
```

`.dump-every on` does this at every idle point and continues in the loaded copy, as
`.suspend-every` does with images. Inputs suspended in their own top-level `await`
dump too. A waiting expression input is the handle `input N (value)` and a statement
input `input N`, so a restore prints them as before; inputs typed after a restore are
numbered after the restored ones. While an input's declarations run (the first part of a
top-level-await input), `.dump-every` skips that idle point.

**The answers file** (D2601) maps tool names to lists of answers, each `{"value": v}` or
`{"error": "message"}`, optionally with `"args": [...]`:

```json
{
  "glob": [{ "value": ["src/a.ts", "src/b.ts"] }],
  "read": [
    { "args": [{ "path": "src/a.ts" }], "value": "// TODO" },
    { "value": "" },
    { "error": "ENOENT" }
  ]
}
```

A call gets the first entry whose `args` equal its arguments as JSON (object keys in any
order); such an entry answers every matching call. Otherwise it gets the next entry
without `args`, in order, and the last one repeats. Entries with `args` give the same
answers however the calls are ordered or split across runs.

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
