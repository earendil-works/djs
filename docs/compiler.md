# djs: Compiler and Bytecode Design

Status: design note for issue 0008. It refines ARCH.md "Compiler (no AST)" and
"Interpreter" so that 0006 (P0) and the P1 issues can be implemented without
re-deciding the fundamentals. Decisions D1080 to D1086 in `docs/decisions.md` record the
load-bearing choices. Places that deviate from or sharpen ARCH.md are marked
**[ARCH]**; section 11 lists them all with the proposed ARCH edits.

QuickJS (`quickjs.c`, `quickjs-opcode.h`) is the main reference. Where we do the same
thing, the note says so briefly. Where we differ, it says why.

Contents:

1. Pipeline
2. Parser structure
3. Ambiguities
4. Bytecode
5. Scopes and the resolution pass
6. Function templates and heap cells
7. Frames and the interpreter contract
8. Error reporting and early errors
9. Worked examples
10. Suggested issue split for P0 and P1
11. Deviations from ARCH and open questions

---

## 1. Pipeline

```
source (&[u8], UTF-8)
  │  lexer (0003): tokens with spans, rescans driven by the parser
  ▼
parser + emitter: recursive descent, one FuncState per function on a stack
  │  emits phase-1 bytecode: scope-generic name ops, symbolic labels,
  │  enter/leave scope markers; records scopes, names, children, lines, handlers
  ▼  (whole compilation unit parsed)
resolution, per function, children before parents:            [D1081]
  │  binding storage (local / arg / box / capture / global), TDZ checks,
  │  hoisting, one linear re-emit, jump fixups, stack depth, line table
  ▼
function template cells in the heap (children first), root template returned
```

- API: `compile(heap: &mut Heap, src: &[u8], opts: &CompileOptions) -> Result<Ref,
  CompileError>`. `CompileOptions { goal: Script | AsyncBody, typescript: bool,
  privileged: bool, filename: &str }`. The result is the top-level function template.
  The caller roots it before anything can collect (allocation never collects, see
  0005, so the compiler itself needs no roots).
- The compiler interns atoms and allocates constant strings in the heap while
  parsing. Everything else (bytecode under construction, scope trees, tables) is
  transient Rust state, dropped at the end of `compile`.
- Out of memory during compilation is `CompileError::OutOfMemory`, which the embedding
  API reports as an out-of-memory error, not a SyntaxError.

## 2. Parser structure

### 2.1 State

```rust
struct Parser<'a, 'h> {
    lx: Lexer<'a>,          // positioned after `tok`
    tok: Token,             // current token
    prev_end: u32,          // end of the previous token (for ASI and spans)
    heap: &'h mut Heap,
    funcs: Vec<FuncState>,  // every function of the unit, in creation order
    cur: FuncId,            // index into `funcs` of the function being parsed
    depth: u32,             // nesting-depth counter (2.4)
    pattern_scan: PatternScan, // memo for destructuring detection (3.3)
    opts: CompileOptions,
    buf8: Vec<u8>, buf16: Vec<u16>, // scratch for cooking identifiers and strings
}
```

`FuncState` (one per function, kept until the end of the unit, see 5.3):

| field | contents |
| --- | --- |
| `kind` | `Script`, `AsyncBody`, `Normal`, `Arrow`, `Method`, `Getter`, `Setter`, `ClassCtor`, `DerivedCtor`, `ClassFieldInit`; plus `is_async`, `is_generator`, the state local `state_var`, `in_params` and the async catch-all region start (7.8) |
| `parent`, `parent_scope` | the enclosing function and the scope in it where this function was created |
| `code: Vec<u8>` | phase-1 bytecode |
| `scopes: Vec<Scope>` | the scope tree (5.1) |
| `vars: Vec<Var>` | every binding declared in this function |
| `refs: Vec<NameRef>` | the name-reference side table (5.2) |
| `labels: Vec<u32>` | label id → phase-1 position (set by the `label` pseudo-op) |
| `consts`, `children` | constant pool (values) and child functions (`FuncId`s) |
| `captures: Vec<Capture>` | filled by the resolution of this function's descendants |
| `lines: Vec<(u32, u32)>` | (phase-1 code position, source byte offset) |
| `handlers: Vec<Handler>` | try regions as label triples plus operand depth (7.4) |
| `flags` | `uses_this`, `uses_arguments`, `has_simple_params`, `has_use_strict`, ... |

### 2.2 Token helpers

| helper | does |
| --- | --- |
| `next()` | `tok = lx.next_token()?`, sets `prev_end` |
| `peek()` | copies `lx`, scans one token, discards the copy (cheap: the lexer is `Copy`) |
| `eat(kind)` / `expect(kind)` | consume if present / or SyntaxError `expecting '%c'` |
| `expect_semicolon()` | ASI (3.5) |
| `binary_op_tok()` | if `tok.kind == Gt`, `tok = lx.rescan_greater(tok)`; called wherever an operator may follow a complete operand, so `>>`, `>=`, `>>=`, ... exist only there |
| `primary_slash()` | in primary position, `Slash`/`SlashAssign` → `tok = lx.rescan_as_regex(tok)?` |
| `template_continue()` | after a substitution expression, requires `RBrace` and calls `lx.rescan_template_continuation(tok)?` |
| `ident_atom(tok)` | `lx.ident_value` → atom (no escapes: borrowed bytes, interned directly) |
| `number_value(tok)` | `lx.number_literal(&tok)` → `crate::num::literal_digits_value(radix, digits)` (separators allowed); BigInt later |

Lookahead never crosses a position whose meaning depends on parser context without
applying the same rule the parser would (regex after an operand, `>` merging,
template continuation). That is why scans (3.1) carry their own regex heuristic.

### 2.3 Grammar entry points and precedence climbing

Statements are plain recursive descent (`parse_statement_list_item`,
`parse_statement`, one function per statement kind). Expressions:

```
parse_expr(no_in)                  // comma lists, loops (no recursion per element)
 └ parse_assign(no_in)             // arrows, async arrows, yield, assignment ops
    └ parse_conditional(no_in)     // a ? b : c
       └ parse_binary(min_prec, no_in)   // precedence climbing, loops
          └ parse_unary()          // delete void typeof + - ~ ! await, prefix ++/--
             └ parse_postfix()     // postfix ++/-- (no newline before)
                └ parse_lhs()      // new, calls, members, optional chains, tagged templates
                   └ parse_primary()
```

Binary operators (`binary_op_tok()` first, so `Gt` may become `Shr`/`Ge`/...):

| prec | operators | assoc | emits |
| --- | --- | --- | --- |
| 1 | `??` | left | `dup; is_nullish; jump_if_false L; drop; <rhs>; L:` |
| 2 | `\|\|` | left | `dup; jump_if_true L; drop; <rhs>; L:` |
| 3 | `&&` | left | `dup; jump_if_false L; drop; <rhs>; L:` |
| 4 | `\|` | left | `bor` |
| 5 | `^` | left | `bxor` |
| 6 | `&` | left | `band` |
| 7 | `==` `!=` `===` `!==` | left | `eq` `ne` `strict_eq` `strict_ne` |
| 8 | `<` `>` `<=` `>=` `instanceof` `in` | left | `lt` `gt` `le` `ge` `instanceof` `in` (`in` off when `no_in`) |
| 9 | `<<` `>>` `>>>` | left | `shl` `sar` `shr` |
| 10 | `+` `-` | left | `add` `sub` |
| 11 | `*` `/` `%` | left | `mul` `div` `mod` |
| 12 | `**` | right | `pow` |

```
parse_binary(min):
    parse_unary()                      // or a private name `#x` followed by `in`
    loop:
        op = binop(binary_op_tok()); if op.prec < min: break
        next(); emit prologue for short-circuit ops
        parse_binary(op.prec + (op is right-assoc ? 0 : 1))
        emit op / place label
```

Special rules, all checked in `parse_binary`:
- `??` cannot be mixed with `||`/`&&` without parentheses: track the operator kinds
  of the current chain; error `cannot mix ?? with && or ||` (qjs reports some orders
  as `expecting ';'`; difftest cases pin the wording).
- `**` with a unary expression on the left (`-a ** b`) is a SyntaxError
  (`unparenthesized unary expression can't appear on the left-hand side of '**'`).
  `parse_unary` returns whether it parsed a unary operator, and `parse_binary` rejects
  `**` after it.
  `(-a) ** b` is fine: parentheses clear the flag.
- `#x in obj`: a `PrivateName` in operand position must be followed by `in` (P1
  classes).

Optional chains (`a?.b.c()`, `a?.[k]`, `f?.()`): each `?.` emits `dup; is_nullish;
jump_if_true L_short` for the value being tested, where `L_short` is a landing pad of
the whole chain. The operand depth at each `?.` is known statically (1 for `a?.b`, 2
for a method pair before `?.()`), so each depth gets its own pad that drops that many
values and pushes `undefined`, and the normal path jumps over the pads at the end of
the chain. A tagged template in a chain is an error (`template literal cannot appear
in an optional chain`), and so is `new a?.b()`.

`parse_assign` handles, in this order: `yield` (in generators), arrows and async
arrows (3.1, 3.2), destructuring assignment (3.3), then `parse_conditional` followed by
an assignment operator (lvalue conversion, 3.7).

### 2.4 Nesting-depth limit

The parser recurses on the Rust stack (ARCH). A counter `depth` is incremented on
entry to `parse_statement`, `parse_assign`, `parse_unary` and `parse_primary` (for
`(`, `[`, `{`, function bodies) and decremented on exit. Above `MAX_DEPTH` the parser
fails with SyntaxError `stack overflow` (QuickJS wording; qjs fails at about 920
nested parentheses).

- `MAX_DEPTH` is 1000 (0014). It is pinned by a test that compiles the deepest accepted
  nesting of each recursive construct (parens, arrays, objects, unary chains,
  ternary chains, blocks, `if`, assignments, `new`) on a thread with a 1 MiB stack,
  the wasm32 default. Natively that depth needs under 512 KiB, so there is a 2x
  margin. One parenthesis level costs 3 depth units (assignment, unary, primary), so
  about 330 nested parentheses compile.
- Constructs that are naturally chains are loops, so they do not count: comma
  lists, binary operator chains (precedence climbing), member/call chains, and
  `else if` chains (`parse_if` loops on `else if`).
- Phases after parsing do not recurse: resolution walks `funcs` in reverse creation
  order (5.3), and the stack-depth check uses a worklist.

### 2.5 Checks the lexer leaves to the parser (0003 follow-ups)

| check | where | message (QuickJS) |
| --- | --- | --- |
| escaped reserved word (`tok.is_escaped_reserved()`) as identifier reference, binding or label | `parse_ident_ref`, `parse_binding_ident`, labels | binding: `variable name expected`; expression: `unexpected token in expression: '%s'`; statement start: `'%s' is a reserved identifier` (name from `kw.as_str()`) |
| escaped contextual keyword used as a keyword (`l\u0065t x`, `\u0061sync function`) | every contextual check uses `tok.is_contextual(kw)`, which is false for escaped tokens, so they fall through to the identifier path; strict-reserved names then fail there | as below |
| strict-mode reserved words (`kw.is_strict_reserved()`: `let static yield implements interface package private protected public`) as reference or binding | identifier helpers | as for reserved words; `yield` in expressions: `unexpected 'yield' keyword` |
| `let` in a destructuring binding (`let [let] = x`) | pattern parsing | `invalid destructuring target` |
| `eval` / `arguments` as a binding name | binding helpers | `invalid variable name in strict mode` (vars), `invalid argument name in strict code` (params), `invalid function name in strict code` |
| `eval` / `arguments` as an assignment target | lvalue conversion (3.7) | `invalid lvalue in strict mode` |
| `await` as an identifier in async functions and the async body, `yield` in generators | identifier helpers, with `FuncState::is_async`/`is_generator` and `Parser::await_reserved` (also arrow parameters nested in an async context); escaped `aw\u0061it` too (0041) | references: `unexpected token in expression: 'await'`; bindings: `variable name expected`; parameters: `missing formal parameter`; in non-async arrow parameters inside an async function: `unexpected 'await' keyword` |
| `await`/`yield` expressions in formal parameters | `FuncState::in_params` | `await in default expression`, `yield in default expression` |
| `#constructor` | class element names | `invalid method name` |
| untagged template with `INVALID_ESCAPE` | template parsing; the error comes from `cook_template` | `malformed escape sequence in string literal` |
| a token directly after a regex literal (`/a/g\u0069`) | nothing special: an identifier after an operand is an ordinary syntax error | `expecting ';'` |
| regex pattern validity | the regex compiler (0011) at compile time, once it lands; until then patterns are accepted unchecked | |

