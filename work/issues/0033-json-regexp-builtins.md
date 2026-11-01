# 0033: JSON and RegExp builtins (+ regex literals end to end)

status: in-progress
agent: num
branch: issue/0033-json-regexp-builtins
depends-on: 0011, 0012, 0031 (part 1: bgen forms)
touches: builtins/json.ts, builtins/regexp.ts, builtins/string-regexp.ts (new files), builtins/intrinsics.d.ts (additions), builtins/gen/, crates/djs-core/src/builtins/ (new modules json/regexp), crates/djs-core/src/regexp/ (VM hookup), crates/djs-core/src/bytecode/intrinsics.rs (regenerated), crates/djs-core/src/compiler/ (regex literal emission only, minimal), crates/djs-core/src/interp/ (a `regexp` op if needed, minimal and additive), ratchets

## Goal
Hook the leaf engines built earlier (0011 regex, 0012 JSON) into the VM as builtins,
following 0031's pattern (builtin-language wrappers around leaf intrinsics):

- `JSON.parse` (the leaf parse; the reviver in the builtin language) and
  `JSON.stringify` (`stringify_fast` first, and the full algorithm with
  replacer/`toJSON`/accessors/wrapper objects/gap in the builtin language when it says
  `Slow`).
- RegExp objects: the constructor, `lastIndex`, flags getters, `exec`, `test`,
  `toString`, `[Symbol.match/matchAll/replace/search/split]`, `RegExp.prototype.source`
  and flags, the match result arrays with `index`/`input`/`groups`/`indices` (the `d`
  flag). Compiled programs stored in a heap cell (pointer-free, as 0011 designed).
  Suspended exec states in the arena (0011's `ExecState`), so a long match is
  fuel-metered and suspendable through the interpreter.
- Regex literals compile (currently "not supported yet" in the compiler): early errors
  from the 0011 parser at compile time, and a fresh RegExp object per evaluation.
- `String.prototype.{match, matchAll, replace, replaceAll, search, split}`, including
  replacement patterns (`$1`, `$<name>`, `$&`, …) and function replacers. Coordinate with
  infra (0031 part 2: String wrappers and primitive lookup) via the coordinator; this
  issue owns only these six methods.

## Acceptance criteria
- [ ] All of the above, with qjs messages.
- [ ] A `StackOverflow` from the matcher becomes a JS error (check qjs's behavior).
- [ ] Fuel: a catastrophic pattern from JS (`/(a*)*b/.test("a".repeat(30))`) runs out of
      fuel (a `timeout` in test262/difftest terms) and is suspendable mid-match: a
      heap-image round trip while suspended gives the same result.
- [ ] difftest cases (tests/corpus/builtins/json-*.js, regexp-*.js), e2e cases in the
      three modes, and the ratchets updated. test262 numbers for built-ins/JSON,
      built-ins/RegExp (much of it is in the later group: report both), and
      built-ins/String/prototype/{match,replace,split,search}.
- [ ] Startup: `Math.random`-like per-VM state isn't involved, but check the base size
      growth (0032 base snapshot) and report it.

## Notes
- 0029 (lexer) is in compiler/. Your compiler change is a small regex-literal emission:
  log it and keep it minimal.
- 0030 (heap) is in interp/; 0031 (infra) is in builtins/ and the realm. Keep to new
  files where possible, and regenerate the intrinsic table on rebase.

## Log

### Side task (coordinator): base image vs. djs-core
- The "Function" instead of "function" seen in djs-cli output came from the agent tool
  display, which capitalizes the first character of a command's output (`echo function`
  prints `Function`; `od -c` shows the real bytes). It was not an engine bug and the base
  was not stale.
- Checked: cargo reruns djs-prelude's build script whenever djs-core is rebuilt (`cargo
  build -v` shows `Dirty djs-prelude: the dependency djs-core was rebuilt`, and then the
  build script runs). After swapping two well-known atoms, the incremental build made a
  new base and the output was right.
- The build hash already covered the well-known atoms and symbols (in LAYOUT_HASH). It did
  not cover realm slot order, object class ids, flag bits or the prelude. Now
  djs-core/build.rs hashes the non-test sources and prelude.js into BUILD_HASH (D1330).

### Part 1: JSON
- builtins/json.ts (JSON object, parse + reviver, stringify: fast leaf first, then the
  full algorithm), builtins/json.rs leaves, interp/run/json_regexp.rs (my intrinsics are
  dispatched from the `_` arm of `call_intrinsic`, so the big match there is untouched
  apart from one line). D1331.
- Intrinsic table regenerated with work/scratch/0033/regen-table.py (mirrors the
  table_matches_sources test).
- Tests: tests/corpus/builtins/json-{parse,stringify}.js (identical to qjs, also with
  --suspend-every 97 --gc-stress), tests/data/e2e/json.js (three modes).
- test262 built-ins/JSON active: 117/119 (the two left need String.prototype.split and
  regex literals: part 2). Later group: 44 (Proxy, BigInt, json-parse-with-source).
  Whole active run: 16,355 → 16,542 passing (ratchet −189 +2 changed reasons).
- difftest: test_json.js and test_optional_chaining.js now get further (new reasons in
  the ratchet). console/{tojson,objects,numbers-in-json}.js still fail: the difftest
  harness's console glue (crates/djs-difftest/src/djs.rs `format`) calls
  `stringify_fast` from the host and prints a marker on Slow.
- Base snapshot (debug build): 351,696 → 373,488 bytes (+21.8 KB for json.ts).

## Follow-ups
- Host console glue (djs-difftest `format`/`output_text`, and the Sandbox in js/) formats
  objects with the host-side fast stringify only. For toJSON, accessors or wrappers it
  should run JSON.stringify in the VM (or the prelude should format console arguments).
- JSON.parse source text access (reviver `{ source }`, JSON.rawJSON/isRawJSON): test262
  "later" (json-parse-with-source), qjs has it.

## Result
