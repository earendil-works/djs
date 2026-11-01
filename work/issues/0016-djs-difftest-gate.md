# 0016: djs in the difftest runner and the gate

status: done
agent: infra
branch: issue/0016-djs-difftest-gate
depends-on: 0010, 0022
touches: crates/djs-difftest/, crates/djs-bench/ (engine track: register djs), tests/corpus/expected-failures.txt, scripts/gate.sh

## Goal
docs/compiler.md section 10, row 0006d. Run 0010's corpus on djs in-process (through
`Vm::compile` in async-body mode with `tools`/`console` parameters, D008, after running
the shared prelude), check in the ratchet file, and put difftest (djs, plus
`--suspend-every` and `--gc-stress` variants) into the gate. Also register djs in
djs-bench's engine track (0019 follow-up) and report the djs/qjs ratios.

## Acceptance criteria
- [ ] A `djs` engine in djs-difftest: in-process, a fresh `Vm` per case, the prelude run
      first (it will mostly fail today because builtins are missing: decide whether
      the prelude must be split into a minimal part djs can run now, or whether the djs
      engine provides `console`/`text`/`exit` natively as host functions for now;
      record the decision), the fuel limit from the case metadata, host calls for
      console/text/store/load/tools, the return value and uncaught error formatted
      exactly like the qjs side.
- [ ] Stress variants: `--suspend-every N` (heap-image round trips) and `--gc-stress`.
      Outputs must be identical to the plain djs run (that is a hard failure, separate
      from the oracle comparison).
- [ ] `tests/corpus/expected-failures.txt` generated (most cases fail now: that is
      expected), and the ratchet in `scripts/gate.sh` for the plain run plus a
      consistency-only check for the stress variants. Keep the gate's added time small
      (parallelism; GC stress maybe on a subset if slow; record the timings).
- [ ] The djs engine in djs-bench's engine track, with a djs/qjs ratio column (most
      benchmarks will not run yet; report what runs).
- [ ] A Log entry with the pass count and the biggest failure clusters (grouped by first
      error line), so the coordinator can prioritize.

## Log

2026-11-01 (infra)
- Prelude decision (D1160): djs cannot run tests/corpus/prelude.js yet, so the djs engine
  provides the codemode globals as host functions with the prelude's exact formatting.
  `Vm::display` is not that format: it prints `-0`, `NaN` and `function` where pi's JSON
  rules give `0`, `null` and the source.
  - A test (`djs_glue_matches_qjs`) runs 14 cases djs can already run on qjs (real
    prelude) and on djs (native globals) and requires identical transcripts.
  - Known differences until promises exist: `tools.*` return directly, `tools.fail`
    throws at the call.
- djs engine (`crates/djs-difftest/src/djs.rs`):
  - a fresh `Vm` per case, `Vm::compile` in async-body mode with `tools`/`console`,
    `this` = global;
  - fuel from the case directive (default 2e10), checked against the wall clock every
    10M fuel;
  - compile errors as `SyntaxError: msg`; Idle as pi's "can never settle" error.
  No TypeScript mode exists yet, so `.ts` cases fail to compile.
- Stress variants (D1161): `--suspend-every N` and `--gc-stress` run each case again; a
  differing transcript is a hard failure. All 154 cases agree, even at N=1. Cost grows with
  work done: a scratch case with 20k objects takes 15 s under GC stress and times out at
  N=7 (one image copy per slice). The gate uses N=97: 0.27 s in debug today.
- `--clusters` groups failures by the first line of the uncaught error; ratchet reasons use
  the same key (`throws ReferenceError: 'Object' is not defined`).
- Result on main (f235cdd + this): **50 of 154 run cases pass** (11 skipped as eval/Function
  non-goals). By area: result 15/23, errors 11/23, dquickts 23/83, codemode 1/8, console
  0/12, ts 0/5. The error and return value paths already match qjs's formatting and
  messages wherever djs has the feature.
- Biggest failure clusters (cases | first line of what djs throws):
  | 11 | throws ReferenceError: 'Object' is not defined |
  | 10 | throws SyntaxError: await is not supported yet |
  | 9 | throws SyntaxError: expecting ';' |
  | 8 | throws SyntaxError: classes are not supported yet |
  | 7 | throws SyntaxError: methods are not supported yet |
  | 6 | throws SyntaxError: BigInt literals are not supported yet |
  | 4 | throws SyntaxError: for-in and for-of loops are not supported yet |
  | 4 | throws SyntaxError: generators are not supported yet |
  | 4 | throws SyntaxError: variable name expected |
  | 3 | throws ReferenceError: 'Date' is not defined |
  | 3 | throws SyntaxError: unexpected token in expression: 'enum' |
  | 3 | throws TypeError: not a function |
  | 2 | throws ReferenceError: 'Array' is not defined |
  | 2 | throws ReferenceError: 'Error' is not defined |
  | 2 | throws ReferenceError: 'Symbol' is not defined |
  | 2 | throws SyntaxError: async functions are not supported yet |
  | 2 | throws SyntaxError: expecting '(' |
  | 2 | throws SyntaxError: getters, setters and methods are not supported yet |
  | 2 | throws SyntaxError: regular expressions are not supported yet |
  | 1 | throws InternalError: not supported yet: array_spread |
  Themes, for prioritizing:
  - Compiler features, about 50 cases: await (10), classes (8), methods/getters (9),
    BigInt literals (6), for-in/of (4), generators (4), async functions (2), regex
    literals (2), destructuring (2), tagged templates, array spread, delete of an
    optional chain.
  - Missing globals, about 28 cases: Object (11), Date (3), Array, Error, Symbol (2 each),
    JSON, Math, Number, Promise, String, TypeError, Uint8Array, parseInt (1 each).
  - TypeScript (P3), about 21 cases: `expecting ';'`, `variable name expected`, `enum`,
    `interface`, decorators, ...
  - Missing methods (`not a function`), 3 cases: String.prototype.repeat,
    Array.prototype.sort, push.
- djs-bench engine track: djs added, with `engine/djs÷qjs/<script>` ratios (unit `x`).
  3 of 9 scripts run on djs: calls 9.0x, loop 8.5x, props 4.0x slower than qjs (native
  release, run time minus start-up). The others need Math, String, regex or missing
  Array methods. djs start-up (compile and run of an empty case) is 6 µs, against
  qjs's 1.9 ms process start.
- TESTING.md: added three lines and a paragraph to "How to run difftest" (not in
  `touches:`, but it is the difftest section).

## Follow-ups
- Switch the djs engine to the shared prelude once djs runs it (JSON, Promise,
  Object.freeze, Map, classes); then remove the native glue (D1160).
- Async: once promises and `await` exist, `tools.*` must return promises like pi's.
- Revisit `--suspend-every 97` and full GC stress in the gate as passing cases grow (D1161).
- Stale ratchet entries are reported, not failed: agents fixing djs should run
  `djs-difftest --engine djs --update-ratchet` in their branch (worth a note in AGENTS.md or
  the board).
- Record a bench baseline with the djs engine (`scripts/bench.sh --record engine`) on an
  idle machine.

## Result
- `djs-difftest --engine djs` (in-process, native codemode globals) with
  `--suspend-every N`, `--gc-stress` (consistency is a hard failure) and `--clusters`.
- `tests/corpus/expected-failures.txt`: 104 known failures; 50 of 154 run cases pass.
- The gate runs `--engine djs --ratchet --suspend-every 97 --gc-stress` (0.27 s).
- djs-bench: djs in the engine track with djs/qjs ratios.
- Decisions D1160, D1161; usage in TESTING.md.
