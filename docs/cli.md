# djs-cli: the `djs` command

```sh
cargo run -p djs-cli -- run [--body] [--ts] [--gc-stress] [--suspend-every N] [--tools answers.json] [--bind NAME=IDENTITY]
    [--snapshot-after K OUT] [--snapshot-every-idle] [--debugger OUT] file.js
cargo run -p djs-cli -- resume [--tools answers.json] [--gc-stress] [--suspend-every N]
    [--debugger OUT] file.snapshot
cargo run -p djs-cli -- describe file.snapshot
cargo run -p djs-cli -- disasm [--body] file.js
cargo run -p djs-cli -- strip-types file.ts
cargo run -p djs-cli -- repl [--ts] [--gc-stress] [--no-history] [--frozen] [--debugger] [--tools answers.json]
```

`run` runs a Script (or, with `--body`, a codemode async body) and serves `console.log`.
`disasm` prints the bytecode. `strip-types` erases TypeScript types like ts-blank-space
(TESTING.md). This file documents the REPL (with its heap report, `.heap`) and tools.

`run` defines three host globals (D2602): `tools` (also passed as `tools` to a
`--body`), whose calls are async host calls (below); `later(v, rejected)`, an async host
call answered with `v` (rejected with it if `rejected`), the corpus's idle point; and
`print(...)`, which prints like `console.log`. Whenever the run is idle (no jobs, only
host calls open), the oldest open call that has an answer gets it, then the run
continues: `later` calls always, tool calls from `--tools answers.json`. A call with no
answer ends the run with `djs: no answer for #4 tools.read({"path":"x"})` on stderr and
exit code 1.

A host function is named by its identity (D2970), its full name: `tools.read` for a tool,
`later`, `print`. A `--body` gets the CLI's default bindings `tools` and `console` as its
parameters; `--bind NAME=IDENTITY` adds a parameter `NAME` holding the async host
function `IDENTITY` (`--bind myTool=@sdk/our-stuff#myTool`, a stand-in for an import,
D2972), answered like a tool from the answers file's entry for `IDENTITY`. A snapshot
names the host values it holds by identity (D3720).

## Snapshots (D3700, D3720)

The CLI writes a waiting run as a logical snapshot of its host (`src/snapshots.rs`): its
host values are named, not written (the globals `tools` and `console`, and the host
functions `later`, `print`, `debugger`, the tool maker and every tool by identity), and
the host's data, JSON, carries the tools the run made, how many ordered answers each tool
has taken from `--tools`, and the REPL's call origins. Loading takes them back, so a
resumed run gets the answers, and its calls the ids, that the uninterrupted run gets.
The snapshot stores the script's text. Snapshots refuse a run whose builtins were written
(D3290); `--frozen` in the REPL makes such a write fail at once instead.

Idle points are numbered from 1 (D2604): every stop with only host calls open, before the
run answers one. `run --snapshot-after K OUT file.js` writes the run as a snapshot to
`OUT` at idle point K and stops (exit code 0, with what the run printed so far); `djs
resume OUT` (with the same `--tools`) prints the rest, so the two together print what the
uninterrupted run prints. A run that ends first says so on stderr; a snapshot the writer
refuses is exit code 1 with the reasons. `run --snapshot-every-idle` snapshots at every
idle point and continues in a fresh VM that loaded the snapshot.

`resume file.snapshot` loads a snapshot into a fresh VM (with frozen builtins if the run
had them) and goes on to its end like `run`: the open calls keep their ids and are
answered the same way. A file that is not a snapshot, or one that does not load (a
waiting function whose bytecode differs in this build, a host value the CLI does not
have), is exit code 2 with `djs: cannot load FILE: ...`. `describe file.snapshot` prints
`snapshot::describe`: its sections and their sizes, the builtins, scripts, host values
and host data, the functions (those with frames marked `bytecode`) and its records by
kind.

### `debugger;` pauses (`--debugger OUT`)

With `--debugger OUT` (D3060), `debugger;` in an async function, or at the top level of a
`--body`, is a pause: it awaits a call of the host function `debugger` (D3050). At the
first pause the run is written as a snapshot to `OUT` and stops, exit code 0, with what
it printed so far on stdout and where it paused on stderr. `djs resume OUT` goes on to
the end; `djs resume --debugger OUT OUT` goes on to the next pause and writes it to `OUT`
again, so a run can be stepped pause by pause. The snapshot holds the pause as an open
call, which a resumed run answers at once. Without the flag `debugger;` does nothing.
Pauses are idle points, so `--snapshot-after` counts them.

