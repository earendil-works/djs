# djs-cli: the `djs` command

```sh
cargo run -p djs-cli -- run [--body] [--ts] [--experimental-decorators] [--emit-decorator-metadata]
    [--gc-stress] [--suspend-every N] [--backend bytecode|native] [--tools answers.json]
    [--bind NAME=IDENTITY] [--dump-after K OUT] [--dump-every-idle] [--debugger OUT] file.js
cargo run -p djs-cli -- disasm [--body] file.js
cargo run -p djs-cli -- strip-types file.ts
cargo run -p djs-cli -- resume [--explain] [--tools answers.json] [--gc-stress] [--suspend-every N]
    [--debugger OUT] dump.js
cargo run -p djs-cli -- repl [--ts] [--gc-stress] [--no-history] [--frozen] [--tools answers.json]
```

`run` runs a Script (or, with `--body`, a codemode async body) and serves `console.log`;
`--experimental-decorators` and `--emit-decorator-metadata` (which implies it) select
TypeScript's legacy decorators (D1651). `--backend` runs the `@native` builtins as their
generated Rust state machines (`native`, the default) or as bytecode (D1635).
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
D2972), answered like a tool from the answers file's entry for `IDENTITY`. A dump names
the host values it holds by identity, `$djs.host("tools.read")` (D2971).