Directive prologues are still recognized even though everything is strict:
`"use strict"` in a function with non-simple parameters is an error (`"use strict"
not allowed in function with default or destructuring parameter`). A directive is a
`String` token without `HAS_ESCAPE` whose content is exactly `use strict`.

## 3. Ambiguities

**[ARCH]** Principle (D1080): **speculation never emits bytecode.** Every ambiguity
is resolved either by a token-level scan or by speculation that only moves the lexer
(the TypeScript type skipper emits nothing). So a checkpoint is just lexer state:

```rust
#[derive(Clone, Copy)]
struct Checkpoint { lx: Lexer<'a>, tok: Token, prev_end: u32 }   // 40 bytes
```

The one rewrite of emitted code is the lvalue conversion (3.7), which removes the
last emitted instruction. ARCH technique 2 (truncating bytecode, constants and scope
depth back to a checkpoint) is not needed. Avoiding it removes a class of bugs: every
piece of compiler state added later (child functions, captures, handlers, line
entries) would otherwise have to be part of the checkpoint.

| ambiguity | resolved by | cost |
| --- | --- | --- |
| arrow head `(a, b) =>` vs parenthesized expression | bailing paren scan (3.1) | scan stops at the first token that cannot be in a parameter list |
| `async` arrow vs call `async(x)` | `peek` plus the same scan (3.2) | |
| destructuring assignment vs literal | memoized bracket scan (3.3) | each literal is scanned once, however deeply nested |
| regex vs divide | the parser's position; scans use a previous-token heuristic (3.4) | |
| ASI, restricted productions | the `NEWLINE_BEFORE` flag (3.5) | |
| `let` as identifier vs declaration | always a declaration (strict) (3.6) | |
| `for (... in/of ...)` vs `for (;;)` | head scan for `;` at depth 1 (3.8) | |
| assignment target vs expression | lvalue conversion of the last instruction (3.7) | |
| TS `f<T>(x)` vs `f < T > (x)`, TS arrows | emission-free speculation with a checkpoint (3.9) | a skip of the type arguments or the parenthesized group |

### 3.1 Arrow heads

In `parse_assign`:
- `Ident` (not a keyword) followed by `Arrow`: a single-parameter arrow. One `peek()`.
- `LParen`: run `scan_arrow_head()` on a copy of the lexer. It returns `true` when the
  balanced `(...)` is followed by `=>` with no newline before the arrow. If so, parse
  the parameter list directly as formal parameters of a new function. Otherwise parse
  a parenthesized expression. In both cases nothing was emitted before the decision.

`scan_arrow_head` keeps a bracket stack (`(`, `[`, `{`, template `${`) and, at depth 1,
a tiny state machine that **bails out as soon as the tokens cannot be a parameter
list**, so `(a + b) * c` and `(function () { ... })()` cost a token or two:

| at depth 1, expecting... | allowed | anything else |
| --- | --- | --- |
| a parameter | `Ident` (contextual keywords included), `...`, `{`/`[` (then skipped as balanced) | not an arrow |
| after a parameter | `,`, `)`, `=` (then skip a default value) | not an arrow |
| inside a default value | anything, until `,` or `)` at depth 1 | |

Template continuations and regexes inside the scan follow 3.4. Errors from the lexer
during a scan mean "not an arrow"; the real parse then reports the same error with
the right message. In TypeScript mode the bailout also accepts `?`, `:` and type
tokens after a parameter (P3, see 3.9).

The bailout makes the scan cost proportional to the arrow head, not to the
parenthesized expression. QuickJS's `js_parse_skip_parens_token` always skips the
whole balanced group.

### 3.2 `async` arrows and functions

`tok.is_contextual(Kw::Async)` followed by, with no newline in between:

| next | meaning |
| --- | --- |
| `function` | async function expression/declaration |
| `Ident` and then `=>` | async arrow with one parameter |
| `(`, and `scan_arrow_head()` is true | async arrow |
| anything else | the identifier `async` (a call `async(x)`, a reference, ...) |

A newline between `async` and `function`/the parameter makes `async` an identifier
(ASI may follow). Inside an async arrow's parameters, `await` is reserved (an early
error). As implemented (0041): `try_arrow` handles both arrow forms (`parse_arrow`
consumes the `async`), `parse_primary` the function expression and
`parse_statement_list_item` the declaration (`at_async_function`). Object literal methods
take `async`, `*` and `async *` prefixes (`async` only without a newline after it).

### 3.3 Destructuring assignment vs literal

When `parse_assign` starts at `{` or `[`, it must know whether the literal is an
assignment pattern (`[a, b] = [b, a]`, `({x} = o)`), before emitting anything,
because patterns compile to completely different code.

Rule: the bracket is a pattern iff its matching closer is directly followed by an
`Assign` token (`=` only; `[a] += 1` is an error, `==` and `=>` are other tokens).

`PatternScan` memo (this is where we differ from QuickJS, which rescans every
nested literal, O(n·depth) for nested data):

```rust
struct PatternScan { start: u32, end: u32, starts: Vec<u32> }  // sorted start offsets
```

- If `start <= tok.start < end`, the last scan covered this token: binary search
  `starts`. (The range check matters after a TS rewind to before the scanned
  region.)
- Otherwise scan from this bracket to its matching closer, replacing the memo. For **every** `{`/`[`
  closed during the scan (nested ones included), check whether the next token is
  `Assign`, and if so record its start. Sort, set `end`.

A data literal like `tools.call({ a: [1, 2], b: { c: 3 } })` is scanned once (one
extra pass over its tokens) and parsed once. The record is purely lexical, so it is
also right for brackets in nested function bodies inside the literal.

Cover grammar: `({a = 1})` is valid only as a pattern. The pattern decision comes
before parsing, so the literal parser simply rejects `a = 1` shorthand (`expecting
'}'` like qjs), and the pattern parser accepts it. Duplicate `__proto__` is an error
only in literals. `({a}) = 1` is an error (`invalid assignment left-hand side`): the
`}` is followed by `)`, so `{a}` is a literal, and lvalue conversion (3.7) rejects the
parenthesized literal.

Binding patterns (`let {a} = o`, parameters, `catch ({a})`) need no scan: the grammar
position says it is a pattern.

As implemented (0036, `scan::PatternScan`): the memo records, for every bracket, what
follows its closer (`Follow`): `=`, an element end (`,` `]` `}`), `of`/`in`, or something
else. In `parse_assign` a bracket is an assignment pattern iff `Follow::Assign`; in a
for-in/of head iff `Follow::InOf`; inside an assignment pattern a nested bracket is a
nested pattern iff `Assign` or `ElemEnd` (otherwise it starts a member expression such
as `[a][0]`). The object literal parser therefore never sees a pattern: `{a = 1}` and
duplicate `__proto__` are plain literal errors there.

### 3.4 Regex vs divide

The parser knows: a `/` or `/=` in primary position is a regex (`primary_slash()`),
and anywhere else it is division. The lexer never guesses.

Scans (3.1, 3.3, 3.8) do not parse, so they use QuickJS's `is_regexp_allowed`
heuristic on the previous token. `/` starts a regex unless the previous token is
`Ident` (other than unescaped `yield`, `await`, `of`), a literal, `this`, `super`,
`true`, `false`, `null`, `)`, `]`, `}`, `++` or `--`. The one known mistake is a regex
right after a block's `}` inside the scanned region (`(function(){ if (a) {} /[(]/.test(s) })`).
It matters only if the regex contains unbalanced brackets, and then it gives a wrong
decision and a SyntaxError for valid code, as in QuickJS. A regression test pins it
as a known limitation.

### 3.5 ASI and restricted productions

`expect_semicolon()`: consume `;`, or accept (insert one) when `tok` is `}` or `Eof`,
or `tok.newline_before()`. Otherwise `expecting ';'`. Exceptions and restricted
productions:

| construct | rule |
| --- | --- |
| `do ... while (c)` | `;` is optional even without a newline |
| `for (...;...;...)` header | `;` is never inserted |
| `return`, `break`, `continue` | a newline right after the keyword ends the statement (no operand/label) |
| `throw` | newline after `throw` → `line terminator not allowed after throw` |
| postfix `++`/`--` | `tok.newline_before()` → not postfix; ASI ends the operand, the operator becomes prefix of the next statement |
| `=>` | newline before `=>` → error (the arrow scan requires no newline) |
| `async` | 3.2 |
| `yield` | newline after `yield` → no operand |
| empty statement | ASI never creates an empty statement (`if (a)\n else` is an error) |

### 3.6 `let`

djs is always strict, so `let` is never an identifier. At statement-list-item
position, an unescaped `let` starts a lexical declaration. As a single statement
(`if (x) let y`) it is `lexical declarations can't appear in single-statement
context`. As an expression, `let` is a reserved-word error. As a property name
(`a.let`, `{ let: 1 }`) it is fine: any `is_identifier_name()` token is. `yield` and
`static` work the same way.

`await` depends on the goal: in the async body and in async functions it is the
operator; in Script mode outside async functions it is an ordinary identifier
(`var await` is valid in a Script), except in class static blocks.

### 3.7 Lvalue conversion

Expressions are emitted as loads. When an assignment operator, `++`/`--` or a
for-in/of head turns out to need a reference, `to_lvalue()` inspects the **last
emitted instruction** (the compiler tracks `last_insn_start`), removes it and returns
a reference kind:

| last instruction | reference | stack left for the store |
| --- | --- | --- |
| `name_get r` | `Name(r)` | nothing |
| `get_field atom` | `Field(atom)` | `obj` |
| `get_elem` | `Elem` | `obj key` |
| `get_private_field` (0040) | `Private` | `obj sym` |
| `get_super_field atom` / `get_super_elem` (0040) | `SuperField(atom)` / `SuperElem` | `this base` / `this base key` |
| anything else | error `invalid assignment left-hand side` (`++`: `invalid increment/decrement operand`) | |

Removing the last instruction is safe because nothing can refer to its position: no
label is placed between an operand's last load and the operator that follows it.
Line-table entries at that position are dropped too. Parenthesized simple targets
stay valid (`(a) = 1`, `(a.b) = 1`). Parenthesized patterns do not (3.3).

### 3.8 `for` heads

After `for (`, scan the balanced parenthesized head for `;` at depth 1 (QuickJS does
the same). With `;` it is a C-style `for`. Without it, it is for-in/of, which tells
the parser *before* parsing the left side that the left side is a per-iteration
assignment target. For an expression on the left, a second scan finds whether the
first `of`/`in` directly inside the parentheses is `of` (an `of` after `.` is a property
name), since for-of has to emit `get_field value` before the target. Code is emitted
in source order (as implemented in 0029, `compiler/for_in_of.rs`, D1292):

```
    jump L_rhs                ; expression heads only
L_assign:                     ; per iteration: the step result on the stack
    [get_field value]         ; for-of
    <lhs reference + store>   (parsed first, emitted here)
    jump L_body
L_rhs:
    <rhs expression>; <iterator setup>
    jump L_step
    [L_throw: for-of close handler]
L_body:
    <body>
L_step:                       ; the continue target
    <next-value logic>; jump_if_false(done) L_assign
    <drop the loop state>
```

With a declaration on the left nothing of it runs before the right side, so the store
moves next to the body and the first jump and `jump L_body` go: `<rhs>; <setup>; jump
L_step; [L_throw]; L_assign: <store>; <body>; L_step: ...`. One jump per iteration.