```text
$ djs run --debugger p.snapshot loop.js
step 1 1
djs: paused at loop.js:6:5, wrote p.snapshot
$ djs resume --debugger p.snapshot p.snapshot
step 2 3
djs: paused at loop.js:6:5, wrote p.snapshot
$ djs resume p.snapshot
step 3 6
total 6
```

A resumed run is the same script, so its positions are the script's lines.

## The heap report (`.heap`)

The REPL's `.heap [sections]` shows what is in the live VM's heap (D2264, D2340): its image
read through djs-core's heap code (verified, walked with the layout table), compared with
the base image every VM starts from. `.heap kinds,largest` picks sections, `.heap all`
shows every one; the default is `summary,diff`.

| section | shows |
| --- | --- |
| `summary` | image size, live vs garbage (and what a collection leaves), old region (`OLD_END`) vs young, the VM's own part (past `BASE_END`) and how many builtin prefix words it changed |
| `kinds` | cells and bytes per cell kind (garbage, old) and per object class |
| `prefix` | the words of the builtin prefix that differ from this build's base, by cell and builtin path (`%Array.prototype%`): the builtins a script changed (D2066) |
| `diff` | against the base image: unchanged cells in place or moved, changed, new (grouped into subtrees with their path and the new bytes only they keep alive), gone; how much of the image sits at the base's offsets. A changed cell counts its differing words; a regrown one compares over the words both have, plus its nonzero new words |
| `largest` | the largest cells and the largest retained sizes (dominator tree), with paths like `globalThis.cache[3].body`; in the session view, retained sizes count the session's cells only |
| `strings` | flat strings and ropes, duplicated contents, the largest strings |
| `shapes` | shapes, objects with a shape or in dictionary mode, the most common key lists |
| `functions` | templates and bytecode per script, closures, builtin closures by intrinsic, boxes |
| `session` | run state, queued jobs, the inbox, pending host calls, promises, suspended async functions and generators with their locals |

`largest`, `strings`, `shapes` and `functions` describe the session's own part: the cells
new or changed against the base (D2432). Each starts with one line of the base's totals
(`unchanged from the reference: ...`). A string the base already holds counts whole as an
extra copy.

