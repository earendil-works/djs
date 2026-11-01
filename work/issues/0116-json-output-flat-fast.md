# 0116: JSON output written in place; flat_fast inline; next pi-like hot spots

status: done
agent: perf
branch: issue/0116-json-output-flat-fast
depends-on: 0110
guide: docs/guide/builtins.md (if the JSON or string paths it describes change)
touches: crates/djs-core/src/json/, crates/djs-core/src/heap/string.rs (flat_fast, cleared), crates/djs-core/src/builtins/ (json leaves)

## Goal
0110's follow-ups: JSON (~9% of pi-like) builds its output in a Rust Vec and copies it into
the arena again (write it in place, as the 0012 follow-up suggested); `flat_fast` is still an
out-of-line call in wasm (~0.9%); then re-profile pi-like and take the next hot spots that
aren't the GC (heap's, after 0101).

## Acceptance criteria
- [x] wasm instruction counts before/after (pi-like, json, jsonh, json-*), size, no behavior
      change (the json corpus, stress, determinism).

## Log
- New workload scripts/perf/w/json-text.js: pi-like's read(), a record with a multi-line
  text round-tripped through JSON.
- Profiles (wasm cpu profile with callers, native samply, lldb traces of
  work/scratch/0116/mi.py micro cases):
  - stringify's output Vec and its final copy are small next to allocations made per
    call: a Vec per cached key, a temporary output per key miss, and growth from an
    empty Vec;
  - parse: an allocation per distinct key per parse (the key cache), and escapes decoded
    unit by unit;
  - `includes("throw")`: the first-byte scan flagged most positions of a line.
- Landed (D2250), wasm Minstr (min of 3):

  | change | effect |
  | --- | --- |
  | parse: escaped runs + source-range key cache | json-text −5.6%, pi-like −3.4% (with the next row) |
  | stringify: key buffer, keys quoted in place, 256-byte start, `quote` reserve | json-indent −13% |
  | `find_latin1`: first and last byte together | strings −10%, pi-like −0.8% |
  | `flat_fast` inline(always) | pi-like −0.3% |

- Tried and dropped:
  - an 8-byte skip in the JSON string scan, in four variants: −9% on long strings but
    +1–6% on short ones;
  - `extend_from_slice` for short runs in wasm: +1% on json;
  - writing the output in place: see D2250.
- json and jsonh moved ±1–2% between measurement batches (the base build too), so for
  them I compared within one batch and checked native counts. Final native:
  json +0.3%, jsonh +0.5% (the key cache array init and code layout), within the gate's
  noise.
- Final vs main 87954447 (wasm, min of 3; full table in work/scratch/0116/wm-e12.txt):
  - pi-like 8,594 → 8,221 (−4.3%); json-indent −13.3%; strings −9.8%;
    json-text −6.4%; indexof −1.0%;
  - everything else within ±0.4%.
  - Size: −0.5 KB.
- Tests: the string search test is wider (n ≤ 90, needles ≤ 6). It caught a missing
  first/last check on candidates. New `string_tokens_agree_across_encodings` (escapes,
  quotes, controls, non-ASCII, bad escapes, across the three lexers). JSON test262,
  unit, e2e (3 modes), difftest (both backends, stress, determinism) and the active
  group are unchanged.
- Guide: builtins.md doesn't describe these paths; none affected.

## Follow-ups
- Next non-GC hot spots of pi-like (wasm):
  - run_hot 6.5% (user code);
  - JSON parse `next` 3%, `quote` 3%, `write_escaped_latin1` ~2%;
  - `alloc_latin1_pieces` 2.3% (LINES.slice().join: the copy itself);
  - the natives' call protocol (`n_call`, `n_slot`) ~3.5%.
  - The GC (≈11%) is still the biggest.
- A one-pass JSON string lexer for the long-string case (scan and write together),
  without costing short strings: the 8-byte skip couldn't do it.

## Result
JSON parse and stringify no longer allocate per key or per call beyond the result.
Escaped strings are written by runs. The one-byte search filters on the first and last
byte. `flat_fast` is inline (D2250).
- wasm: pi-like −4.3%, json-indent −13%, strings −10%, json-text −6%, nothing slower;
  −0.5 KB.
- An 8-byte skip in the JSON string scan and writing the output in place were
  measured, or reasoned, and dropped.
