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

## Follow-ups

## Result
