# 0091: Move dense-array leaves to @fast entries; fromCharCode/URI @fast

status: open
agent: -
branch: issue/0091-dense-to-fast
depends-on: 0085, 0090
touches: crates/djs-core/src/builtins/fast.rs (adding entries), crates/djs-core/src/interp/run/dense.rs, builtins/ (tags), ratchets

## Goal
After perf's 0085 (`@fast`, D1850) lands: move infra's dense.rs paths (push/pop/indexOf/
lastIndexOf/includes/slice/join/fill) to frameless @fast entries (push+pop is still ~4x
qjs because of the two native frames), and add @fast entries for String.fromCharCode and
the URI functions (the debug decodeURI test262 cases take 3.9 s in harness call overhead).

## Acceptance criteria
- [ ] Instruction counts before/after (push+pop per pair, indexOf/includes per call, the
      decodeURI tests), both backends identical, ratchets.

## Log

## Follow-ups

## Result