`for (let x of xs)` and `for (const k in o)` declare per-iteration bindings (5.9).
A pattern head (`for (const [k, v] of m)`) is skipped and parsed at the store (3.10);
an expression pattern head (`for ([a, b] of xs)`) is emitted at `L_assign` like any
expression target. `for await` is not supported yet.

### 3.9 TypeScript (0048, compiler/ts.rs)

`CompileOptions::typescript` turns the TypeScript grammar on (D1480). The type skipper
consumes types token by token and emits nothing; its rules port dquickts' parser
(which follows esbuild and tsc), so both engines accept and reject the same programs
with the same messages (`unexpected token in type: 'x'`, `unexpected end of input in
type`). All disambiguation is emission-free speculation (D1080): `ts_try` keeps the
position on success and rewinds otherwise, `ts_lookahead` always rewinds. Errors inside
speculation carry no message (`Parser::spec`).

- `expr <` where a call or instantiation may follow (`parse_lhs`, the member loop of
  `new`): try to skip a type argument list, then apply tsc's
  `canFollowTypeArgumentsInExpression` (`(`, a template, a line break, a binary
  operator or a token that cannot start an expression). Inside types `>` is never
  rescanned, so `Array<Array<T>>` closes two lists. In an expression the closing `>`
  must not be the start of `>=` or `>>` (`a < b >= c` compares).
  `Lexer::rescan_less` splits `<<`, `<=` for `f<<T>(x: T) => T>()`.
- Arrows: in TS mode a `(` that the JS arrow-head scan rejects is skipped as a balanced
  group; `=>` after it, or `:` and a return type followed by `=>`, makes it an arrow.
  Right after the `?` of a conditional (`Parser::after_q`) the return type form also
  needs a `:` after the arrow body, as in tsc (`a ? (b) : c => d` is a conditional).
  Generic arrows `<T>(x) => x` and `async <T>(x) => x` are type parameters followed by
  either form.
- `x as T`, `x satisfies T` are a postfix at relational precedence in the binary
  loop: higher operators apply to the result (`x as T * 2` is `(x as T) * 2`). The
  expression keeps its kind, so `(x as T) = 1`, `(<T>x)++` and `x! += 1` assign `x`.
- `<T>x` assertions are unary operators (the `**` rule applies); `x!` is a postfix
  in the member loop when no line break precedes it.
- Statements: `type`, `interface`, `declare ...`, `abstract class`, ambient
  `module "m" {}` and function overload signatures (a `function` header without a
  body) are recognized at the start of a statement with tsc's line break rule (`type\n
  Foo = 1` is an expression). Class members without code (index signatures, method
  overloads, `declare` and `abstract` members) are skipped by `ts_skip_class_member`
  before the element is parsed.
- Annotations: bindings (`let x!: T`), parameters (`x?: T`, patterns, rest, a leading
  `this: T`, accessibility / `readonly` / `override` modifiers), return types, type
  parameters of functions, methods, arrows and classes, `extends B<T>`,
  `implements I`, fields (`x?: T`, `x!: T`), catch parameters.

Runtime constructs (compiler/ts_decl.rs, D1482 to D1484):

- `enum` and `namespace` compile like tsc: a binding (`var` at the function level, `let`
  in a block, none if it merges with an enum, namespace, function or class of the same
  scope) and an arrow-like closure called with `N || (N = {})` (`P.N || (P.N = {})` for
  a member of namespace `P`). The closure's one parameter `<ns K>` is the object.
  A `TsDecl` record per declaration collects the members of all merged blocks
  (exported names, enum members, nested declarations). Resolution turns a free name
  in such a closure (or in a function nested in one) that is a member into
  `<ns K>.name`: `resolve::Target::Member`, a property access on the parameter
  reached like any other binding. Because resolution runs after the whole unit,
  merged blocks see each other's exports. `export let x` in a namespace declares no
  local binding: its stores go through the member.
- Enum initializers that are constant expressions (literals, members, `E.A` of other
  enums through namespaces, `+ - * / % ** | & ^ << >> >>> ~`, string `+`) are
  evaluated at compile time over tokens (speculation); others compile in the closure.
  `const enum` requires constants, and its member reads `CE.A`, `N.CE.A` are folded at
  resolution, where the binding is known (`Var::ts_decl`), so a read before the
  declaration folds too and a shadowing binding does not.
- A namespace without values (types only) discards its closure and binds nothing.
- `import X = A.B` is a `const` read like `typeof A` and optional members.
- Parameter properties (`constructor(public x)`): the names are scanned before the
  parameters; their fields are defined as `undefined` before the other fields (at
  entry in a base class, right after `super()` in a derived one) and assigned
  `this.x = x` after the parameters (after the field initializers in a derived class).

Every erased range is recorded (when `Parser::erased` is on) by the code that commits
to it, never inside speculation. `compiler::strip_types` blanks them like ts-blank-space
(one space per UTF-16 unit, line breaks kept) for `djs strip-types`, the parse-only
oracle of TESTING.md. Positions never move, so line tables and error positions refer to
the original source.

### 3.10 Destructuring patterns (0036, compiler/pattern.rs)

A pattern consumes the value on top of the stack. Binding patterns declare their names
(`var`, `let`/`const`, parameters, catch parameters, which bind like `let`); assignment
patterns store into references, evaluated before the value they receive.

**Deferred patterns.** A pattern mostly comes before its value in the source (`let [a]
= v`, `[a] = v`, `{x} = {}` as a parameter default, a nested pattern with a default,
for-of declaration heads), but its code runs after the value. The parser skips the
pattern (`scan::skip_group`), parses and emits what follows, then rewinds the lexer to
the pattern, emits it and jumps back (`Parser::deferred_pattern`, D1360). Like the other
speculation (D1080) only the lexer moves; nothing emitted is rewritten. Resolution runs
after parsing, so the initializer of `let {a} = a` still sees the new `a` in its TDZ;
the pattern's bindings get `init_end` = the end of the declarator (or parameter, or
for head).

**Object patterns** keep the value `v` (after `require_object_coercible`) and read each
property with `pick` + `get_field`/`get_elem`. A computed key goes through
`%ToPropertyKey` once, before the target is evaluated (spec order: key, target
reference, Get, default, store). With a rest property (found by a scan,
`scan::object_has_rest`) an array of the keys seen sits above `v` and
`%CopyDataPropertiesExcluding(v, keys)` builds the rest object.

**Array patterns** keep `next it`. Each element evaluates its target references
(0 to 2 values) and then steps the iterator, reaching under the references with
`pick`:

```
first step:  pick r+1; pick r+1; call 0; ...
later steps: pick r; dup; is_undefined; jump_if_true L_end; pick r+2; swap; call 0
             iter_check_object; dup; get_field done; jump_if_true L_done
             get_field value; jump L_end
