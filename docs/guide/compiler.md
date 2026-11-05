# The compiler: tokens to bytecode, no AST

A codemode script is compiled once, run once, and thrown away. Whatever the compiler
does before the first instruction runs is pure latency, so djs compiles the way QuickJS
does: a recursive-descent parser that emits bytecode *while* it parses, with no syntax
tree in between. This page explains how that works for a language whose grammar is full
of constructs that only make sense once you have read past them, and what the missing
tree costs.

> **In short.** The parser reads tokens and writes stack-machine bytecode directly.
> Ambiguities (arrow heads, destructuring, TypeScript generics) are settled by scanning
> ahead at the token level or by rewinding the lexer, never by undoing emitted code.
> Variable names are emitted as scope-generic placeholders and resolved in one pass
> after the whole unit is parsed, when hoisting is known. TypeScript types are skipped
> as they are parsed.

## The pipeline

```text
source text
  │  lexer: tokens with spans, rescanned on the parser's request
  ▼
parser + emitter: recursive descent, one function state per function
  │  emits phase-1 bytecode: name placeholders, symbolic labels, scope markers
  ▼  (the whole compilation unit parsed)
resolution, per function, children before parents
  │  local / argument / box / capture / global storage, TDZ checks, hoisting,
  │  peepholes, one linear re-emit, jump fixups, stack depths, line table
  ▼
function template cells in the heap
```

Only the last step touches the arena: the compiler interns atoms and string constants
while it parses, and writes one **function template** cell per function at the end
(D1131). Everything else (code under construction, scope trees, fixup lists) is Rust
state that is dropped when `compile` returns. A template holds the bytecode, the
constant pool, the line and handler tables and the child templates, so compiled code is
part of the heap and of every snapshot.

The template is laid out for the builtins, which are most of the templates in every
heap (D2444). The bytecode, the capture descriptors and the handler table sit inline at
the end of the template cell, so a call finds its code at a fixed offset from the
template and no separate code cell pays a header and padding. Everything a builtin never
has, the child templates and the debug info (script, source span, line table, local
names), lives in a separate info cell that a builtin's template does not point to: a
builtin's template is 48 bytes plus its code.