Builtins are named by their registry paths (D2371, tests/builtins/registry.tsv):
`%Array.prototype.map%`, `%get Map.prototype.size%`, `%ArrayIteratorPrototype%.next`;
`globalThis` is the global object. Paths read like JavaScript where the heap allows: `.name` for properties (from the
object's shape or table), `[i]` for elements, `~name` for captured variables, `<frame>.x`
for a suspended function's locals, `<proto>`, `handle[n]` for the embedder's handles,
and the layout's field names in brackets for internal cells (`<atoms><entries>`).

## `djs repl`

An interactive REPL for debugging the engine and scripts (issue 0064), built only on the
public `Vm` API. djs's own features are commands: fuel, suspension, the heap report, bytecode
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
| `.disasm <code>` | if `code` is an expression that evaluates to a function (a bound one is followed to its target), that function's bytecode with its nested functions. Otherwise the bytecode of `code` itself, compiled and not run. The expression *is* evaluated |
| `.fuel [n\|off]` | without an argument: the fuel the last input used, and the limit. With `n`: each input may use `n` fuel, then it is stopped (`vm.abort()`, the globals stay) |
| `.suspend-every [n\|off]` | run inputs in slices of `n` fuel. At every stop, print the fuel so far and the innermost frame (`Vm::backtrace`), then continue in a new VM restored from the heap image, as `djs run --suspend-every` does |
| `.step [n\|off]` | pause a running input every `n` fuel (default 1000) at a nested `step>` prompt that shows the innermost frame: Enter (or `s`/`n`) runs the next slice, `c` runs to the end, `bt` prints all frames (`Vm::backtrace`), `fuel` the fuel so far, `q` (or Ctrl-C) stops the input (`vm.abort()`, the globals stay). In line mode the prompt reads the next input lines |
| `.complete <text>` | the Tab completions for `text` (what Tab offers in the editor) |
| `.time [on\|off]` | after each input: wall time (compile and run) and fuel |
| `.gc` | collect; the arena size before and after |
| `.heap [sections]` | arena bytes used, allocated and the limit, handles, atoms; then the heap report's sections over the live VM against the base image (above; default `summary,diff`: the session's own cells and the base cells it changed, apart from the base realm every VM starts with; `all`, or a list such as `kinds,largest`) (D2340) |
| `.snapshot [file]` | write the session as a snapshot (default `session.snapshot`, D3720): its globals and everything in flight (open calls, suspended activations, waiting inputs as handles by label), the CLI's host values by name; prints the size and what is in flight. What a snapshot refuses is reported with the reasons |
| `.restore <file>` | replace the VM with a fresh one that loaded a snapshot (with frozen builtins if the session had them); its waiting inputs and open calls come back (below). A file that is not a snapshot is refused |
| `.clear` | a fresh VM |
| `.pending` | the open tool calls: `#2 tools.read({"path":"a.ts"})  input 1, at repl:1:11`, with the input (or `.resolve`/answer) that made the call and where the innermost activation waiting on it stopped |
| `.resolve <id> <json>` | answers call `#id` with a JSON value (`null` without one), then runs what that unblocks |
| `.reject <id> <message>` | answers call `#id` with `new Error(message)` |
| `.continue [id]`, `.c [id]` | continue a `debugger;` pause (the oldest, or `#id`) in place (below) |
| `.abort` | drop the paused input and everything else in flight (below) |
| `.bt [id\|all]` | the suspended activations waiting on call `#id`, innermost first, through other activations, `then` reactions and `Promise.all`-style combinators; or, with `all` or no argument, every suspended activation (after a collection). Each with where it stopped, the source line with a caret under the `await`/`yield`, and its named bindings: parameters (named from the function's source) and locals |

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
`.clear` forgets the waiting inputs. `.snapshot FILE` of a waiting session writes them
as handles under their labels (`input 2`), and `.restore FILE` brings them back as
waiting inputs, with the open calls under their ids (D2604):

```text
> let out = []; tools.read({ path: 'a' }).then(v => out.push(v)); 0
[waiting] #2 tools.read({"path":"a"})
0
> .snapshot s.snapshot
wrote 2535 bytes to s.snapshot (1 open call, 0 waiting inputs)
> .restore s.snapshot
loaded s.snapshot into a fresh VM
waiting: #2 tools.read({"path":"a"})
```

Inputs suspended in their own top-level `await` snapshot too. Inputs typed after a
restore are numbered after the restored ones.

### `debugger;` pauses

In `djs repl --debugger` (D3731), `debugger;` pauses (D3060): in an async
function, or anywhere in an input that has one outside the functions it defines (such an
input runs as a top-level-await input, with the same gaps). The pause is an open call of
the host function `debugger`, so the paused input waits as on a tool call and the prompt
comes back; the REPL prints where it paused, with the line, and the first time how to go
on:

```text
> let n = 0
> for (let i = 0; i < 3; i++) { n += i; debugger; }
[paused] #1 input 2 at repl:1:39
  for (let i = 0; i < 3; i++) { n += i; debugger; }
                                        ^
(.snapshot saves it, .c continues, .abort drops it)
> .bt 1
#1 debugger()
  async <eval> at repl:1:39 (input 2)
      for (let i = 0; i < 3; i++) { n += i; debugger; }
                                            ^
    i = 0
> .c
[paused] #2 input 2 at repl:1:39
...
```

While paused, inputs run as usual (`n` prints the global), `.pending` lists the pause
and `.bt` shows the paused activation's bindings. `.continue` (`.c`) answers the
pause in place and the input runs on to its end or its next pause; `.c 3` picks the
pause `#3` when several inputs are paused. To continue in a fresh VM instead,
`.snapshot p.snapshot`, `.restore p.snapshot` (the pause is open again, at the same
place of the input) and `.c`; a VM restored from a snapshot always has the `debugger`
function, so the input pauses again at its next `debugger;`. `.abort`, or Ctrl-C at an empty prompt while something is paused, drops
the paused input with everything else in flight (`vm.abort()`, as Ctrl-C does to a
running input: other waiting inputs and their open calls go too, and the message counts
them); the globals stay. Without `--debugger`, `debugger;` does nothing. `--frozen`
only makes the VMs' builtins frozen (D2304).

**The answers file** (D2601) maps functions by identity (`tools.read`, or a `--bind`
identity) to lists of answers, each `{"value": v}` or `{"error": "message"}`, optionally
with `"args": [...]`:

```json
{
  "tools.glob": [{ "value": ["src/a.ts", "src/b.ts"] }],
  "tools.read": [
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
and prints only results. `crates/djs-cli/tests/it/repl.rs` drives it that way.

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