L_done:      drop; undefined; poke r; undefined      ; it := undefined (done)
L_end:       <default>; <store or nested pattern>
```

A done iterator is `undefined` in its slot: later steps produce `undefined` without
calling `next`, and nothing closes it. A rest element calls `%IteratorRest(it, next)`.
After the last element a not-done iterator is closed with `%IteratorClose`. A handler
around everything but the steps (`next it exc`: `swap; dup; is_undefined;
jump_if_true L; call_intrinsic %IteratorCloseThrow 1; L: drop; throw`) closes it on an
exception from a target, a default, a store or a nested pattern; errors from the
iterator itself do not close it (D1361). Its depth is relative (`HandlerLabels::
depth_from`: the depth at the label after `swap`), because expressions do not track
their absolute stack depth; resolution finds it with `bytecode::stack_depths` on the
final code. The handler is left out when the regions cannot throw (binding names
without defaults or nested patterns).

**Tagged templates** call the tag with `this` like a call (`o.m`x``), the template
object first. `get_template_object site` finds `CONSTS[site]`: the template object once
created, before that the index of `n, cooked x n, raw x n` (cooked `undefined` for an
invalid escape). The first run builds the frozen array with its frozen, non-enumerable
`raw` and stores it in the slot: one object per site, reused by every closure of the
template (D1362).

Early errors as qjs reports them: `rest element must be the last one`, `assignment rest
property must be last`, `invalid destructuring target` (a non-reference target, a
parenthesized pattern, `eval`/`arguments` as a binding). NamedEvaluation applies to
identifier targets with anonymous function defaults, also through parentheses
(`[cover = (function () {})] = []`).

## 4. Bytecode

### 4.1 Encoding

D1082 summarizes 4.1 to 4.3.

- An instruction is a `u8` opcode followed by its immediates, little-endian,
  **unaligned**, read with bounds-checked `from_le_bytes` on subslices.
- Immediate kinds and widths:

| kind | width | used for |
| --- | --- | --- |
| `u8` | 1 | small enums (special object kind, flags) |
| `i8`, `i32` | 1, 4 | small and full integer constants |
| `u16` | 2 | local, argument and capture indexes; `argc` |
| `u32` | 4 | constant pool index, child index, name-ref index (phase 1) |
| `atom` | 4 | an atom id (stable across GC, 0005, so code never needs rewriting) |
| `label` | 4 | phase 1: a label id; final: an `i32` offset relative to the **next** instruction |

- Limits per function: 65535 locals, arguments, captures and call arguments
  (`Too many call arguments`, a SyntaxError). Constants and children are `u32`.
- P0/P1 jumps are always 5 bytes (`op` + `i32`). Short forms (`i8` jumps, `get_loc0`,
  `push_const8`, ...) are a later optimization that needs a relaxation pass, and they
  are not worth the complexity before there are benchmarks (QuickJS does them in
  `resolve_labels`).

### 4.2 One instruction table, defined once

A single macro (`bytecode/ops.rs`) declares every instruction. Everything else derives
from it:

```rust
ops! {
    /// `-> n`: an integer constant.
    PushI32 push_i32(v: I32) pop 0 push 1 cost Basic;
    GetLoc get_loc(loc: Loc) pop 0 push 1 cost Basic;
    /// `f this a1 .. an -> r`
    Call call(argc: Argc) pop 2 + imm 0 push 1 cost Call flags THROWS | CALLS | ALLOC;
    JumpIfFalse jump_if_false(target: Target) pop 1 push 0 cost Basic flags BRANCH;
    Return return_() pop 1 push 0 cost Basic flags TERM;
    NameGet name_get(r: NameRef) pop 0 push 1 cost Free flags PHASE1 | THROWS | CALLS;
    // ...
}
```

(As implemented in `bytecode/ops.rs`, 0013: variant name, emitter method name, typed
immediates, stack effect with `+ imm k` for counts, a cost class and flags. Method names
that are Rust keywords end in `_`; the disassembler name drops it.)

The macro generates:

| output | used by |
| --- | --- |
| `enum Op: u8` with `Op::from_u8`, and `const fn size(self)` | everything |
| `OPS: [OpInfo; N]`: name, size, immediate kinds, pop, push (or "pop = k + imm"), cost, flags | disassembler, stack checker, fuel |
| encoders, `fn emit_get_loc(code: &mut Vec<u8>, loc: u16)` | compiler |
| a generic decoder `decode(code, pc) -> (Op, [Imm; 2])` | disassembler, resolution re-emit, tests |
| `disasm(template) -> String` (one line per instruction, labels, names, constants) | `djs disasm`, test snapshots |
| `OPS_HASH: u32` over names, immediate kinds and order | snapshot build hash, next to 0005's `LAYOUT_HASH` |

The interpreter is a hand-written `match op` that reads immediates with typed
helpers at fixed offsets (`read_u16(code, pc + 1)`) and advances `pc += Op::X.size()`.
Agreement with the table is enforced by a debug-build check in the dispatch loop:
after every non-jump instruction, `pc` advanced by `size` and `sp` moved by `push -
pop`. The suspension stress mode runs in debug builds, so the corpus exercises the
check.

Flags: `PHASE1` (only in phase-1 code; the interpreter treats it as corrupt
bytecode), `JUMP` (unconditional, no fallthrough), `BRANCH` (conditional), `TERM`
(no fallthrough: `return`, `throw`, `throw_*`), `GOSUB`, `CALL` (may push a frame),
`REENTRANT` (may call user code through a continuation, 7.3).

### 4.3 Labels and fixups

- The emitter is `CodeBuf<Phase1>` (`CodeBuf::phase1()`) or `CodeBuf<Final>`
  (`CodeBuf::new()`), as implemented in 0013.
- Phase 1: `new_label() -> Label`. `buf.jump(label)` and the other jumps write the
  label id in the `Target` immediate. `place_label(label)` emits the `label id`
  pseudo-instruction (5 bytes, `PHASE1`). Forward and backward jumps are handled the
  same way.
- Re-emit (5.3): a `CodeBuf<Final>` created with `with_labels(phase1.label_count)`
  takes over the label ids. When the re-emit sees a `label` pseudo-op it calls
  `place_label` (recording the **final** position, emitting nothing). Jumps are
  copied with `emit_insn`, which records a fixup.
- After the re-emit: patch each jump's `i32` with `label_pos - (jump_pos + 5)`. That
  is a fixed-width, in-place patch. Exception handler regions and the line table are
  expressed with labels or phase-1 positions and are remapped the same way.
- Break/continue: the parser keeps a stack of `BlockEnv { label_name: Option<Atom>,
  break: LabelId, cont: Option<LabelId>, kind, finally: Option<LabelId>, iterator:
  bool }` (QuickJS's `BlockEnv`). `break`/`continue` walk it outward, emitting a
  `gosub` for every enclosing `finally` and an iterator close for every enclosing
  for-of they leave (7.4), and then the jump.

### 4.4 Stack depth

After the fixups, a worklist pass over the final code computes the operand stack depth
at every reachable instruction from `OPS` (pop/push, jump targets, `gosub` pushing a
return address, handler entry points starting at `handler.depth + 1`). It checks that
join points agree, which catches compiler bugs, and records the maximum in the
template (`STACK`). The interpreter reserves `locals + STACK` slots at frame entry, so
pushes inside a frame never grow the stack (7.1). This is QuickJS's
`compute_stack_size`.

### 4.5 Initial opcode list (P0 and P1)

Stack notation: `a b → c` pops `b` then `a`, pushes `c` (top of stack on the right).
"P0"/"P1" is the phase that needs the op. R = `REENTRANT` (7.3).

Literals and constants:

| op | imm | stack | notes |
| --- | --- | --- | --- |
| `undefined` `null` `push_true` `push_false` | | → v | P0 |
| `push_i8` / `push_i32` | `i8` / `i32` | → n | integer constants (int32 or f64 Values, whichever 0005/0006 settle on), P0 |
| `push_const` | `u32` | → v | numbers that are not int32, strings, P0 |
| `push_atom` | `atom` | → s | the atom's string (property names as values), P1 |
| `object` | | → o | `{}` with Object.prototype, P1 |
| `array_from` | `u16` n | v1..vn → arr | array literal without holes or spread, P1 |
| `array_push` / `array_hole` | | arr v → arr / arr → arr | general array literals, P1 |
| `array_spread` | | arr iterable → arr | R (iterator), P1 |
| `fclosure` | `u32` child | → f | create a closure, capturing boxes (6.3), P1 |
| `regexp` | | pattern flags → re | P5 |
| `get_template_object` | `u32` const | → t | tagged templates, cached per site in the constant slot (3.10), P1 |
| `special_object` | `u8` kind | → v | `arguments`, `new.target`, home object, active function (function entry only), P1 |

Stack shuffles (P0 unless noted): `drop` (a →), `dup` (a → a a), `dup2` (a b → a b a b),
`swap` (a b → b a), `nip` (a b → b), `insert2` (a b → b a b), `insert3`
(a b c → c a b c), `insert4` (a b c d → d a b c d, P1 super), `rot3l` (x a b → a b x),
`rot3r` (a b x → x a b), `pick n` (x a1..an → x a1..an x) and `poke n` (x a1..an v →
v a1..an), whose stack effects include their `u8` immediate (`pop 1 + imm 0 push 2 +
imm 0`, so the stack checker verifies the reach; 0036).

Names, phase 1 only (5.2), all with `u32` ref index:

| op | stack | meaning |
| --- | --- | --- |
| `name_get` | → v | read |
| `name_put` | v → | assign |
| `name_set` | v → v | assign, keep value |
| `name_init` | v → | initialize a declaration (no TDZ check, const allowed) |
| `name_typeof` | → v | read for `typeof` (no ReferenceError for undeclared globals) |
| `enter_scope` / `leave_scope` | | `u16` scope id: TDZ setup, boxes, block functions |
| `label` | | `u32` label id |

Resolved variable access (P0 locals/globals, P1 the rest):

| op | imm | stack | notes |
| --- | --- | --- | --- |
| `get_loc` `put_loc` `set_loc` | `u16` | → v / v → / v → v | plain locals |
| `get_loc_chk` `put_loc_chk` `set_loc_chk` | `u16` | same | TDZ check: ReferenceError `x is not initialized` (the name comes from the template's `NAMES`) |
| `set_loc_uninit` | `u16` | | store the uninitialized marker (0005 tag 6) |
| `get_arg` `put_arg` `set_arg` | `u16` | same | parameters in the argument area (7.1), P1 |
| `get_box` `put_box` `set_box` (+ `_chk`) | `u16` loc | same | local slot holds a Box; access its value, P1 |
| `new_box` | `u16` loc | | loc = new Box(uninitialized), P1 |
| `new_box_init` | `u16` loc | v → | loc = new Box(v) (captured var/param/this at entry), P1 |
| `rebox` | `u16` loc | | loc = new Box(current value): per-iteration bindings (5.9), P1 |
| `get_cap` `put_cap` `set_cap` (+ `_chk`) | `u16` | same | through the closure's capture `i` (a Box), P1 |
| `get_global` | `atom` | → v | global lexical record, then global object; ReferenceError `'x' is not defined`, P0 |
| `get_global_typeof` | `atom` | → v | `undefined` if missing, P0 |
| `put_global` | `atom` | v → | ReferenceError if missing (strict); TypeError for global const, P0 |
| `init_global_lex` | `atom` | v → | initialize a Script top-level let/const/class (5.8), P1 |
| `global_decls` | `u32` const | | Script GlobalDeclarationInstantiation from a declaration list (5.8), P1 |
| `throw_const_assign` | `atom` | | TypeError `'x' is read-only` (TERM), P0 |

Properties (P0 field access on plain objects, P1 the rest):

| op | imm | stack | notes |
| --- | --- | --- | --- |
| `get_field` | `atom` | o → v | R (getter) |
| `get_method` | `atom` | o → f o | callee and `this` for calls, R |
| `put_field` | `atom` | o v → | R (setter) |
| `get_elem` | | o k → v | R (getter, ToPropertyKey) |
| `get_elem_method` | | o k → f o | R |
| `put_elem` | | o k v → | R |
| `define_field` | `atom` | o v → o | CreateDataProperty (literals), P1 |
| `define_elem` | | o k v → o | computed keys, R (ToPropertyKey), P1 |
| `define_getter` / `define_setter` | `atom` | o f → o | P1; `_elem` variants for computed keys |
| `define_method` | `atom` | o f → o | sets the home object, P1; `define_method_elem` for computed keys |
| `copy_data_props` | `u8` flags | o src → o | object spread (mode 0; plain data properties inline, getters and strings through `%CopyDataProperties`). Object rest uses `%CopyDataPropertiesExcluding` (3.10). R, P1 |
| `set_proto` | | o v → o | literal `__proto__: v` (objects and null only; 0027) |
| `set_function_name` | `u8` prefix | key f → key f | SetFunctionName for computed keys (prefix none/"get"/"set"), R (ToPropertyKey); 0027 |
| `require_object_coercible` | | v → v | TypeError for null/undefined (`const {} = null`), P1 |
| `delete` | | o k → b | P1 |
| `in` | | k o → b | (in the binary table) |
| `private_symbol` | `atom`, `u8` flags | → sym | a new private name (7.9, D1400) |
| `get_private_field` / `put_private_field` | | o sym → v / o sym v → | own properties only, never a proxy trap; brand checks inside |
| `define_private_field` / `define_private_accessor` | `u8` kind / | o sym v → o / o sym g s → o | PrivateFieldAdd / PrivateMethodOrAccessorAdd |
| `private_in` | | sym o → b | `#x in o` |
| `get_super` | | o → proto | the super base (from the home object) or the super constructor (from the active function) |
| `get_super_field` `put_super_field` | `atom` | this base → v / this base v → | `super.x`, receiver `this` |
| `get_super_elem` `put_super_elem` | | this base k → v / this base k v → | `super[k]` |
| `set_home_object` | | home f → home f | field initializers, private methods, static blocks |
| `define_class` | `u8` flags | parent protoParent ctor → ctor proto | 7.9 |
| `define_class_method` / `_elem` | `atom`, `u8` kind / `u8` kind | o f → o / o k f → o | non-enumerable method/getter/setter, sets the home object |
| `define_class_field` | | o k v → o | CreateDataPropertyOrThrow (a proxy's trap) |

Operators (P0): `neg`, `plus` (ToNumber), `bnot` (`~`), `not` (`!`), `typeof`, `inc`,
`dec`, `to_numeric` (postfix forms), `to_string` (template substitutions), `add`
`sub` `mul` `div` `mod` `pow` `shl` `sar` `shr` `band` `bor` `bxor` `lt` `le` `gt`
`ge` `eq` `ne` `strict_eq` `strict_ne` `instanceof` `in`, `is_nullish` (v → b),
`is_undefined` (v → b, defaults). All arithmetic and comparison ops are R: their fast
path is primitives; objects take the ToPrimitive slow path (7.3). `concat` (`u16` n:
s1..sn → s) joins template literal parts in one allocation (P1).

Control flow:

| op | imm | stack | notes |
| --- | --- | --- | --- |
| `jump` | `label` | | JUMP, P0 |
| `jump_if_false` / `jump_if_true` | `label` | v → | ToBoolean, BRANCH, P0 |
| `gosub` | `label` | → ret | push the return pc (an int32 Value) and jump, P1 |
| `ret` | | ret → | jump to the popped pc, JUMP, P1 |
| `throw` | | v → | TERM, P1 (P0 needs only runtime errors) |
| `return` / `return_undef` | | v → / | TERM, P0 |
| `call` | `u16` argc | f this a1..an → r | CALL, P0 |
| `construct` | `u16` argc | f a1..an → r | `new f(...)`, new.target = f, P1 |
| `call_spread` / `construct_spread` | | f this arr → r / f arr → r | P1 |
| `super_call` / `super_call_spread` | `u16` argc / | f a1..an nt → r / f arr nt → r | derived constructors (7.9) |
| `call_intrinsic` | `u16` id, `u16` argc | a1..an → r | privileged code only (7.3), P1 |
| `get_iterator` | | o → iter | R (`%GetIterator` slow path), P1 |
| `iter_check_object` | | v → v | TypeError if not an object, P1 |
| `for_in_start` / `for_in_next` | | o → it / it → it key done | leaf (snapshot of keys), P1 |
| `iterator_close` | | iter → | R (calls `return`), P1 |
| `check_ctor` | | | class constructor called without `new`: TypeError |
| `check_this` / `check_this_uninit` | | v → v / v → | a derived constructor's `this` (7.9) |
| `check_ctor_return` | | v → v | a derived constructor's result: an object or `undefined` |
| `throw_error` | `atom`, `u8` kind | | throws a TypeError/ReferenceError (`delete super.x`), TERM |
| `debugger`, `nop` | | | |

For-of has no dedicated loop opcodes. Setup is `call_intrinsic %GetIterator 1; dup;
get_field next` (→ `iter next`; as implemented, the builtin-language `%GetIterator`
replaces the `get_iterator` op, D1290). Each step is ordinary ops: `dup2; swap; call 0; iter_check_object;
dup; get_field done; jump_if_true L; get_field value`. That reuses
the reentrancy of `call` and `get_field`, so nothing in for-of needs a resume state.
An array-iterator superinstruction is a later optimization.

P2 adds `await`, async function return/throw and promise ops. P5 adds `yield`,
`yield_star`, `initial_yield`, `regexp`.

## 5. Scopes and the resolution pass

### 5.1 The scope tree

```rust
struct Scope { parent: ScopeId, kind: ScopeKind, vars: Vec<VarId> }
enum ScopeKind { FunctionBody, Params, Block, Catch, For, Switch, Class, ScriptTop }

struct Var {
    name: Atom,
    kind: VarKind,        // Var, Let, Const, Class, FunctionDecl, Param, CatchParam,
                          // FuncName (named function expression), This, Arguments,
                          // NewTarget, HomeObject
    scope: ScopeId,       // where it is bound (for `var`: the function scope)
    decl_scope: ScopeId,  // where the declaration appeared (for var/let conflicts)
    init_end: u32,        // source offset where initialization completes (TDZ, 5.6)
    captured: bool,       // set while resolving descendants
    slot: u16,            // assigned during resolution
}
```

Declaration-time early errors: `let x` in a scope that already binds `x`, or where a
`var x` was declared in this scope or a scope inside it (`decl_scope` walk), is
`invalid redefinition of lexical identifier`. `var x` that crosses a scope with a
lexical `x` is the same error. Lookup by name during parsing uses a per-function map
`Atom → [VarId]` (an `alloc::collections::BTreeMap`; no new dependency).

### 5.2 Scope-generic names (phase 1)

Every identifier reference emits a `name_*` op with an index into `FuncState::refs`:

```rust
struct NameRef { atom: Atom, scope: ScopeId, src_pos: u32, init: bool }
```

(`init`: the reference of a `name_init`. It is never TDZ-checked and does not by
itself require the uninitialized marker at scope entry.)

`scope` is the innermost scope at the reference. `src_pos` is used for the TDZ
elision (5.6) and for errors. QuickJS puts atom and scope in the instruction
(`scope_get_var atom, scope`). The side table keeps phase-1 instructions small and
gives the resolver everything in one place.

### 5.3 Resolution: when and in what order

**[ARCH]** ARCH says each function is resolved "when that function ends". That is not
quite possible. A free name in an inner function can only be classified (outer
binding or global) once every enclosing function has seen all its declarations, and
hoisting means those can come later:

```js
function outer() { function inner() { return x } var x = 1 }
{ function f() { return y } let y = 2 }
```

D1081: resolution runs **once per compilation unit, after the whole unit is parsed**,
over `funcs` in **reverse creation order**. A child is always created after its
parent, so this visits children before parents (QuickJS does the same in
`js_create_function`) without recursion. For each function `F`:

1. **Classify each name ref**: walk `F`'s scopes from `ref.scope` outward.
   - Found in `F`: a local binding (step 2 decides its storage).
   - Not found: a free name. Walk the ancestors, starting at `F.parent` from
     `F.parent_scope`, then that function's `parent_scope`, and so on, until a
     binding is found.
     - Found in ancestor `A`: mark it `captured`, and add a capture entry to every
       function on the path from `A`'s child down to `F` (`Capture::Local(var)` in
       `A`'s child, `Capture::Outer(idx)` below it), deduplicated by
       `(function, binding)`. The ancestors are not resolved yet, so they see
       `captured = true` when their turn comes.
     - Found nowhere: global (Script top-level declarations count as global, 5.8).
2. **Assign storage** to `F`'s bindings:

   | binding | storage |
   | --- | --- |
   | captured (any kind) | a local slot holding a Box (`*_box` ops) |
   | simple parameter, not captured | its argument slot (`*_arg` ops) |
   | everything else | a local slot (`*_loc` ops) |

   Slots are numbered in declaration order. Sharing slots between sibling blocks is
   a later optimization.
3. **Re-emit** `F`'s code into a fresh buffer, in one linear pass:
   - `name_*` → the resolved op (table below), with or without a TDZ check (5.6).
   - `enter_scope s` → TDZ setup and boxes for `s`'s lexical bindings, then the
     hoisted function declarations of `s` (5.5).
   - `leave_scope s` → nothing (boxes are heap cells, so there is no `close_loc`,
     D1083).
   - `label` → record the position; jumps → fixup list (4.3).
   - Function entry gets a prologue: boxes for captured params, `this`, `arguments`
     (`special_object` + store), and `var`s.
4. Jump fixups (4.3), stack depth (4.4), line table and handler table remap, then
   write the template cell (6.1). Children were written first, so `CHILDREN` can
   refer to them.

`name_*` resolution table:

| binding storage | get | put | init | typeof |
| --- | --- | --- | --- | --- |
| local | `get_loc[_chk]` | `put_loc[_chk]` | `put_loc` | the get, then `typeof` |
| argument | `get_arg` | `put_arg` | `put_arg` | the get, then `typeof` |
| box | `get_box[_chk]` | `put_box[_chk]` | `put_box` | the get, then `typeof` |
| capture | `get_cap[_chk]` | `put_cap[_chk]` | (never) | the get, then `typeof` |
| global | `get_global` | `put_global` | `init_global_lex` | `get_global_typeof`, then `typeof` |
| const (any storage), put/set | read as above for compound ops, then `throw_const_assign` | | | |

`name_set` maps to `set_*`. For globals, `name_set` maps to `dup; put_global`.

### 5.4 Captures

- An inner function's capture list is `[Capture]`, `Local(var_slot_in_parent)` or
  `Outer(capture_idx_in_parent)`. It goes into the template (`CAPTURE_DESC`).
- `fclosure child` builds the closure: for each entry it copies the Box reference from
  the parent frame's local slot (`Local`) or from the parent closure's captures
  (`Outer`) into the new function object (6.3).
- **[ARCH confirmed, D1083]** Captured bindings live in heap Box cells from the
  moment their scope is entered. Frames never hold anything a closure points into,
  so returning, suspending and snapshotting need no `close_loc` (QuickJS's closures
  point into the stack frame until it is closed). The cost is an allocation per
  captured binding per scope entry, which also gives per-iteration bindings (5.9)
  for free.

### 5.5 Hoisting

| declaration | bound in | initialized |
| --- | --- | --- |
| `var x` | function scope (Script top: global object, 5.8) | `undefined` at function entry: frames fill locals with `undefined`; boxes via `new_box_init` |
| function declaration at function-body level | function scope | at function entry, in source order (a later duplicate wins), before any other code |
| function declaration in a block | the block (strict: lexical, no Annex B) | at block entry (`enter_scope`) |
| `let`, `const`, `class` | their block | TDZ until their declaration runs |
| parameters | the function scope | by the call (simple lists: they stay in argument slots, or a Box if captured) or by the parameter code in order (non-simple lists: locals copied from the argument slots with defaults applied, `rest` for `...r`, TDZ between them, `function f(a = b, b)` throws) |
| `catch (e)` | `Catch` scope | by the handler |
| named function expression name | its own scope outside the function scope (`special_object ACTIVE_FUNCTION`) | immutable: assignment is TypeError `'f' is read-only` |

As implemented (0024): with a non-simple parameter list, the parameter code runs in the
phase-1 code before the body's `enter_scope`, so body-level function declarations are
created after the defaults have run. Simplification: a body `var` named like a parameter
shares the parameter's binding even with parameter expressions. The spec gives it a
separate var scope initialized from the parameter (FunctionDeclarationInstantiation
step 28). The difference is only observable when a closure in a default captures the
parameter and the body reassigns the `var`.

### 5.6 TDZ checks and their elision

A check is needed for reads and writes of `let`/`const`/`class` bindings (and
non-simple parameters) that may run before initialization. The resolver **omits** the
check (and, if no checked access remains, the `set_loc_uninit` at scope entry) when
all of these hold:

1. The access is in the function that declares the binding (not via a capture).
2. `ref.src_pos >= var.init_end`: the access comes textually after the declaration
   finishes initializing (`init_end` is set when the declarator or class declaration
   ends, so `const x = x` and `class C extends C {}` keep the check).
3. The binding's scope is not a `Switch` scope. Case labels are the only jumps into
   the middle of a block. `continue`, `break`, `ret` (finally) and handlers all land
   at block starts, at positions that re-run the declaration, or outside the scope.

Accesses through captures always check, because the closure may run at any time
(hoisted function declarations called early). Box-stored bindings follow the same
rules for accesses in their own function.

### 5.7 `this`, `arguments`, `new.target`, `super`

These are pseudo-bindings (`VarKind::This`, ...) declared in every non-arrow function
scope, created only if referenced, like QuickJS. As implemented (0024), they are found by
name: `this`, `arguments` and `new.target`. `new.target` is not an identifier, so it cannot
collide with a user binding. A reference from an arrow declares the binding in the
innermost non-arrow function, and resolution then turns it into a capture. Arrow functions do not declare them,
so a reference inside an arrow resolves outward and becomes an ordinary capture. That
handles arrows' lexical `this` with no special case.

- `this`: the frame's `this` slot for normal functions, copied to a local in the
  prologue (`special_object THIS; put_loc n`, 0014). In derived constructors it is
  a local in TDZ until `super()` returns (checked access, ReferenceError `this is not
  initialized`).
- `arguments`: `special_object arguments` at entry (always unmapped, strict).
- `new.target`: `special_object new_target`. Outside functions it is an early error
  (`new.target only allowed within functions`).
- `super` property access: the home object, a binding set by `define_method`.

### 5.8 Script mode vs async-body mode (D004)

| | Script | AsyncBody (codemode) |
| --- | --- | --- |
| top-level function | a script function, `this` = global object | an async function body, called with `this` = undefined |
| top-level `var`, function declarations | global object properties, created by `global_decls` at script start (CanDeclareGlobalVar/Function checks, conflicts with global lexicals throw at instantiation) | locals of the body function |
| top-level `let`/`const`/`class` | entries in the realm's global lexical record (uninitialized at `global_decls`, set by `init_global_lex`) | locals |
| inner functions referring to top-level names | global access (another script may define or read them) | captures |
| free names | `get_global` etc.: global lexical record, then global object | same |
| `await` at top level | identifier | operator |
| `return` at top level | `return not in a function` | allowed, the result of the run |
| completion value | needed for `$262.evalScript`: a hidden local `%completion` updated by expression statements in Script mode only (later, when 0007 needs it) | no |

The global lexical record is a realm heap cell (atom → value, flags `const`), shared by
all scripts of the realm. Global accesses go through it first, so TDZ for global
`let` is a runtime check inside `get_global` (and `get_global_typeof`: `typeof x` of an
uninitialized global `let` throws). The `global_decls` declaration list is a run of
int32 constants (`bytecode::global_decls`). An assignment to any `const`, local or
global, compiles to a TDZ read (when needed) followed by `throw_const_assign`.

### 5.9 Per-iteration bindings

- `for (let i = 0; ...; i++)`: if `i` is captured, emit `rebox loc` after the init and
  at the start of each iteration's update (CreatePerIterationEnvironment). If it is
  not captured, nothing is emitted, since a plain local needs no copy. The parser emits
  the phase-1 pseudo-op `iter_scope s` at those two points, and the re-emit turns it into
  `rebox` for each captured binding of the `for` scope (0024).
- `for (const x of xs)` / `for (let k in o)`: the binding's scope is entered inside
  the loop, so `enter_scope` makes a fresh Box per iteration when it is captured.
  The scope is also entered before the right side, which sees the bindings in their
  TDZ (`for (let x of x)` is a ReferenceError); their `init_end` is the end of the head.

## 6. Function templates and heap cells

0005 (heap) and 0009 (copying GC, handles) have landed. These are sketches in their
`cells!` style, proposals for 0006 to add to `heap/cells.rs`. They build on what exists:
`Ref` fields, `Value` (NaN-boxed, tag 6 = uninitialized for TDZ), `ValueArray`,
`U32Array`, the object cell with a `SLOTS: [Value]` tail, stable `u32` atom ids, and
tracing derived from field types (D1090), so new kinds are collected automatically.
The GC runs only at explicit safepoints and invalidates raw references held in Rust
(D1091). The compiler never reaches a safepoint, so it may hold raw `Str`/atom values
for the whole compilation.

### 6.1 Cells

```rust
cells! {
    /// Immutable compiled function (one per source function).
    FuncTemplate {
        FLAGS: u32 = 8,        // kind (4 bits), async, generator, simple_params,
                               // uses_arguments, uses_this, class_ctor, derived, ...
        NAME: u32 = 12,        // atom (function `name`, stack traces)
        PARAMS: u16 = 16,      // formal parameter count (argument area padding)
        LENGTH: u16 = 18,      // `fn.length`
        LOCALS: u16 = 20,
        STACK: u16 = 22,       // max operand depth (4.4)
        CAPTURES: u16 = 24,
        RESERVED: u16 = 26,
        CODE: Ref = 28,        // Bytes: final bytecode
        CONSTS: Ref = 32,      // ValueArray: numbers, strings, template-object caches; or null
        CHILDREN: Ref = 36,    // RefArray of FuncTemplate; or null
        CAPTURE_DESC: Ref = 40,// U32Array: bit 31 = Outer, low bits = slot/index; or null
        HANDLERS: Ref = 44,    // Bytes: 16-byte (start, end, handler, depth) records; or null
        LINES: Ref = 48,       // Bytes: line table (6.2)
        SCRIPT: Ref = 52,      // Script
        SRC_START: u32 = 56,   // source span of the function (Function.prototype.toString, later)
        SRC_END: u32 = 60,
        NAMES: Ref = 64,       // U32Array of atoms: locals, then captures (TDZ messages,
                               // disassembler, debugger)
        RESERVED2: u32 = 68,
    };
    /// One per compiled source: name for stack traces, optionally the source text.
    Script { NAME: Ref = 8 /* Str */, SOURCE: Ref = 12 /* Str or null */ };
    /// A captured binding (`BoxCell` in code, so it does not shadow `alloc::boxed::Box`).
    BoxCell { VALUE: Value = 8 };
    /// Raw bytes, not traced (bytecode, line tables).
    Bytes { LEN: u32 = 8 } tail DATA: [u8] = 12;
    /// Traced references to non-JS cells.
    RefArray { LEN: u32 = 8 } tail REFS: [Ref] = 12;
}
```

Constant pool: `CONSTS` holds JS Values only (numbers that are not int32 or i8, strings
deduplicated per function during compilation, later BigInts). Atoms are immediates,
not constants. Child templates are in `CHILDREN`, so the constant pool never holds a
non-JS reference. A tagged-template site owns a `CONSTS` slot that starts
`undefined` and caches the template object on first evaluation (spec: one object per
site). This is a mutation of the constant pool, which snapshots capture like
everything else.

### 6.2 Line table

LEB128 triples `(pc_delta, line_delta (zigzag), column)` per position change, starting
at line 1, column 1. Lines and columns come from `lexer::LineTable` (D1032), built
once per compilation, so stack traces and runtime errors do not need the source text. The parser
records a position before emitting an instruction that can throw (calls, property
access, operators, `throw`, TDZ checks). The anchor token (call `(`, operator,
property name) follows qjs and is tuned against the oracle by the difftests.

### 6.3 Closures

A function object is an object cell (class Function) with `SLOTS[0]` = its template
and `SLOTS[1..=CAPTURES]` = the Box references. `prototype`, `name` and `length`
are ordinary properties. The interpreter's `fclosure` creates them from the template:
`NAME`, `LENGTH`, and `prototype` for `KIND_NORMAL` (D1240). The compiler sets `NAME` by
NamedEvaluation where the name is static (`let f = () => 1`, `f = function () {}`,
`{ f: () => 1 }`, `(f = () => 1) =>` defaults).

**Request to 0005 / coordinator:** object `SLOTS` hold `Value`s, but templates and
Boxes are not JS values. Proposal: a reference tag for internal cells (for example
tag 13 `CELL`, traced like other references, never visible to JS). The alternative is
a parallel `Ref` tail on function objects, which complicates the object layout. A
dedicated tag is simpler and is also useful for frames (7.1) and generator state
(P5).

## 7. Frames and the interpreter contract

### 7.1 The value stack and frame records

D1086 summarizes this section.

The value stack is a growable `ValueArray` in the arena (`Roots::STACK`), addressed by
**index**, so growing it (allocate and copy) moves nothing the interpreter holds.
Frame layout:

```
bp      callee            the function object (or host function)
bp+1    this
bp+2    arg0 .. arg(n-1)  n = max(argc, PARAMS), missing args padded with undefined
hp      prev_bp           int32 Value     (hp = bp + 2 + n)
hp+1    ret_pc            int32: caller pc to resume (after the call instruction)
hp+2    argc              int32: actual argument count (for `arguments`)
hp+3    flags             int32: continuation kind (7.3), construct, ...
hp+4    new_target        Value
lp      local0 .. local(L-1)       lp = hp + 5, filled with undefined
sb      operand stack (≤ STACK)    sb = lp + L
```

- Header fields are int32 Values, so the GC traces the whole stack array from 0 to
  `sp` with no frame knowledge. Snapshots need no frame serializer.
- Interpreter registers (`bp`, `hp`, `sp`, `pc`, cached code offset) are Rust locals
  in the dispatch loop. They are flushed to an `Interp` state cell (in the roots)
  whenever `run` returns. The cached code offset is re-derived from
  `stack[bp] → SLOTS[0] → CODE` after every safepoint, because a copying GC may move
  the code cell.
- A call reserves `5 + LOCALS + STACK` slots once, at frame entry, and checks the
  stack limit there (qjs: InternalError `stack overflow`). Pushes within the frame
  need no capacity check.

### 7.2 Calls and returns

- `call argc`: `[f, this, args]` are already on the stack in the right place, so they
  become the new frame's `bp..` with **no copying**. For a JS closure: pad args, write
  the header, init locals, `pc = 0`, switch registers. Native leaf builtins (Rust)
  run inline and push their result. Host functions: 7.5.
- `return`: take the value, read the header, set `sp = bp`, restore the caller
  registers from `prev_bp` / `ret_pc`, then apply the continuation (7.3).
- `construct`: allocates `this` (base) before entering the frame, or leaves it
  uninitialized (derived).

### 7.3 Continuations and re-entrant ops

**[ARCH, sharpened: D1085]** ARCH allows "a small numbered resume state" per
multi-step op. We avoid it entirely: **an op never resumes in the middle.** A
re-entrant op does its fast path inline (pure leaf code). If it needs user code, it
pushes a call frame and finishes immediately; the callee's return value completes the
op through a fixed continuation kind in the callee's header:

| continuation | on return |
| --- | --- |
| `PushResult` | push the value on the caller's operand stack (calls, getters, slow-path builtins) |
| `DiscardResult` | drop it (setters) |
| `ConstructResult` | if not an object, use the frame's `this` (and check derived-ctor rules) |
| `Entry` | the bottom frame of a `vm_call`: `run` returns `Done(value)` |

Examples:
- `get_field` finds an accessor: pop `obj`, push `[getter, obj]`, call with
  `PushResult`. The caller's `ret_pc` is the next instruction.
- `put_field` finds a setter: pop `obj v`, push `[setter, obj, v]`, `DiscardResult`.
- `add` with an object operand, `ToPropertyKey` of an object key, `get_iterator` on a
  non-array, `instanceof` with `Symbol.hasInstance`: call a **privileged JS builtin**
  (`%Add(a, b)`, `%ToPropertyKey(k)`, `%GetIterator(o)`, ...) with `PushResult`. The
  multi-step logic (valueOf, then toString, then the operation) lives in JS, where
  suspension is free.

So the interpreter needs no per-op state machines, and the leaf rule holds: Rust only
runs fast paths.

As implemented (0027): `CompileOptions { privileged: true }`
(`CompileOptions::privileged_script`) enables `%Name(args)`. The id is the index into
`bytecode::intrinsics::INTRINSICS` (D1270), and the compiler checks names and arities.

**[ARCH timing]** This needs privileged JS builtins already in P1 (getters, ToPrimitive
on objects: the test262 harness uses `toString` all over). ARCH plans the builtin
language (bgen) for P2. Proposal: P1 adds a minimal **privileged mode** in the compiler
(`%Name(args)` intrinsic calls in primary position, where `%` cannot otherwise start
an expression, compiled to `call_intrinsic`) and a small hand-written `prelude.js`
compiled at VM creation. bgen later produces the same kind of code, and the base
snapshot replaces compile-at-startup.

### 7.4 Exceptions

- Each template has a handler table of `(start, end, handler, depth)` entries
  (phase-1 labels, remapped). Entries are appended when a `try` block closes, so inner
  regions come first, and the first entry that covers the pc is the innermost.
- Throw at `pc` in a frame: find an entry with `start <= pc < end`. Found: `sp = sb +
  depth`, push the exception, `pc = handler`. Not found: pop the frame and continue in
  the caller with key `ret_pc - 1` (a byte inside the call instruction, so the covering
  region is the one around the call). The `Entry` frame with no handler: `run` returns
  `Threw(value)`.
- `finally` uses `gosub`/`ret` (QuickJS). The finally block is compiled once and is
  always entered at the same operand depth: the try's base depth plus **one value
  slot**, plus the return address. So the stack checker sees one depth at its entry:
  - normal exit: `undefined; gosub L_finally; drop`;
  - `return v`: unwind the operand stack to the try's base keeping `v` on top (close
    and `nip` any for-of iterators in between), then `gosub` each enclosing finally,
    innermost first, re-basing between levels the same way, then `return`;
  - `break`/`continue` leaving it: unwind to the base, `undefined; gosub; drop` for
    each finally left, then `jump`;
  - exception: the handler entry for the try (and catch) region has depth = base, so
    the exception is the value slot: `gosub L_finally; throw`.

  The return address is an int32 Value on the operand stack, so it suspends and
  snapshots like any value.