The functions of one compilation unit share one constant pool (D2446): a string or number
that many functions use, an error message or a property name passed as a string, is
stored once, and the unit pays one cell header instead of one per function. The top
level gets a pool of its own, since it runs once. Indexes are handed out as the functions
are emitted, so the short `push_const8` form covers the first 256 constants. The base
image drops the entries that only dead code used (the prelude's set-up code), keeping
the order, so every index only gets smaller and the code is patched in place.

## Emitting while parsing

Expressions are parsed by precedence climbing, and each operator is emitted after its
operands, which is exactly the order a stack machine wants:

```js
const price = 12, qty = 3, discount = 5;
return price * qty - discount;
```

<!-- gen:disasm -->
```text
function 0 <anonymous>: params=0 locals=4 stack=2 captures=0
  handler [3, 19) -> 19 depth 0
    0  async_start loc0 (<state>)
    3  push_i8 12
    5  put_loc1 (price)
    6  push3
    7  put_loc2 (qty)
    8  push_i8 5
   10  put_loc3 (discount)
   11  get_loc1 (price)  ; 2:8
   12  get_loc2 (qty)  ; 2:16
   13  mul  ; 2:14
   14  get_loc3 (discount)  ; 2:22
   15  sub  ; 2:20
   16  async_return loc0 (<state>)
   19  async_reject loc0 (<state>)
```
<!-- /gen -->

The function is a codemode body, an async function (D008), which is why it starts with
`async_start` and ends with `async_return`; the `handler` line is the region that turns
an exception into a rejected promise. In between, every operand is pushed and every
operator pops its inputs and pushes its result. The [opcode reference](ref-opcodes.md)
lists every instruction; this page uses a handful:

<!-- gen:ops get_loc put_loc mul sub new_box get_box put_box get_cap_chk fclosure -->
| opcode | byte | immediates | size | pop → push | cost | description |
| --- | ---: | --- | ---: | --- | --- | --- |
| `get_loc` | 39 | Loc | 3 | 0 → 1 | Basic | `-> v` |
| `put_loc` | 40 | Loc | 3 | 1 → 0 | Basic | `v ->` |
| `mul` | 123 | – | 1 | 2 → 1 | Basic | `a b -> a * b` |
| `sub` | 122 | – | 1 | 2 → 1 | Basic | `a b -> a - b` |
| `new_box` | 57 | Loc | 3 | 0 → 0 | Alloc | `loc = new Box(uninitialized)` |
| `get_box` | 51 | Loc | 3 | 0 → 1 | Basic | `-> v`: the value of the Box in local `loc`. |
| `put_box` | 52 | Loc | 3 | 1 → 0 | Basic | `v ->` |
| `get_cap_chk` | 63 | Cap | 3 | 0 → 1 | Basic | `-> v` with a TDZ check. |
| `fclosure` | 13 | Child | 5 | 0 → 1 | Alloc | `-> function`: create a closure of child template `child`. |
<!-- /gen -->

## Ambiguity without backtracking

JavaScript's grammar does not always say what a token starts until much later. Is
`(a, b)` a parenthesized expression or the parameters of an arrow function? Is `[a, b]`
an array literal or the target of a destructuring assignment `[a, b] = pair`? A parser
that builds a tree can parse a "cover" form and reinterpret it afterwards. A parser that
emits code as it goes has already committed to bytecode.

djs follows one rule (D1080): **speculation never emits bytecode.** Every ambiguity is
decided before anything is emitted for it:

- **Token scans.** For `(`, a scan on a copy of the lexer skips the balanced parentheses
  and checks for `=>`. It bails out at the first token that cannot be in a parameter
  list, so `(a + b) * c` costs a token or two. For `[` and `{`, a scan checks whether
  the matching closer is followed by `=`, and remembers the answer for every nested
  bracket it passed, so a large data literal is scanned once, not once per nesting
  level.
- **Rewinding the lexer.** A checkpoint is the lexer state plus the current token, a
  small struct that is copied. TypeScript's `f<T>(x)` versus `f < T > (x)` is decided by
  skipping a type argument list speculatively and looking at what follows; if it was not
  one, the lexer is put back. Nothing else needs to be restored, because nothing else
  changed.
- **Deferred patterns.** In `let [a, b] = pair`, the pattern comes before its value, but
  its code must run after. The parser skips the pattern with a bracket scan, parses and
  emits the value, rewinds the lexer to the pattern and emits it, then jumps forward
  past the value again (D1360).

The one place where emitted code is rewritten is assignment: `a.b.c = v` has already
emitted `get_field c` when the parser sees `=`, so it turns that last instruction into
the matching store. That is a rewrite of one instruction, at the end of the buffer.

> **Trade-off.** Lexer-only speculation over truncating emitted code. The original
> design planned checkpoints that would truncate the bytecode, constants and scope state
> back to a saved point. Every piece of compiler state added later (child functions,
> captures, handler entries, line entries) would have had to be part of that checkpoint,
> and forgetting one is a bug that shows up only on unusual input. With lexer-only
> checkpoints there is nothing to forget. The cost: some constructs are scanned twice,
> once to decide and once to parse.

## Names are resolved after parsing

When the parser meets an identifier, it often cannot know what it refers to. `x` in an
inner function may be a `var` its parent declares further down, a `let` that is still in
its temporal dead zone, or a global. So the parser emits a scope-generic placeholder and
records the name and scope in a side table. After the whole compilation unit is parsed,
a **resolution pass** walks the functions in reverse creation order, which visits
children before parents without recursion (D1081). For each name it decides the storage:

| binding | storage |
| --- | --- |
| captured by an inner function | a local slot holding a heap **box** (`get_box`/`put_box`) |
| a parameter, not captured | its argument slot (`get_arg`) |
| any other local | a local slot (`get_loc`) |
| not found in any enclosing function | a global lookup (`get_global`) |

and then re-emits the function's code in one linear pass, replacing placeholders,
dropping scope markers, inserting TDZ checks where a read might come before the
declaration has run (and only there), patching jumps and computing the maximum stack
depth.

A captured variable lives in a box from the moment its scope is entered (D1083), so a
closure holds the box, never a pointer into its parent's frame:

```js
let count = 0;
const bump = () => { count++; };
bump(); bump();
return count;
```

<!-- gen:disasm -->
```text
function 0 <anonymous>: params=0 locals=3 stack=2 captures=0
  handler [3, 29) -> 29 depth 0
    0  async_start loc0 (<state>)
    3  new_box loc1 (count)
    6  push0
    7  put_box loc1 (count)
   10  fclosure child0
   15  set_loc2 (bump)  ; 3:1
   16  undefined
   17  call0  ; 3:5
   18  drop
   19  get_loc2 (bump)  ; 3:9
   20  undefined
   21  call0  ; 3:13
   22  drop
   23  get_box loc1 (count)  ; 4:8
   26  async_return loc0 (<state>)
   29  async_reject loc0 (<state>)
function 0.0 bump: params=0 locals=0 stack=1 captures=1
    0  get_cap_chk cap0 (count)  ; 2:22
    3  inc  ; 2:27
    4  put_cap_chk cap0 (count)  ; 2:22
    7  return_undef
```
<!-- /gen -->

`count` is captured by the arrow function, so the body keeps it in a box (`new_box`,
then `put_box` and `get_box`), and `fclosure` copies the box reference into the new
closure, where `bump` reaches it as capture 0 (`get_cap_chk`). Through a capture the TDZ
check stays, because a closure can run before the declaration it captures has; in the
body, where the read comes textually after the declaration, the resolver dropped it.
`bump` itself is not captured, so it stays a plain local, and its store followed by a
load became one `set_loc`.

> **Trade-off.** Boxes for every captured binding over closing over the stack. QuickJS's
> closures point into the frame's variables until the frame returns, then copy them out
> (`close_var`). djs pays an allocation per captured binding per scope entry instead. In
> return no frame ever holds something a closure points into, so a frame can be
> suspended, copied into a generator object or snapshotted as plain values, and
> per-iteration `let` bindings in loops come for free: each iteration gets a new box.

## TypeScript is skipped while parsing

TypeScript is a compile flag, not a separate front end (D1480). With it on, the parser
calls a **type skipper** wherever a type may appear, and the skipper consumes the type
without emitting anything: annotations, type parameter and argument lists, `as` and
`satisfies`, `!`, interfaces and type aliases, `declare`, overload signatures. The rules
for when something is a type are ported one to one from dquickts, along with its error
messages, so the two engines agree on what is valid TypeScript.

Constructs with runtime meaning compile directly to bytecode the way tsc would emit
them: enums and namespaces become closures called with their object, `const enum`
members are folded into constants, and parameter properties become assignments in the
constructor (D1482-D1484). `djs strip-types` shows the erasure, blanked like
ts-blank-space so line and column numbers stay where they were (D1481).

> **Trade-off.** Erasing types in the parser over a separate transpile step. Codemode
> scripts are often TypeScript, and a separate pass (like ts-blank-space in front of
> QuickJS) would scan the source twice and need its own parser. The cost: the type
> skipper is grammar the JavaScript parser must step around, and it must follow tsc's
> disambiguation rules exactly.

## The bytecode

The bytecode is stack-based, with variable-length instructions: a `u8` opcode followed
by little-endian immediates (local slots, atom ids, constant indexes, relative jump
offsets). The instruction set is declared once, in the `ops!` table in
`bytecode/ops.rs`, with each instruction's immediates, stack effect, fuel cost and flags
(D1082, D1130). The emitter, the decoder, the disassembler, the stack-depth checker, the
opcode hash that images are checked against and the [opcode reference](ref-opcodes.md)
are all generated from that table.

There are no general superinstructions. What the compiler does instead is a small set of
layout changes and peepholes over the phase-1 code (D1860):

- loops are **rotated**: the condition is tested at the bottom, so an iteration costs
  one conditional jump;
- `i++` as a statement on a plain local becomes `inc_loc`;
- jumps to jumps are threaded, `not` before a conditional jump flips the jump, a store
  followed by a load of the same local becomes `set_loc`.

The final code is then made **short** (D2443). Most operands are small: locals, arguments
and constant indexes below 256, calls with up to three arguments, jumps of a few dozen
bytes. So the most frequent instructions have short forms that the emitter picks when
the operand fits. `get_loc2` has the operand in its opcode, `get_loc8 12` has a
one-byte operand, `call_intrinsic8` takes its argument count from the intrinsic's
declared arity, and `get_builtin_callee8` is `get_builtin; undefined` in two bytes.
Jumps are emitted with 4-byte offsets, and the last step shortens every jump whose
offset fits a byte, repeating until no shortened jump pushes another out of range. Code
after a `return`, `throw` or `jump` up to the next label is dropped. This halved the
builtins' bytecode (86 to 42 KB per VM); each short form runs exactly like its wide
form, and big functions simply keep the wide ones.

```js
let sum = 0;
for (let i = 0; i < 10; i++) sum += i;
console.log(sum);
```

<!-- gen:disasm -->
```text
function 0 <anonymous>: params=0 locals=3 stack=3 captures=0
  handler [3, 43) -> 43 depth 0
    0  async_start loc0 (<state>)
    3  push0
    4  put_loc1 (sum)
    5  push0
    6  put_loc2 (i)
    7  jump8 -> 16
    9  get_loc1 (sum)  ; 2:30
   10  get_loc2 (i)  ; 2:37
   11  add  ; 2:34
   12  put_loc1 (sum)  ; 2:30
   13  inc_loc loc2 (i)  ; 2:26
   16  get_loc2 (i)  ; 2:17
   17  push_i8 10
   19  lt  ; 2:19
   20  jump_if_true8 -> 9  ; 2:30
   22  get_global "console" ic0  ; 3:1
   29  get_method "log" ic1  ; 3:8
   36  get_loc1 (sum)  ; 3:13
   37  call1  ; 3:12
   38  drop
   39  undefined
   40  async_return loc0 (<state>)
   43  async_reject loc0 (<state>)
```
<!-- /gen -->

The loop is entered with one jump to the condition at the bottom; every iteration after
that is the body, `inc_loc`, the comparison and one backward conditional jump. Two more
details show up here: `console` is a global the host defines, read with an inline
cache like a property (`get_global "console" ic0`), and `get_method "log" ic1` carries
the number of its own inline cache slot (D1620). Measured in wasm, rotation, the peepholes and the matching interpreter fast
paths cut the instructions executed by the tightest benchmark loops by 35 to 38%.

> **Trade-off.** A stack machine over registers. A register VM executes fewer
> instructions, but allocating registers well needs either a syntax tree or a much
> smarter single-pass allocator. A stack machine is the natural target for code emitted
> in parse order. The cost is instruction count, which the peepholes, the interpreter's
> fast paths and inline caches claw back where it matters.

## Limits

The compiler recurses on the Rust stack, unlike the interpreter: compilation is bounded
by the size of the input and never needs to suspend. A nesting-depth limit turns
pathological input (ten thousand nested parentheses) into a SyntaxError instead of a
stack overflow. Codemode scripts are always strict; there is no `with`, no direct
`eval`, and no ES modules: `import()` compiles and rejects (D1582). A Script goal with
global `var` bindings and completion values exists for test262, indirect `eval` and the
REPL (D004, D1581).

> **Trade-off.** No AST at all. Without a tree there are no optimization passes over
> whole functions: no constant folding across statements, no escape analysis, no
> inlining. For code that runs once, that is the right trade: the compiler allocates
> almost nothing, and compiling TypeScript's own `typescript.js` takes fewer wasm
> instructions in djs than in quickjs-wasi, the QuickJS build pi uses today. What it
> costs is cleverness the bytecode cannot have, and a parser that has to answer every
> ambiguity before it emits.

## Where to look

- [`crates/djs-core/src/compiler/`](../../crates/djs-core/src/compiler/): the
  parser (`compiler/parser.rs`, `compiler/expr.rs`, `compiler/stmt.rs`), the scans
  (`compiler/scan.rs`), resolution (`compiler/resolve.rs`), peepholes
  (`compiler/peephole.rs`), TypeScript (`compiler/ts.rs`).
- [`crates/djs-core/src/bytecode/ops.rs`](../../crates/djs-core/src/bytecode/ops.rs):
  the instruction table.
- `djs disasm file.js`, a command of djs's command-line tool, prints the bytecode of any
  script, as the listings on this page show it.

Decisions on this page:

<!-- gen:decisions D1080 D1360 D1081 D1083 D1082 D1130 D1131 D1860 D1620 D1480 D1481 D1482 D008 D1582 D2443 D2444 D2446 -->
- D1080: Speculative parsing never emits bytecode
- D1360: destructuring patterns are parsed after their value (deferred patterns)
- D1081: Name resolution runs once per compilation unit, children first
- D1083: Captured bindings live in heap Box cells
- D1082: Bytecode encoding and the instruction table
- D1130: The bytecode table and emitter API
- D1131: Code cells
- D1860: Loop rotation and peepholes in the compiler; inc_loc/dec_loc; fast_more
- D1620: Inline caches: verified table indexes in a per-template array
- D1480: TypeScript is a compile flag; the skipper ports dquickts' rules
- D1481: Erased ranges are recorded by committing code, blanked like ts-blank-space
- D1482: Enums and namespaces are closures; their members resolve after parsing
- D008: Codemode top level is a function body
- D1582: `import()` without modules: compiled, rejected in a job
- D2443: Short opcode forms and shortened jumps, a recorded trade
- D2444: Code inline in the template, the rest of a user function in FuncInfo
- D2446: One constant pool per compilation unit
<!-- /gen -->
