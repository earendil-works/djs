# The command line and the REPL

The `djs` command runs scripts natively, outside any wasm host: to try code, to debug a
script with its tool calls answered from a file, to look at bytecode, and to write,
inspect and resume snapshots of waiting runs. Its REPL is a workbench for the engine
itself, with commands for fuel, suspension, the heap and snapshots. This page covers
both.

> **In short.** `djs run` runs a file, as a Script or as a codemode body, with tools
> answered from an answers file; `djs resume` continues a snapshot; `djs disasm` and `djs
> strip-types` show what the compiler makes of a file. `djs repl` runs inputs as
> Scripts and prints their values, and its dot-commands expose fuel, stepping, the heap
> report, open tool calls, `debugger;` pauses and snapshots.

The command is built from the `djs-cli` crate: `cargo run -p djs-cli -- run file.js`, or
`cargo build --release -p djs-cli` for a `djs` binary.

## `djs run`

```text
djs run [--body] [--ts] [--gc-stress] [--suspend-every N] [--tools answers.json] [--bind NAME=IDENTITY]
        [--snapshot-after K OUT] [--snapshot-every-idle] [--debugger OUT] file.js
```

`djs run file.js` runs the file as a Script: top-level declarations are globals, and
there is no top-level `await`. With `--body` it runs as a codemode body, the way hosts
run scripts: the body of an async function, with `await` and `return` at the top level
and the parameters `tools` and `console`. `.ts` files are TypeScript; `--ts` makes any
file TypeScript ([TypeScript](typescript.md)).

The run has three host globals besides `console`:

- `tools`: every `tools.name(...)` is an async host call, answered from `--tools`;
- `later(v, rejected)`: an async host call answered with `v` (or rejected with it), a
  way to make a run wait without a tools file;
- `print(...)`: prints like `console.log`, as a sync host call.

Whenever the run is idle, with only host calls open, the oldest open call that has an
answer gets it, and the run continues. A call with no answer ends the run with
`djs: no answer for #4 tools.read({"path":"x"})` on stderr. A host function is named by
its full name, its identity: `tools.read` for a tool, `later`, `print`. `--bind
myTool=@sdk/x#myTool` gives a `--body` one more parameter, `myTool`, the async host
function of that identity, answered like a tool from the answers file's entry for it, a
stand-in for an import a host would provide.

Exit codes: 0 when the run succeeds; 1 for an uncaught exception (printed as
`Uncaught TypeError: ...`) or a call without an answer; 2 for a usage or compile error.

The remaining flags exercise the engine: `--gc-stress` collects garbage at every
safepoint, `--suspend-every N` stops the run every N fuel and goes on. A script's output
must not change under either of them.

### The answers file

`--tools answers.json` maps host function names to lists of answers, each
`{"value": v}` or `{"error": "message"}`, optionally with the `"args"` it answers:

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

A call gets the first entry whose `args` equal its arguments (compared as JSON, object
keys in any order), and such an entry answers every matching call. Otherwise it gets the
next entry without `args`, in order, and the last one repeats. Entries with `args` give
the same answers however the calls are ordered or split across runs.

## Snapshots on the command line

Idle points are numbered from 1: every stop where the run has only host calls open,
before an answer is given. At any of them the CLI can write the run as a
[snapshot](snapshots.md): its state as data, the CLI's own `tools`, `console` and
host functions named, not written.

- `djs run --snapshot-after K OUT file.js` writes the run as a snapshot to `OUT` at idle
  point K and stops. `djs resume OUT`, with the same `--tools`, prints the rest, so the
  two together print what the uninterrupted run prints. A run that cannot be snapshotted
  is exit code 1, with the reason: one whose script wrote a builtin, for instance
  (`--frozen` in the REPL makes such a write fail at once instead). A run that ends
  before idle point K says so on stderr.
- `djs run --snapshot-every-idle file.js` snapshots at every idle point and continues in
  a fresh VM that loaded the snapshot: the run's output must be the same as without it.