- As implemented (0025): whether a `try` has a `finally` is found by a token scan
  before the try block is parsed (`scan::try_has_finally`), because `break`, `continue`
  and `return` inside it must emit the `gosub`s (D1080). A `Finally` entry on the
  `BlockEnv` stack records the finally label and the try's base depth. The parser
  tracks the operand depth at statement boundaries (`stmt_depth`, 2 inside a finally
  block), and jumps out of a finally block drop the value slot and the return address.
  The catch parameter and the catch block's declarations share one `Catch` scope, so
  `catch (e) { let e }` is a redefinition. A `var e` there is allowed (as in qjs, Annex
  B.3.4 behavior) and assigns the parameter.
- Iterator close on abrupt exit from for-of: `break`/`return` emit `iterator_close`
  before leaving; a throw goes through a handler entry around the loop body that
  calls `iterator_close` (ignoring errors from `return()` per spec) and rethrows.
- As implemented (0029, D1292): the close is `call_intrinsic %IteratorClose` and the
  handler calls `%IteratorCloseThrow` (`it next exc`: `nip; swap; call_intrinsic
  %IteratorCloseThrow 1; drop; throw`). The handler covers the target store and the
  body, not the `next`/`done`/`value` steps. A `break` of the loop itself jumps to an
  inline close after the loop. Jumps that leave the loop (`break L`, `continue L`,
  `return`) cannot close inline: the close would sit inside the loop's own handler
  region, and a throwing `return()` would close the iterator again. They `gosub` a close
  subroutine placed after the loop instead, found on the `BlockEnv` stack as an `Iter`
  entry and run in innermost-first order together with the finally blocks. It is
  entered on `v it next ret` (`v` the return value or `undefined`, as a finally block's
  value slot) and, since `gosub` keeps the caller's depth, leaves `v it undefined` for
  the caller to drop (`rot3r; drop; dup; call_intrinsic %IteratorClose 1; rot3l; ret`).