`resume dump.js` (D2603) runs a dump (`"djs-dump";`, D2830, D2835) in a fresh VM with the
same globals and goes on to its end like `run`: the open calls the dump recreates keep
their ids and are answered the same way. A dump is a Script: `resume` runs it with the
global object `$djs` (D2890), the dump runtime's helpers (D2903) plus the hooks:
`seed(lo, hi, next)` sets `Math.random`'s state and the next call's id (D2897),
`resume(id, name, args, state)` opens the dump's call `id` of `name` (`later`,
`tools.read`, ...) again under that id, and the CLI then answers it like any call;
`source(name, hash, params)` compiles the body file `name` again (D2882), and
`host(name)` returns the host value `name` (D2971): the tool of that identity from the
`tools` table (the same function the run's `tools.read` reads), else the global of that
path (`tools`, `console`, `print`, `later`). Once the
Script stops at an idle point or ends, the CLI deletes `$djs` and the dump's `$dump`
(`Vm::end_dump_load`, D2916). A file that is not a dump, or a dump whose Script throws (a hook
refused an unknown name, say), is exit code 2 with `djs: cannot load ...` or `djs: the
dump did not load: Uncaught ...`. `--explain`, and the REPL's `.restore`, load dumps the
same way; `--explain` lists each call's state.

Dumps need frozen builtins (D2836): `run` with `--dump-after` or `--dump-every-idle`, and
every `resume` (the run goes on as the dumped one, which had them), make a VM with frozen
builtins.

The dumps the CLI writes give every open call a state (D2894, D2897): the REPL input
that made it (`origin`), how many ordered answers each tool has taken from `--tools`
(`answered`), and the tools made so far (`made`). Resuming takes them back, so a resumed
run gets the answers, and its calls the ids, that the uninterrupted run gets.

`resume --explain dump.js` (D2641) only loads it and prints what it holds in flight,
since a dump has no comments: the open calls, each with the activations waiting on it,
then the other suspended activations (a generator at a `yield`, an async function
waiting on another's result), each with where it stopped in the dump, the source line
with a caret under the `await`/`yield`, and its bindings; handles name the activation
whose promise they are. Positions are in the dump's own file, in the continuations it
holds (a suspended activation is written as a function that runs the rest of the
original from where it stopped). Nothing is answered; a dump that names a host function
the CLI does not define does not load (its code calls the function while it loads).
Exit code 0, or 2 if it does not load. Loading runs the dump's code, as `resume` does. The REPL's `.pending` and `.bt` after `.restore
FILE` also show positions in `FILE` (until `.dump-every` reloads the VM).

```text
$ djs resume --explain reads.dump.js
reads.dump.js: 1 open call, 1 suspended activation; handles: run
#2 tools.read({"path":"a"}), state {"at":"a"}
  async function <anonymous> at reads.dump.js:13:12 (run)
      const a = (await $dump.p2);
                 ^
    a = undefined, b = undefined
```

Idle points are numbered from 1 (D2604): every stop with only host calls open, before the
run answers one. `run --dump-after K OUT file.js` writes the run as a dump to
`OUT` at idle point K and stops (exit code 0, with what the run printed so far); `djs
resume OUT` (with the same `--tools`) prints the rest, so the two together print what the
uninterrupted run prints. A run that ends first says `djs: --dump-after K: the run ended
after N idle points` on stderr; a dump the exporter refuses is exit code 1 with the
refusal (`cannot dump:` and one `path: reason` line per value). `run --dump-every-idle`
dumps at every idle point and continues in a fresh VM that ran the dump. An async
body's promise (`--body`) is the dump's handle `run`, its Script's completion value.

### `debugger;` pauses (`--debugger OUT`)

With `--debugger OUT` (D3060), `debugger;` in an async function, or at the top level of a
`--body`, is a pause: it awaits a call of the host function `debugger` (D3050). At the
first pause the run is written as a dump to `OUT` and stops, exit code 0, with what it
printed so far on stdout and where it paused on stderr. `djs resume OUT` goes on to the
end; `djs resume --debugger OUT OUT` goes on to the next pause and writes it to `OUT`
again, so a run can be stepped pause by pause. The dump holds the pause as an open call
(`await $djs.resume(4, "debugger", [])`), which a resumed run answers at once. Without the
flag `debugger;` does nothing. Pauses are idle points, so `--dump-after` counts them.

```text
$ djs run --debugger p.js loop.js
step 1 1
djs: paused at loop.js:6:5, wrote p.js
$ djs resume --debugger p.js p.js
step 2 3
djs: paused at p.js:17:5, wrote p.js
$ djs resume p.js
step 3 6
total 6
```

The positions of a resumed run are in the dump (the continuation of the loop it holds).

A `--body` script suspended in its own top level dumps as the async function it runs as
(D2617). Known gap: a body whose parameter (`tools`, `console`) a closure captures is
refused ("a codemode body whose frame at await#1 differs from the async function the dump
writes for it"), reported to the exporter.

## The heap report (`.heap`)

The REPL's `.heap [sections]` shows what is in the live VM's heap (D2264, D2340): its image
read through djs-core's heap code (verified, walked with the layout table), compared with
the base image every VM starts from. `.heap kinds,largest` picks sections, `.heap all`
shows every one; the default is `summary,diff`. (The `djs heap` command, which read image
files, and the `.image` command that wrote them are gone, D2269.)

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
| `.decorators [tc39\|legacy\|metadata]` | TypeScript decorators: the standard ones (the default), TypeScript's `experimentalDecorators`, or those with `emitDecoratorMetadata` (D1651). Legacy decorators need TypeScript mode |
| `.disasm <code>` | if `code` is an expression that evaluates to a function (a bound one is followed to its target), that function's bytecode with its nested functions. Otherwise the bytecode of `code` itself, compiled and not run. The expression *is* evaluated |
| `.fuel [n\|off]` | without an argument: the fuel the last input used, and the limit. With `n`: each input may use `n` fuel, then it is stopped (`vm.abort()`, the globals stay) |
| `.suspend-every [n\|off]` | run inputs in slices of `n` fuel. At every stop, print the fuel so far and the innermost frame (`Vm::backtrace`), then continue in a new VM restored from the heap image, as `djs run --suspend-every` does |
| `.step [n\|off]` | pause a running input every `n` fuel (default 1000) at a nested `step>` prompt that shows the innermost frame: Enter (or `s`/`n`) runs the next slice, `c` runs to the end, `bt` prints all frames (`Vm::backtrace`), `fuel` the fuel so far, `q` (or Ctrl-C) stops the input (`vm.abort()`, the globals stay). In line mode the prompt reads the next input lines |
| `.complete <text>` | the Tab completions for `text` (what Tab offers in the editor) |
| `.time [on\|off]` | after each input: wall time (compile and run) and fuel |
| `.gc` | collect; the arena size before and after |
| `.heap [sections]` | arena bytes used, allocated and the limit, handles, atoms; then the heap report's sections over the live VM against the base image (above; default `summary,diff`: the session's own cells and the base cells it changed, apart from the base realm every VM starts with; `all`, or a list such as `kinds,largest`) (D2340) |
| `.dump <file\|-> [names]` | write the session's globals (or the named ones, plus the globals their functions read) as a JavaScript dump (docs/functions-as-code.md): functions as their source text, closures that share variables made by one factory, data as literals and constructor calls, binary data as base64, builtins referenced by name. With something in flight it also holds the open calls, the suspended activations (as continuations) and pending promises, with the waiting inputs as handles (below). Needs `djs repl --frozen` (D2836). What a dump cannot write is refused with its path. `-` prints it |
| `.restore <file>` | replace the VM with a fresh one that ran a dump (a file starting with `"djs-dump";`, run as a Script), from any build; its waiting run and open calls come back (below): a dump's run becomes the waiting input `run`, several waiting inputs are the properties of the dump's completion value (D2895). Images are not restored (D2700) |
| `.dump-every [on\|off]` | at every idle point, dump the session and continue in a fresh VM that ran the dump (needs `--frozen`), printing `[dumped N bytes]`; a refused dump prints `[not dumped]` with the reasons and turns it off |
| `.clear` | a fresh VM |
| `.pending` | the open tool calls: `#2 tools.read({"path":"a.ts"})  input 1, at repl:1:11`, with the input (or `.resolve`/answer) that made the call and where the innermost activation waiting on it stopped |
| `.resolve <id> <json>` | answers call `#id` with a JSON value (`null` without one), then runs what that unblocks |
| `.reject <id> <message>` | answers call `#id` with `new Error(message)` |
| `.continue [id]`, `.c [id]` | continue a `debugger;` pause (the oldest, or `#id`) in place (below) |
| `.abort` | drop the paused input and everything else in flight (below) |
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
still has open are listed). `.restore` of a dump restores them (D2603): a fresh VM runs
it, its open calls keep their ids (a tool named in the dump gets the id its function has
afterwards), and the values it names for the host (its Script's completion value: the
run's promise as `run`, or an object of several by name, D2895) become waiting inputs
labelled by name, printed as `[run] value` when they settle:

```text
> .restore reads.dump.js
loaded reads.dump.js into a fresh VM
waiting: #2 tools.read({"path":"a"})
> .bt 2
#2 tools.read({"path":"a"})
  async function <anonymous> at reads.dump.js:13:12 (run)
      const a = (await $dump.p2);
                 ^
    a = undefined, b = undefined
> .resolve 2 "AA"
[waiting] #4 tools.read({"path":"b"})
```

`.dump FILE` of a waiting session (in `djs repl --frozen`) writes what is in flight too
(D2604): the waiting inputs are its
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

### `debugger;` pauses

In `djs repl --frozen`, the dumping mode, `debugger;` pauses (D3060): in an async
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
(.dump - shows the continuation, .c continues, .abort drops it)
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
and `.bt` shows the paused activation's bindings. `.dump -` prints the dump the session
is: the paused input is a continuation that starts with the pause, `await
$djs.resume(1, "debugger", [], {"origin":"input 2"})`, followed by the rest of its code
(the loop's later iterations, with their `debugger;`). `.continue` (`.c`) answers the
pause in place and the input runs on to its end or its next pause; `.c 3` picks the
pause `#3` when several inputs are paused. To continue from the dump instead, `.dump
p.js`, `.restore p.js` (a fresh VM runs the dump: the pause is open again, at its line in
`p.js`) and `.c`; the continuation pauses again at the next `debugger;` and can be dumped
again from there. `.abort`, or Ctrl-C at an empty prompt while something is paused, drops
the paused input with everything else in flight (`vm.abort()`, as Ctrl-C does to a
running input: other waiting inputs and their open calls go too, and the message counts
them); the globals stay. Without `--frozen`, `debugger;` does nothing.

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
