# 0095: Documentation coherence pass and a project README

status: in-progress
agent: infra
branch: issue/0095-docs-coherence
depends-on: 0091
touches: README.md (new), ARCH.md, TESTING.md, docs/*.md, builtins/README.md, work/README.md (process docs only), no code

## Goal
ARCH.md was written before any code, and ~200 decisions have refined or changed it since
(docs/decisions.md). Other docs grew piecemeal. Make the documentation a coherent,
current description of what djs is and how it works, so a new contributor (or a new agent
after a context reset) can get oriented without reading the whole decision log.

## Acceptance criteria
- [ ] `README.md` (new): what djs is, status (test262/difftest numbers, perf vs
      quickjs-wasi, wasm size: current measured numbers), how to build/run/test
      (djs run/repl/disasm/strip-types, scripts/build-wasm.sh, the gate), the crate
      layout, the JS host package, and pointers to the other docs.
- [ ] ARCH.md reviewed section by section against the code and decisions: correct what has
      changed (shapes and dictionaries, native frames and @fast, the host inbox, fuel debt,
      deterministic mode, journals, OOM reserve/host OOM, the base snapshot, the dev flow,
      D016 wasm-first), and keep the original rationale. Mark superseded plans clearly
      (for example the old-generation delta plan, pending the owner's decision on 0088).
      Don't change designs; describe them.
- [ ] TESTING.md, docs/host.md, docs/compiler.md, docs/objects.md, docs/cli.md, docs/wire.md
      checked for stale statements (numbers, removed workarounds, renamed flags), fixed.
- [ ] A short "where to find things" index of the decision log by topic (docs/decisions.md
      stays append-only; the index goes in a new docs/decisions-index.md).

## Log

## Follow-ups

## Result