- A handler entry may have a depth relative to a label (`depth_from`, array
  destructuring inside expressions, 3.10): resolution computes it from the final code
  with `bytecode::stack_depths`, repeating while a region sits in code that only
  another such handler reaches.
- **[ARCH: no change]** Differs from QuickJS, which pushes catch offsets onto the
  value stack and scans the stack while unwinding. A table keeps the stack plain.

### 7.5 Host calls

Calling a host function object (`fn_id`): the op copies the arguments into a new
entry of the pending host call table (heap), pops `[f, this, args]`, saves the
registers with `pc` after the call instruction, records the `call_id` in the
`Interp` cell, and `run` returns `HostCall(call_id, fn_id, args)`. The frame is
parked exactly as for a JS call that has not returned yet.
`vm_resolve_host_call(call_id, Ok(v))` pushes `v` (as `PushResult` would). `Err(e)`
throws `e` at the call instruction (handler lookup with `pc - 1`). The next `run`
continues. Async host functions (D1374) instead return a promise immediately and never
park: the call goes into the same table with its promise, the embedder reads it with
`Vm::new_host_calls`, and `resolve_host_call` queues a job that settles the promise.

### 7.6 Fuel and safepoints

- Every instruction subtracts its `cost` (from `OPS`) from the fuel counter in a Rust
  register. At zero, the loop flushes the registers and returns `OutOfFuel` at that
  instruction boundary. Leaf ops charge extra in proportion to their work (copying,
  key enumeration).
