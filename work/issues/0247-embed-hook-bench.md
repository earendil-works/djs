# 0247: The hook benchmark on djs-embed: per-call costs for Gondolin

status: done
agent: embed
branch: issue/0247-embed-hook-bench
depends-on: 0240
touches: crates/djs-embed (examples/ or benches/, the shim and policies copied from Gondolin's prototype), crates/djs-core (only for performance fixes found), docs/guide/rust-api.md (a short costs section), tests/scores/status.tsv (only if recorded through scripts/status.py)
guide: docs/guide/rust-api.md

## Goal

Port `~/Development/gondolin/native/experiments/djs-hooks/src/bin/bench.rs` (and the parts of
`src/lib.rs` and `src/shim.js` it needs, plus `policies/*.js`) to djs-embed, inside djs, with
no Gondolin crates and no third-party crates (std threads instead of tokio for the 8-caller
column; the host's request head as plain data; the body as host bytes behind an async host
function; the decision as plain data out, not JSON). Report per-call p50/p99 and calls/s for:
empty policy, header-only allowlist (fresh VM and reset/pooled), 64 KiB body as text and as
bytes (unused), 64 KiB body policy (fresh and pooled), plus fuel per call, compile time and
snapshot size. Compare with the research's numbers (djs-hooks.md §4: header-only 26 µs pooled /
33 µs fresh, 64 KiB body 20 µs pooled / 35 µs fresh, empty 3.5 µs) on this machine at low load
(note the load average; the research ran at load ~10–18).

Then fix what the numbers show is the embedding's own cost (instantiate/reset, the head going
in, the decision coming out, bytes and strings in), within djs-embed and small djs-core changes;
record each with before/after numbers.

## Acceptance criteria
- [x] `cargo run --release -p djs-embed --example hooks-bench -- 10000` (or similar) prints the
      table
- [x] results in the issue and a short section in docs/guide/rust-api.md
- [x] perf fixes with measurements

## Log

- 2026-11-07 embed: ported the bench to `crates/djs-embed/examples/hooks-bench/` (main.rs,
  shim.js adapted: no `__settle`, the entry's settled value is the decision, replaced bytes
  leave as a Uint8Array; policies copied unchanged). URL parsing: a small absolute-URL parser
  in the example (no `url` crate). Modes fresh / reset (pool + `reset` after each call) /
  pooled; native Rust rows; no node tier-3 rows (needs Gondolin crates). 8 callers = 8 std
  threads sharing the hooks. `probe <policy> <mode> <calls> [data]` loops one row for
  `sample` / `/usr/bin/time`.
- Load was 90-240 all session (other agents). Wall-clock latencies swing 2x run to run, so
  before/after numbers are CPU time (`/usr/bin/time -p`, user+sys / calls), alternating the
  two binaries 5 times and taking each one's minimum: work/scratch/0247/ab.sh.
- Profile (sample, noop pooled, before fixes): interpreter (the shim) ~47%, `start` 32%
  (head in: Data -> wire 9.5%, wire decode 17%), decision out (`to_data`: encode + from_wire)
  8%, host Data building/dropping ~10%. Header-only pooled: interpreter 75%, head in 10%,
  decision out 7%, URL parse reply (Data -> wire -> decode) 6%. Body pooled: `includes`
  26%, minor GC (mark_old) 12%, host `from_utf8` of 64 KiB 7%, copy in 6%; the harness's
  body copy 5%. Reset ~8% of a header-only call.
- Fix 1 (5e57cae5): `Value::Data` builds in the heap directly (explicit stack, same protos,
  int numbers, holes, depth limit as the decoder), `to_data` builds the tree from the
  outbound encoder through a new `wire::Sink` trait (djs-core, `embed` only; Writer is one
  Sink). The to_data cap now counts the tree's host memory (32 B a node), not wire bytes.
  CPU/call: noop pooled 5.10 -> 3.67 us (-28%), header-only pooled 38.0 -> 34.5 (-9%),
  64 KiB body pooled 28.0 -> 24.3 (-13%).
- Fix 2 (32cc7522): `Value::Array(&[Value])` / `Value::Object(&[(&str, Value)])`: plain data
  from borrowed parts (the head without a Data tree); the bench moves the decision's
  strings out of the Data. noop 3.70 -> 3.47 (-6%), header-only 40.3 -> 35.5 (-12%, noisy).
- Fix 3 (4bc0234d, djs-core): `canonical_numeric_index` built every key's String
  (`atom_name`) and every IC addition fill asks it: 4% of a noop call on a reset instance
  (ICs cold each call). Now it rejects by the first unit. noop reset 8.57 -> 8.20 (-4%).
  Wasm sizes: shipped 426,780 gzip (ceiling 426,806), slim 360,398 (ceiling 360,485).
- Fix 4 (0256a396): `Value::Utf8(&[u8])`: text from bytes, lossy, one pass over ASCII (the
  host's `from_utf8` before the VM's ASCII check was a second pass). 64 KiB as text pooled
  17.8 -> 16.0 (-10%), 64 KiB body policy 27.8 -> 27.0 (-3%).
