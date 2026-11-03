# 0165: Tool results stored once: sliced strings over result buffers

status: open
agent: -
branch: issue/0165-result-buffers
depends-on: 0152, 0158, 0162 item 3 (D2551)
guide: docs/guide/strings.md (or values/heap), snapshots.md
touches: crates/djs-core/src/heap/ (a result-buffer cell kind, sliced string representation), crates/djs-core/src/vm/ (host result path), crates/djs-core/src/builtins/ + json (parse into slices), image/session writers (buffers in snapshots), js/src/ (host side, if buffers are referenced instead of stored), docs/decisions.md; coordinate: speed (0162, JSON stringify output, host results), bloat (0151, heap/ templates and atoms), strings (0163, string hashing)

## Goal
docs/lessons-from-pico.md lesson 3: pico stores a tool result once, as a buffer tagged with
the call id and a hash; strings derived from it are slices over that buffer, and snapshots
list the buffer instead of storing it (a 5 MB log costs 0.44 MB in an image instead of
5.44 MB). djs's peak memory on pico's log and parallel-call cases is 2.2–2.5x pico's.

Design and build the djs version:
- a result-buffer cell (host text kept once, immutable), slices (substring/split/JSON
  string values) referencing it without copying, with the GC keeping a buffer alive while
  any slice is (and a policy to copy out small slices of huge dead buffers);
- JSON.parse of a tool result producing slices for string values where profitable;
- snapshots: buffers written once; optionally referenced by (call id, hash) so a host that
  journals results doesn't store them twice (a container option; D024: no compat code);
- determinism and canonical_hash unaffected by the representation;
- measure peak memory (mem-perturb) on pico's log / parallel-call cases, the workloads and
  the pi bench, image sizes, and instructions (no workload slower than +1% without a trade).
Start with the design note in the issue log (sent to the coordinator) before building.

## Log

## Follow-ups

## Result