- GC safepoints: instruction boundaries where `gc_pending` is set, checked together
  with fuel at backward jumps, calls and returns. All live values are on the value
  stack or in heap cells, so the collector needs no interpreter cooperation beyond
  the register flush and the code-offset refresh.

### 7.7 Generators and async functions (0037, D1375)

The runtime side is in `interp/run/suspend.rs` and `builtins/generator.ts`; the tests on
hand-assembled bytecode in `vm/tests.rs` show complete functions. A suspension copies
the frame's slice `stack[bp..sp)` into a heap Generator object (with the resume pc and
`hp - bp`) and resuming copies it back on top of the resumer, so frames need nothing
special: locals, arguments, `finally` return addresses and for-of iterators survive as
they are. Every op names the hidden local that holds the Generator object (a `Loc`
immediate); the compiler reserves it (call it `%gen` / `%async`).

| op | stack | does |
| --- | --- | --- |
| `generator_start loc` | `->` | first instruction of a generator function: creates the generator object (prototype: the function's own `prototype` if an object, else %GeneratorPrototype%), stores it in `loc`, suspends, and returns it to the caller. `next()` resumes at the following instruction. |
| `yield loc` | `v -> received isReturn` | suspends; `next()`/`return()`/`throw()` get `v`. Resumed by `next(x)`: `x false`; by `return(x)`: `x true`; by `throw(e)`: `e` is thrown at the `yield` (handler lookup with the yield's pc). |
| `yield_star loc` | `innerResult -> received isReturn` | sync generators only (0042): `yield` for `yield*`, whose `next()`/`return()`/`throw()` return `innerResult` itself (GeneratorYield ( innerResult ): no `value` read, no new result object). Resumes like `yield`. |
| `async_start loc` | `->` | first instruction of an async function (and of an async codemode body): creates the async state with its result promise in `loc`. The body runs on synchronously. |
| `await loc` | `v -> value` | suspends until `v` settles (PromiseResolve, then a reaction): pushes the value, or throws the reason at the `await`. The first `await` returns the result promise to the caller. |
| `async_return loc` | `v ->` | resolves the result promise with `v` (thenables adopted) and returns the promise. TERM. |
| `async_reject loc` | `e ->` | rejects the result promise with `e` and returns the promise. TERM. |

What the compiler emits:

- **Generator functions** (`func_flags::GENERATOR`; their `prototype` property is made by
  `fclosure`, inheriting from %GeneratorPrototype%):
  ```
  generator_start %gen          ; after the parameters' initialization (spec order:
                                ; FunctionDeclarationInstantiation runs before the
                                ; generator object exists, so default-parameter errors
                                ; throw at the call)
  ...
  <v> yield %gen                ; yield v
  jump_if_true L_ret            ; return(x) was called: x is on the stack
  ...                           ; the received value is the yield expression's value
  L_ret: <return the value: the ordinary `return` sequence with its finally gosubs>
  ```
  `return v` in a generator is the ordinary `return`; the value becomes `{value: v, done:
  true}` (the builtins tell a yield from a return by the generator's state). An
  exception leaving the body completes the generator. `yield*` is left to the compiler
  (a loop over `next`/`throw`/`return` of the inner iterator around `yield`).
- **Async functions and arrows** (`func_flags::ASYNC`, also `KIND_ASYNC_BODY`): the whole
  body sits in a catch-all handler region whose handler (depth 0) is `async_reject`:
  ```
  async_start %async
  L_s: ... <v> await %async ...      ; await v
       <v> async_return %async       ; every return (after its finally gosubs), and
                                     ; `undefined; async_return` at the end
  L_e:
  L_h: async_reject %async           ; handler (L_s, L_e, L_h, depth 0)
  ```
  Parameter initialization goes inside the region (errors there reject the promise). The
  caller always gets the promise, whatever the body does.
- **Async generators** (`ASYNC|GENERATOR`, 0042, D1421) use the generator ops plus
  `await` on the same local; `generator_start` makes an async generator when the
  function has the ASYNC bit (prototype: its own `prototype`, else
  %AsyncGeneratorPrototype%). The runtime keeps the request queue and settles the
  promises of `next`/`return`/`throw`; the body only yields, awaits and returns:
  ```
  generator_start %gen
  ...
  <v> await %gen; yield %gen          ; yield v: the operand is awaited first
  jump_if_false L_next
  await %gen                          ; return(x): x is awaited, then returned
  <the ordinary return sequence>
  L_next: ...                         ; the received value
  <v> await %gen; return              ; return v: awaited, then the ordinary return
  ```
  `throw(e)` throws at the `yield`. An exception leaving the body rejects the pending
  request (no catch-all handler, unlike async functions); falling off the end returns
  `undefined` as usual (`return_undef`, nothing to await). `yield*` in an async
  generator iterates the inner async iterator with `await`s around `next`/`throw`/
  `return` and yields `value` with plain `yield` (no `yield_star`; spec: the inner
  result's value is not awaited again, so skip the operand `await` there).
- **`for await (x of o)`** in async functions and async generators (`%state` is the
  async state or the generator):
  ```
  <o> call_intrinsic %GetAsyncIterator 1        ; it (sync iterables are wrapped, D1421)
  dup; get_field next                           ; it next, as for-of
  loop: <it next> call 0 (this = it); await %state; iter_check_object
        dup; get_field done; jump_if_true L_done; get_field value; <bind x>; <body>
  ```
  Leaving the loop early (`break`, `return`, `continue L` outward) closes the iterator
  (AsyncIteratorClose, normal completion):
  ```
  <it> call_intrinsic %AsyncIteratorReturnMethod 1   ; return method or undefined
  dup; is_undefined; jump_if_true L_skip
  <it> call 0 (this = it); await %state; iter_check_object
  L_skip: drop
  ```
  An exception leaving the body closes it with the throw completion:
  `<it> call_intrinsic %AsyncIteratorCloseThrow 1; await %state; drop; throw` (the
  intrinsic gives a promise that always fulfills: errors from `return()` are ignored).
  qjs's message for a non-iterable is `not a function`.

### 7.8 What the compiler emits for async functions and generators (0041)

- **Flags and the state local.** `FuncState::is_async`/`is_generator` become the template's
  `ASYNC`/`GENERATOR` bits on top of the kind (D1420). An async or generator function
  gets a `VarKind::Internal` local (`<state>`, never captured); phase-1 code emits the
  state ops with a placeholder `Loc` and resolution rewrites it to the local's slot
  (D1412). The codemode body is an async function too (D1410).
- **Async functions** (`start_flavor`, `end_async_region` in compiler/function.rs):
  `async_start` before the parameters, every `return` and the end of the body
  `async_return` (`emit_return_value`/`emit_return_undef`), the catch-all region pushed
  last (outermost) by `end_function`.
- **Generators**: `generator_start` at the start of `parse_function_body`, after the
  parameters. `yield` (compiler/generator.rs) is parsed at the AssignmentExpression level;
  it has no operand before `) ] } , ; :`, the end of input or a line break.
- **Return from a resumed `yield`** (D1411). The operand stack at a `yield` holds the
  enclosing expression's operands, which the parser does not count, and the finally
  blocks of the return sequence run at statement depth. So the return path throws the
  value in a one-instruction handler region whose handler has the statement depth
  (`stmt_depth`) and runs the ordinary return sequence (`emit_unwind` + return). Before
  that, each open iterator (`Parser::open_iters`: array patterns being destructured,
  for-of heads being stored) gets the same treatment with its own depth (relative to the
  pattern's base label, or absolute for for-of), and closes its iterator (IteratorClose,
  or AsyncIteratorClose for `for await`). These regions are inserted at the front of the
  handler list, as they are the innermost. Async generators `await` a `return(x)` value
  first.
- **`yield*`** keeps `it next` on the stack: `received` → `next.call(it, received)` →
  (await) → `iter_check_object` → `done`? the `value` is the result : `yield_star` (sync)
  or `get_field value; yield` (async). A `return(x)` resumption calls `it.return`, a
  `throw(e)` (thrown at the yield, caught by a region around it whose depth is relative
  to the `it next` label) calls `it.throw`, or closes the iterator and throws `iterator
  does not have a throw method`.
- **`for await`** is the for-of layout (compiler/for_in_of.rs) with `%GetAsyncIterator`,
  an `await` after each `next()` call, `%AsyncIteratorCloseThrow` + `await` in the throw
  close handler and AsyncIteratorClose (`emit_async_close_normal`) in the break/return
  paths.

### 7.9 Classes (0040, compiler/class.rs)

- **Scopes.** A class compiles in its enclosing function, in two nested `Class` scopes:
  the outer one holds the inner class binding (const, in TDZ until the class is
  defined); the inner one holds the private names (`#x`, one binding each, holding a
  private symbol made by `private_symbol` at scope entry, D1400) and the hidden
  bindings: `<fields N>` (the instance initializer, N = class nesting depth, so a
  `super()` in a nested class's computed key still finds its own), `<key N>`
  (converted computed field keys) and `<#m>` / `<get #m>` / `<set #m>` (private method
  closures). The heritage is evaluated in the outer scope only, so its private name
  references belong to the enclosing class. Undeclared private names are collected per
  class and checked at its end (passed outward).
- **Definition.** `[heritage; dup; call_intrinsic %ClassHeritage 1 | undefined;
  undefined]; fclosure ctor; define_class flags` leaves `ctor proto`. The constructor's
  child index is reserved first (it may appear anywhere in the body, or is synthesized
  at the end); its source span is the whole class. Methods go on `proto` (static ones
  and computed keys of static elements: `swap; ...; swap`). At the end: `.fields` =
  the instance initializer closure (`set_home_object` to `proto`) or `undefined`; drop
  `proto`; the inner binding; the static initializer is called with `this` = the class
  (home object the class).
- **Initializers.** Instance and static field initializers each compile into one
  `FieldInit` function, parked while the class body is parsed and resumed per field
  (`Parser::resume_function` / `park_function`). Each starts with a jump to its end,
  where the private methods and accessors are installed once all are known, then jumps
  back: PrivateMethodOrAccessorAdd before the fields. A static block is its own
  `StaticBlock` function (its own var scope), created and called by the static
  initializer at its position. `arguments` in either is an early error; `await` is
  reserved in static blocks.
- **Constructors.** `check_ctor` first. A base constructor then calls `.fields` with
  `this` (a runtime `undefined` check), before the parameters. A derived constructor
  gets no `this` from `construct` (it is `undefined`, read through `check_this`);
  `super(args)` is `<active>; get_super; args; new.target; super_call n`, then binds
  `this` (`check_this_uninit`) and calls `.fields`. Its returns store the value in a
  local and jump to an epilogue after the body, outside every try region (the result
  check belongs to [[Construct]]): `check_ctor_return`, and `undefined` → the checked
  `this`. Default constructors: `{}`, or `super(...args)` with `rest 0;
  super_call_spread` (no iteration).
- **Home objects.** A function that uses `super.x` (itself or in arrows) declares the
  `<home>` pseudo-binding (`special_object HOME_OBJECT`); its template gets
  `func_flags::HOME`, and its closures a home slot after the captures, set by
  `define_class`, `define_class_method`, `define_method`/`getter`/`setter` (object
  literals too) and `set_home_object`. `super.x` is `this; <home>; get_super;
  get_super_field x` (MakeSuperPropertyReference order: `this`, then the base, then the
  key); `super.m()` reads `this` again for the call.

## 8. Error reporting and early errors

### 8.1 Compile errors

```rust
enum CompileError {
    Syntax { pos: u32, message: Cow<'static, str> },  // or String for formatted ones
    OutOfMemory,
}
```

- `pos` is a byte offset, converted to `line:col` with `LineTable` only when the error
  is reported. The embedding API creates a `SyntaxError` with the message and, in
  its stack, `at <filename>:<line>:<col>`, as qjs does. Lexer errors (`LexError`)
  convert directly.
- Positions follow QuickJS: usually the start of the current token; lexer errors use
  the lexer's position; "unexpected token" errors point at the token.
- The parser stops at the first error (no recovery).
- Messages use QuickJS wording verbatim (TESTING.md: matching is cheap here). QuickJS
  words some errors differently by position (2.5), so difftest cases pin them. Messages
  verified against qjs while writing this note:

| situation | message |
| --- | --- |
| nesting limit | `stack overflow` |
| unexpected token | `unexpected token in expression: '%s'`, `expecting '%c'`, `expecting ';'` |
| bad assignment target | `invalid assignment left-hand side` |
| `let x; let x` / `let x; { var x }` | `invalid redefinition of lexical identifier` |
| `{ var x; let x }` at top level | `invalid redefinition of global identifier` |
| `delete x` | `cannot delete a direct reference in strict mode` |
| `var eval` / `eval = 1` | `invalid variable name in strict mode` / `invalid lvalue in strict mode` |
| `var let`, `var yield`, `var \u0069f` | `variable name expected` |
| `-a ** 2` | `unparenthesized unary expression can't appear on the left-hand side of '**'` |
| `throw` + newline | `line terminator not allowed after throw` |
| duplicate params | `duplicate argument names not allowed in this context` |
| `"use strict"` + non-simple params | `"use strict" not allowed in function with default or destructuring parameter` |
| `({__proto__: 1, __proto__: 2})` | `duplicate __proto__ property name` |
| `return` outside functions (Script) | `return not in a function` |
| `break` / `continue` misuse | `break must be inside loop or switch`, `continue must be inside loop`, `break/continue label not found` |
| `super` misuse | `'super' is only valid in a method` |
| `new.target` outside functions | `new.target only allowed within functions` |
| duplicate constructor | `property constructor appears more than once` |
| `#constructor` | `invalid method name` |
| `label: label:` | `duplicate label name` |
| `yield` as identifier | `unexpected 'yield' keyword` |
| `await`/`yield` in parameters | `await in default expression`, `yield in default expression` |
| `for await (x in y)` | `'for await' loop should be used with 'of'` |
| runtime: `yield*` with a `throw()` call and no `throw` method | TypeError `iterator does not have a throw method` |
| `if (x) let y` | `lexical declarations can't appear in single-statement context` |
| getter/setter arity | `invalid number of arguments for getter or setter` |
| `((a)) => 1` | `missing formal parameter` |
| runtime: TDZ | ReferenceError `x is not initialized` |
| runtime: const assignment | TypeError `'x' is read-only` |
| runtime: undeclared global | ReferenceError `'x' is not defined` |
| runtime: assignment to a named function expression's name | TypeError `'f' is read-only` |
| runtime: `this` before `super()` | ReferenceError `this is not initialized` |
| runtime: class constructor without `new` | TypeError `class constructors must be invoked with 'new'` |
| runtime: call of a non-function | TypeError `not a function` |
| runtime: property of null/undefined | TypeError `cannot read property 'x' of null` |
| runtime: JS stack limit | InternalError `stack overflow` |

### 8.2 Early-error coverage strategy

- Early errors are implemented where the information is: declaration conflicts at
  declaration time (5.1), assignment targets at lvalue conversion, label and
  break/continue checks via the `BlockEnv` stack, function-kind rules (`await`,
  `yield`, `super`, `new.target`) via a per-function context, class rules in class
  parsing. Nothing waits for the resolution pass, except that resolution itself never
  fails.
- test262 measures it. The runner (0002/0007) gets a `--parse-only` mode: compile each
  test without running it. Negative `phase: parse` tests must fail with SyntaxError,
  and every other active test must compile. Both are cheap enough for the gate and
  feed the ratchet, so early-error regressions show up immediately.
- `phase: resolution` tests are module-only (skipped). Regex early errors wait for the
  regex compiler (0011); until then those tests are expected failures.
- One P1 issue (10) walks the negative-test failure clusters after the language
  features land.

## 9. Worked examples

### 9.1 Closure over a `let`

```js
function counter() { let n = 0; return () => ++n; }
```

Phase 1:

```
counter:                         arrow:
  enter_scope s1   ; {n}           name_get r0     ; n @ s_arrow
  push_i8 0                        inc
  name_init r0     ; n @ s1        name_set r1     ; n @ s_arrow
  fclosure 0                       return
  return
  leave_scope s1   ; unreachable: the stack checker only walks reachable code,
  return_undef     ; and dead-code removal is a later optimization
```

(Name refs are numbered per function: `counter`'s `r0` and the arrow's `r0` are
different entries.)

Resolution (the arrow first): `n` is not in the arrow, so it is found in `counter`
(captured). The arrow gets `captures = [Local(n)]`, and `n` becomes a Box in
`counter`'s `loc0`. The arrow's accesses are captures, so they keep TDZ checks, so
`counter` needs the uninitialized Box:

```
counter:                         arrow:
  new_box loc0                     get_cap_chk 0
  push_i8 0                        inc
  put_box loc0                     set_cap_chk 0
  fclosure 0                       return
  return
  return_undef
```

### 9.2 `if`/`else` and short-circuit

```js
if (a && b) f(); else g();
```

```
  get_global a                    ; Script, a and b free
  dup
  jump_if_false L1
  drop
  get_global b
L1:
  jump_if_false L_else
  get_global f; undefined; call 0; drop
  jump L_end
L_else:
  get_global g; undefined; call 0; drop
L_end:
```

(A later peephole pass can turn `&&` in a condition into direct jumps.)

### 9.3 `try`/`finally` with `return`

```js
function f() { try { return g(); } finally { h(); } }
```

```
L_try:                              handlers: [L_try, L_try_end) → L_exc, depth 0
  get_global g; undefined; call 0   ; [v]       v is the value slot
  gosub L_fin                       ; [v ret] → [v]
  return
L_try_end:                          ; (no normal exit: the block always returns)
L_exc:                              ; [exc]     the exception is the value slot
  gosub L_fin
  throw
L_fin:                              ; [x ret]   depth 2 on every path
  get_global h; undefined; call 0; drop
  ret
```

## 10. Suggested issue split for P0 and P1

P0 (replaces the single 0006, whose acceptance criteria can be taken from here):

| id | title | depends on | scope |
| --- | --- | --- | --- |
| 0006a | Bytecode table and tools | 0005 (atoms) | `ops!` macro, encoders, decoder, disassembler, stack-depth checker, `OPS_HASH`; tests on hand-built code |
| 0006b | P0 compiler | 0003, 0004, 0005, 0006a | parser + emitter for expressions and statements without functions (literals, operators, `if`, loops, blocks, `let`/`const`/`var` at top level and in blocks), phase-1 names, resolution for locals and globals, labels, line table, templates in the heap, SyntaxErrors with positions |
| 0006c | P0 interpreter and embedding | 0005, 0006a | value stack, frames, dispatch loop, fuel, `Interp` cell, globals, `console.log` as a sync `HostCall`, `vm_new`/`vm_compile`/`vm_call`/`vm_run`, `djs run`. It can start in parallel with 0006b using hand-assembled bytecode |
| 0006d | djs in the difftest runner | 0006b, 0006c, 0010 | run 0010's corpus on djs, in the gate |

P1:

| # | title | depends on | scope |
| --- | --- | --- | --- |
| P1-1 | Functions and closures | 0006b, 0006c | declarations, expressions, arrows, calls/`new`, `return`, captures and Boxes, `this`/`arguments`/`new.target`, default and rest params, named function expressions |
| P1-2 | Objects, arrays, privileged prelude | P1-1 | literals (computed keys, shorthand, methods, accessors, spread), property ops, `delete`/`in`, getters/setters through continuations, privileged mode + `prelude.js` + `call_intrinsic`, ToPrimitive slow paths |
| P1-3 | Exceptions | 0006b, 0006c | `throw`, `try`/`catch`/`finally`, handler tables, `gosub`/`ret`, Error objects with stack traces from the line table |
| P1-4 | Lexical scoping | P1-1 | TDZ (checks + elision), block functions, `switch`, per-iteration bindings, redeclaration early errors |
| P1-5 | Loops and iteration | P1-2, P1-3 | labels, `break`/`continue` through `finally`, `for-in`, `for-of` + iterator protocol + close, array fast path |
| P1-6 | Templates, spread, destructuring | P1-2, P1-5 | template literals (cooked/raw, tagged, site cache), call/array spread, rest, binding and assignment patterns with defaults (the 3.3 scan) |
| P1-7 | Classes | P1-1, P1-2, P1-4 | methods, accessors, static members, fields, private names and brand checks, static blocks, `super`, derived constructors |
| P1-8 | Script mode globals | 0006b, 0006c | `global_decls`, the global lexical record, `init_global_lex`, completion value, `$262.evalScript` hook (feeds 0007) |
| P1-9 | GC in the interpreter | 0006c, 0009 | safepoints at back edges/calls/returns, register flush and code-offset refresh, a collect-every-N stress mode |
| P1-10 | Early-error sweep | 0007, P1-1..P1-7 | `--parse-only` runner mode, walk test262 negative-test clusters |

## 11. Deviations from ARCH and open questions

Proposed ARCH edits:

1. **Technique 2 (rewind):** replace "emit, and on failure truncate back to the
   checkpoint" with "speculate without emitting; a checkpoint is the lexer state plus
   the current token" (D1080). The only emitted-code rewrite is the last-instruction
   lvalue conversion.
2. **Technique 3 (resolution):** "runs once per compilation unit after parsing, over
   all functions, children before parents" instead of "when that function ends"
   (D1081, 5.3).
3. **Interpreter, re-entrant operations:** drop "a small numbered resume state is
   recorded on the frame for multi-step ops". Ops never resume mid-way; slow paths
   call privileged JS builtins with one of a few fixed continuations (D1085).
4. **Phases:** a minimal privileged mode and a hand-written JS prelude move into P1
   (7.3). bgen in P2 then replaces the hand-written prelude.
5. **Exceptions** (new paragraph): handler tables and `gosub`/`ret` for `finally`
   (D1084).

Open questions for the coordinator:

- **Internal-reference Value tag** (6.3): needed for closures' template and Box
  references in object slots. 0005 owns the Value encoding.
- **Async-body `this`** and whether codemode top-level declarations should ever be
  globals: dquickts' own async eval (`JS_EVAL_FLAG_ASYNC`) uses global-eval semantics
  (top-level `var` becomes a global), but our difftest harness wraps cases in an async
  function (TESTING.md), so this note follows ARCH (locals). Worth confirming against
  pi's actual codemode wrapper.
- **Source retention** for `Function.prototype.toString` (keep `Script::SOURCE` or
  not): memory vs conformance. The template records the span either way.
