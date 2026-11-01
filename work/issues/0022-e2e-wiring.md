# 0022: End-to-end wiring: Vm::compile, `djs run`, e2e tests

status: open
agent: -
branch: issue/0022-e2e-wiring
depends-on: 0014, 0015
touches: crates/djs-core/src/vm/, crates/djs-cli/, crates/djs-core/tests/ (or an e2e test module)

## Goal
The 0015 follow-ups: connect the compiler (0014) to the interpreter (0015) so that JS
source runs end to end.

## Acceptance criteria
- [ ] `Vm::compile(source, mode)` on top of `compiler::compile`, for Script mode and
      the async-body mode (D008; the parameters `tools` and `console`. Until async
      exists, run the body as a plain function body and document the gap).
- [ ] `djs run file.js` in djs-cli main.rs, with `--gc-stress` and `--suspend-every N`,
      printing `console.log` output through the host call. Exit codes: 0 ok, 1 uncaught
      error (printed as `Name: message`), 2 usage or compile error.
- [ ] End-to-end tests: a set of small JS programs covering P0 (and whatever P1 the
      interpreter already supports through the compiler), each run in the three modes
      (normal, fuel 1 with a heap-image round trip at every stop, GC stress), with
      identical output required.
- [ ] Fix whatever mismatches show up between compiler output and interpreter
      expectations, and log them.

## Log

## Follow-ups

## Result