- `djs resume file.snapshot` loads a snapshot into a fresh VM, with frozen builtins if
  the run had them, and goes on to the end, like `run`. Its open calls keep their ids and
  are answered the same way; the tools the run made and the answers it used travel with
  it, so a resumed run gets the answers the uninterrupted run gets. `resume` takes the
  flags of `run`. A file that is not a snapshot, or one that does not load (a waiting
  function whose bytecode differs in this build, a host value the CLI does not have), is
  exit code 2 with `djs: cannot load FILE: ...`.
- `djs describe file.snapshot` says what a snapshot holds, readable first: each open
  call and the activations waiting on it, with their source lines and their variables as
  JavaScript values, up to the run's result; then what a loading djs must have (the code
  of the stopped functions, builtins, host values), the size in words, and every record
  as djs stores it. `djs describe --json file.snapshot` writes all of it as JSON, for
  tools ([describing a snapshot](snapshots.md#describing-a-snapshot)).

### `debugger;` pauses

With `--debugger OUT`, a `debugger;` statement in an async function, or at the top level
of a `--body`, is a pause: the run waits there as on a tool call. At the first pause the
run is written as a snapshot to `OUT` and stops, with exit code 0, what it printed so far
on stdout, and where it paused on stderr. `djs resume OUT` goes on to the end; `djs
resume --debugger OUT OUT` goes on to the next pause and writes it to `OUT` again, so a
run can be stepped pause by pause:

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

A resumed run is in the same script, so its pauses are at the script's lines. Without
`--debugger`, `debugger;` does nothing.

A snapshot from a live example of this site works the same way: the examples' runs have
the host values `console` and `debugger`, which the CLI gives too. Copy one as base64 at
a pause, decode it with `base64 -d > pause.snapshot`, and `djs describe` and `djs resume
[--debugger]` take it from there ([the same steps as host code](snapshots.md#the-same-steps-as-host-code)).
The other way round, a snapshot of `djs run` names the CLI's `tools`, `print` and
`later`, which the site does not have, and its restore box says so.

## `djs disasm` and `djs strip-types`

`djs disasm [--body] file.js` prints the bytecode of a file and of every function in it,
as the [compiler](compiler.md) emits it, with source positions and the names of locals.
`djs strip-types file.ts` prints a TypeScript file with its types erased by blanks, the
way the compiler sees it ([TypeScript](typescript.md#djs-strip-types)).

## The REPL

```text
djs repl [--ts] [--frozen] [--debugger] [--tools answers.json] [--gc-stress] [--no-history]
```

The REPL runs each input as a Script, so declarations persist from one input to the
next, and prints its value:

```text
> let o = {a: [1, 2], m: new Map([[1, 'x']])}
> o
{ a: [ 1, 2 ], m: Map(1) { 1 => 'x' } }
> await Promise.resolve(42)
42
```

**Inputs.** A line that leaves the input incomplete (the compile error is at its end, or
a template or comment is open) prompts `...` for more; Ctrl-C drops it. Each input is a
Script: top-level `let`, `const` and `class` live in the global lexical scope and `var`
and functions on the global object, so later inputs see them, and declaring a `let`
again is a SyntaxError, as between two Scripts. An input prints its completion value,
except the `undefined` of one that starts with a declaration. Two kinds of input run as
the async body `return [(input)];` instead: an expression a Script cannot run (`await
f()`), and one that starts with `{`, which is an object literal as in node (a block that
is not an expression still runs as a Script). The array keeps the body from adopting a
promise, so `Promise.resolve(1)` prints as a promise; a runtime error on the first line
of such an input reports a column 9 too far. `--ts` (or `.ts on`) parses inputs as
TypeScript.

**Top-level `await`.** An input that is not a valid Script but contains `await` runs as
an async body whose declarations become globals: `let`, `const` and `var` become
assignments to names a Script declared first (so a redeclaration fails before the input
runs), `class C {}` becomes `C = class C {};`, and a function declaration is followed by
`globalThis.f = f`. In TypeScript the types are erased first. The rewrite has gaps: a
`const` is assignable afterwards, a statement with `await` prints no value, a name
declared by an input that throws stays declared as `undefined`, and the scan for
declarations works on tokens, so exotic semicolon insertion can mislead it.

**Output.** Values print in the style of node's `util.inspect`, read straight from the
heap: no code of the script runs to print them, so getters show as `[Getter]` and
proxies as their target and handler. Nesting deeper than 3 prints `[Object]`, more than
100 elements print `... n more items`, and a value wider than 72 columns breaks into one
entry per line. An uncaught error prints as `Uncaught Name: message` with its stack.
`console.log` prints strings raw and everything else the same way.

**Editing.** Left and Right, Home and End, Up and Down for history, the usual Ctrl and
Alt keys, and Tab, which completes globals and properties after a dotted path (`.complete
text` shows the candidates). Completion follows data properties only, read from the
heap: no getter or proxy trap runs, so a path through one completes nothing. History is
kept in `$DJS_HISTORY`, else `~/.djs_history` (the last 1000 lines); `--no-history` turns
it off. Ctrl-C stops a running input at the end of its fuel slice, a few milliseconds,
and the globals stay. When stdin is not a terminal, the REPL reads lines with no prompt
and prints only results; the CLI's tests drive it that way.

> **Trade-off.** A line editor of its own over a library. The terminal is switched to
> raw input with `stty`, and a thread hands keys to the REPL between fuel slices, so
> Ctrl-C arrives as a key while a script runs, and the CLI has no dependencies for it.
> The cost: characters count as one column each, long lines that wrap redraw
> imperfectly, and without `stty` the REPL falls back to line mode.

### Commands

| command | does |
| --- | --- |
| `.help`, `.exit` | the list; leave (Ctrl-D on an empty line too) |
| `.load <file>` | run a file as one input |
| `.ts [on\|off]` | parse inputs as TypeScript |
| `.disasm <code>` | the bytecode of the function `code` evaluates to (a bound function's target), with its nested functions, or of `code` itself, compiled and not run, when it is not such an expression; the expression is evaluated |
| `.fuel [n\|off]` | the fuel the last input used and the limit; with `n`, stop inputs after `n` fuel (the globals stay) |
| `.time [on\|off]` | print wall time and fuel after each input |
| `.suspend-every [n\|off]` | run inputs in slices of `n` fuel and print the fuel so far and the innermost frame at each stop |
| `.step [n\|off]` | pause running inputs every `n` fuel (default 1000) at a `step>` prompt: Enter runs the next slice, `c` to the end, `bt` prints the frames, `fuel` the fuel so far, `q` or Ctrl-C stops |
| `.gc` | collect garbage; the arena's size before and after |
| `.heap [sections]` | the heap report ([below](#the-heap-report)) |
| `.pending` | the open tool calls and `debugger;` pauses, with the input that made each and where it waits |
| `.resolve <id> <json>`, `.reject <id> <message>` | answer call `#id` |
| `.bt [id\|all]` | the suspended functions waiting on call `#id`, or all of them (the default), with the source line and their bindings |
| `.continue [id]`, `.c [id]` | continue a `debugger;` pause in place: the oldest, or `#id` |
| `.abort` | drop the paused input and everything else in flight; the globals stay |
| `.snapshot [file]` | write the session as a snapshot (default `session.snapshot`) |
| `.restore <file>` | replace the VM with a fresh one that loaded a snapshot |
| `.clear` | a fresh VM |

### Tools and waiting inputs

The REPL has a global `tools`: `tools.name(...)` is an async host call that stays open
until it is answered, by hand with `.resolve` and `.reject`, or from `--tools
answers.json`. An input that waits on a call does not block the prompt: the call prints
as `[waiting]`, later inputs run, and when the input settles, its value prints with its
label.

```text
> const r = await tools.read({ path: "a.ts" })
[waiting] #2 tools.read({"path":"a.ts"})
> .pending
#2 tools.read({"path":"a.ts"})  input 1, at repl:1:11
> .resolve 2 "export const x = 1;"
> r.length
19
```

Call ids count every host call, including the ones that make a tool's function on its
first use and `console.log`. Answers are applied one at a time, in the order they are
given, as a host delivers results.

### Snapshots and `debugger;` in the REPL

`.snapshot s.snapshot` writes the session as a [snapshot](snapshots.md): its
globals and everything in flight (open calls, suspended functions, waiting inputs), and
`.restore s.snapshot` replaces the VM with a fresh one that loaded it, with the open calls
under their ids and the waiting inputs back under their labels:

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

Started with `--debugger`, the REPL makes `debugger;` in an async function, or at an
input's top level, pause that input: it waits as on a tool call, and the REPL says
where.

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
```

While an input is paused, the prompt works as usual: other inputs run, `.pending` lists
the pause, and `.bt` shows the paused code's bindings. `.c` continues the pause in
place, and a loop pauses again at its next iteration. To continue in a fresh VM instead:
`.snapshot p.snapshot`, `.restore p.snapshot` (paused again, at the same place of the
input) and `.c`. `.abort`, or Ctrl-C at an empty prompt while an input is paused, drops
it with everything else in flight and reports what it dropped. Without `--debugger`,
`debugger;` does nothing.

`--frozen` is separate: it makes the VMs' builtins frozen ([frozen
builtins](builtins.md#frozen-builtins)), so that a script that writes one fails there
rather than when the session is snapshotted.

A pause at an input's top level shows the input as the REPL rewrites it for top-level
`await`, and such an input prints no final value.

### The heap report

`.heap` shows what is in the session's heap, read with the engine's own heap code and
compared with the base image every VM starts from. `.heap all` shows every section;
`.heap kinds,largest` picks some; the default is `summary,diff`.

| section | shows |
| --- | --- |
| `summary` | the image's size, live and garbage bytes, the old and young regions, and the session's own part |
| `kinds` | cells and bytes per cell kind and per object class |
| `prefix` | the builtins a script changed, by their names (`%Array.prototype%`) |
| `diff` | against the base image: cells unchanged, moved, changed, new and gone, and the largest new subtrees with their paths |
| `largest` | the largest cells and the largest retained sizes, with paths like `globalThis.cache[3].body` |
| `strings` | strings and ropes, duplicated contents, the largest strings |
| `shapes` | shapes and the most common key lists |
| `functions` | bytecode per script, closures, builtin closures |
| `session` | the run state, queued jobs, open host calls, promises, and suspended functions with their locals |

`largest`, `strings`, `shapes` and `functions` describe the session's own part: the cells
new or changed against the base, each after one line of the base's totals; a string the
base already holds counts as an extra copy. Builtins are named by their paths
(`%Array.prototype.map%`, `%get Map.prototype.size%`, `%ArrayIteratorPrototype%.next`),
and paths read like JavaScript where the heap allows: `.name` for a property, `[i]` for
an element, `~name` for a captured variable, `<frame>.x` for a suspended function's
local, `<proto>`, `handle[n]` for the host's handles, and the layout's field names in
brackets for internal cells (`<atoms><entries>`).

[Images and the base image](images.md#inspecting-a-heap) explains how the report
matches a heap against the base.

## Where to look

- [`crates/djs-cli/src/run.rs`](../../crates/djs-cli/src/run.rs): `run` and `resume`;
  [`crates/djs-cli/src/tools.rs`](../../crates/djs-cli/src/tools.rs): the answers file.
- [`crates/djs-cli/src/repl/mod.rs`](../../crates/djs-cli/src/repl/mod.rs): the REPL and
  its commands; [`crates/djs-cli/src/heap/`](../../crates/djs-cli/src/heap/): the heap
  report.
- [`crates/djs-cli/tests/`](../../crates/djs-cli/tests/): the CLI's tests, which show
  every command at work.
