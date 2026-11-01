# 0044: djs runs the shared difftest prelude (retire D1160's native console)

status: open
agent: -
branch: issue/0044-difftest-shared-prelude
depends-on: 0033 (JSON, done), 0041 (await), 0040 (classes, if the prelude uses them)
touches: crates/djs-difftest/, tests/corpus/prelude.js (only if a djs incompatibility must be worked around, with a comment), js/src/sandbox.ts (console formatting, if it has the same issue), ratchets

## Goal
D1160 had djs provide `console`/`text`/`exit`/… natively, because djs could not run
tests/corpus/prelude.js. With JSON (0033 part 1), Promise (0037) and await (0041), it
should be able to now. Running the same prelude on both engines is what makes difftest
compare exactly what the model sees (D1100). The native glue only uses the host-side
`stringify_fast`, so `console/{tojson,objects,numbers-in-json}.js` fail (num, 0033).

## Acceptance criteria
- [ ] The djs engine runs the shared prelude and the case like the qjs engine does; the
      native host functions remain only for the primitives the prelude itself needs from
      the host (output, store/load, fake tools), now as async host functions where pi's
      are async (`tools.*`).
- [ ] The D1160 known difference (`tools.*` returning directly) is gone.
- [ ] Ratchet updated: report how many cases changed (both directions) and why.
- [ ] Same for the Sandbox if it formats console output on the host side (check pi's
      prelude behavior, D1100).

## Log

## Follow-ups

## Result
