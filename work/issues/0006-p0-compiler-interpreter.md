# 0006: P0 compiler + interpreter: expressions, statements, fuel, console.log

status: open
agent: -
branch: issue/0006-p0-compiler-interpreter
depends-on: 0003, 0004, 0005
touches: crates/djs-core/src/{compiler,bytecode,interp,vm}/, crates/djs-cli/, tests/corpus/

## Goal
ARCH.md phase P0: the single-pass compiler emitting bytecode for expressions and
statements, a stackless interpreter loop with fuel, `console.log` as a sync host call,
the embedding API, `djs run`, and the first differential corpus with its runner in the
gate.

(Detailed acceptance criteria are written by the coordinator once 0003–0005 have landed.)