- Fix 5 (6dc207e0): `Instant::now()` only for calls with a deadline (~1% of a noop call).
- Fresh and reset are dominated by copying the image: memmove is 39% of a noop call on a
  reset instance and 29% on a fresh one (237 KiB image; ~3 us at this load). Not cold ICs:
  an experiment (`Instance::to_snapshot` after a warm-up call, patch in
  work/scratch/0247/warm-experiment.patch) gave no measurable gain, so it was dropped.
- Not embedding costs, left alone: the shim's interpretation (private fields, async
  wrappers: ~60% of a noop call), minor GCs in pooled instances walk the whole old region
  (no write barrier; 12% of a pooled 64 KiB body call), atom lookups of the head's keys
  (~4% of noop), `global_path` per call (~1%).

- Guide: docs/guide/rust-api.md gained a "Costs" section and the borrowed values in
  "Values"; no other page affected. Decisions D3225 (values without a wire buffer, borrowed
  values in, `Value::Utf8`) and D3226 (where the time goes, what was not done).

## Follow-ups
- Engine, not embedding: a minor collection walks the whole old region (no write barrier),
  so a long-lived instance that takes 64 KiB bodies spends ~12% of a call in GC. A card table
  or remembered set for the sealed snapshot region would make that proportional to what a
  call wrote, and would also let `reset` copy back only what changed (now a full memcpy of
  the image, ~30-40% of an empty call on a fresh/reset instance).
- Gondolin's shim (copied unchanged in spirit): the empty policy is ~60% interpretation of
  the shim (private fields, `async` wrappers, a `Request` and `Headers` per call). A leaner
  shim (plain fields, a sync fast path when the policy returns synchronously) is Gondolin's
  call; the bench is where to measure it.
- Atom lookups of a head's keys (`method`, `url`, `headers`) are ~4% of an empty call;
  `global_path` of the entry ~1%. A per-instance cache would need invalidation on reset.
- The table's wall-clock columns are unreliable on a loaded machine (calls/s for 1 caller
  is dominated by preemption tails). Rerun `hooks-bench 10000` at load < 4 for numbers to
  quote outside this issue.

## Result
`crates/djs-embed/examples/hooks-bench/` (main.rs, shim.js, policies/): Gondolin's prototype
bench on djs-embed with no Gondolin or third-party crates. `cargo run --release -p djs-embed
--example hooks-bench -- 10000` prints sanity decisions, the fail-closed cases, compile time
(6.4 ms), snapshot size (237 KiB) and the table; `hooks-bench probe <policy>
<fresh|reset|pooled> <calls> [data]` loops one row for profiling.

Recorded 2026-11-07 19:42 at load 33-52 (the lowest this session; the research ran at
10-18), release build, Apple M-series, each row best of 3 rounds; the second p50 column is
the same binary before the fixes (5e57cae5^), run right after:

| engine | hook | p50 µs | p50 before | p99 µs | fuel/call | research p50 |
|---|---|---:|---:|---:|---:|---:|
| native Rust | header-only | 1.0 | 1.2 | 1.2 | – | 0.6 |
| djs pooled | empty policy | 3.7 | 6.0 | 6.8 | 312 | 3.5 |
| djs fresh | header-only | 43.1 | 49.8 | 107 | 5,763 | 33.0 |
| djs reset | header-only | 42.4 | 45.4 | 321 | 5,763 | – |
| djs pooled | header-only | 37.4 | 43.3 | 250 | 5,763 | 26.2 |
| djs pooled | header-only, head as a Data tree | 35.0 | – | 323 | 5,763 | – |
| native Rust | 64 KiB body | 23.8 | 21.7 | 89 | – | 18.9 |
| djs pooled | 64 KiB as text, unused | 12.5 | 15.4 | 77 | 1,132 | 15.4 |
| djs pooled | 64 KiB as bytes, unused | 14.7 | 15.7 | 56 | 1,501 | 15.3 |
| djs fresh | 64 KiB body | 36.7 | 31.9 | 107 | 1,158 | 35.1 |
| djs reset | 64 KiB body | 27.4 | 35.2 | 2390 | 1,158 | – |
| djs pooled | 64 KiB body | 26.2 | 29.4 | 154 | 1,158 | 20.3 |

8-caller rates (best of 3): empty 558k/s, header-only pooled 88k/s, fresh 117k/s, body pooled
90k/s; at this load they mostly measure the scheduler. The wall-clock rows still swing ±20%
between runs, so the fixes were judged by CPU time per call (Log): empty policy -32%,
header-only pooled ~-13%, 64 KiB text -10%, body -16%, reset -4% (IC fill). Header-only is
interpreter-bound (75%); its gap to the research's 26 µs is the load (the native rows are
1.3-1.7x the research's too).

Landed in djs-embed: `Value::Data` built in the heap directly; `to_data` built from the
encoder (`wire::Sink`, djs-core `embed` only) with its cap counting tree memory;
`Value::Array` / `Value::Object` (borrowed plain data), `Value::Utf8` (text from bytes,
lossy); the clock read only for calls with a deadline. In djs-core:
`canonical_numeric_index` without a `String` per key (shipped module 426,780 gzip, slim
360,398, both a little smaller). Guide: rust-api.md "Costs". Known gaps: Follow-ups.
