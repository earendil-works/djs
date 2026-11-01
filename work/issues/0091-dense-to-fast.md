# 0091: Move dense-array leaves to @fast entries; fromCharCode/URI @fast

status: done
agent: infra
branch: issue/0091-dense-to-fast
depends-on: 0085, 0090
touches: crates/djs-core/src/builtins/fast.rs (adding entries), crates/djs-core/src/interp/run/dense.rs, builtins/ (tags), ratchets

## Goal
After perf's 0085 (`@fast`, D1850) lands: move infra's dense.rs paths (push/pop/indexOf/
lastIndexOf/includes/slice/join/fill) to frameless @fast entries (push+pop is still ~4x
qjs because of the two native frames), and add @fast entries for String.fromCharCode and
the URI functions (the debug decodeURI test262 cases take 3.9 s in harness call overhead).

## Acceptance criteria
- [x] Instruction counts before/after (push+pop per pair, indexOf/includes per call, the
      decodeURI tests), both backends identical, ratchets.

## Log
- builtins/dense.rs: D1801's loops at heap level (search, slice, join, push, fill);
  interp/run/dense.rs keeps the leaf wrappers (fuel per element) and Object.values/
  entries / TypedArraySetFromDense. `@fast` entries in builtins/fast.rs (D1910).
- Gap from 0090: an entry at the heap cap declined→ builtin path retries. One arm in
  fastpath.rs `try_fast_call` (perf's file; told the coordinator). Test
  `fast_entries_decline_at_the_cap` with scripts/determinism/js/oom_fast.js: caps
  2.6–3.5 MB printed `.EEEEEEE` when entries threw, all dots now; identical sliced/
  restored.
- Numbers in D1910. decodeURI A2.5_T1 in the debug runner: 3.9 s → 1.75 s (incl. the
  runner start). Corpus case builtins/fast-entries-array.js (each entry and its
  declining cases: valueOf arguments, non-arrays, holes, symbols, malformed URIs).
- work/README.md: run scripts/regen.sh after every manual rebase (the 0090 lesson).

## Follow-ups

## Result
- `@fast` entries for Array push(1)/pop/indexOf/lastIndexOf/includes/slice/join/fill,
  String.fromCharCode and the four URI functions; the loops are shared with the
  builtin-language leaves (builtins/dense.rs).
- `@fast` entries decline at the heap cap (D1910), so D1700/D1900's retry applies.
- Instruction counts in D1910: push+pop wasm 10.6G → 5.8G (qjs-wasi 4.7G), indexof
  1.04G → 0.73G, strings 6.0G → 3.3G (better than qjs-wasi), arrays 18.2G → 13.4G.
- Known gaps: push with several items and pop on holey arrays take the builtin; the
  remaining push/pop cost is the call itself (get_field + call + fast hook).
