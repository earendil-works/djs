# 0247: The hook benchmark on djs-embed: per-call costs for Gondolin

status: in-progress
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
- [ ] `cargo run --release -p djs-embed --example hooks-bench -- 10000` (or similar) prints the
      table
- [ ] results in the issue and a short section in docs/guide/rust-api.md
- [ ] perf fixes with measurements

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

## Follow-ups

## Result
