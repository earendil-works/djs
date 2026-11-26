# 0299: Heap snapshots revisited (1): what psychoscript does, and how often a bytecode-keyed snapshot would break

status: in-progress
agent: psnapr
branch: issue/0299-snapshot-stability
depends-on: -
touches: docs/heap-snapshots-revisited.md (new, shared with 0300: you write sections 1-2), work/scratch/0299/, scripts/research/ (optional, on the branch only)
guide: -

## Goal
The owner doubts that continuation dumps (a waiting run written as JavaScript, ~26.5k
lines of Rust in vm/dump*.rs, ~270 KB gz of the shipped module) are the right design, after
looking at psychoscript (https://github.com/anomalyco/psychoscript, cloned in
`vendor/psychoscript` of the primary checkout), which persists runs as ordinary heap
snapshots: a logical graph of records, code never saved but recompiled on load and checked
by bytecode hash, builtins by name or by position in a fresh base, refusing another build.
The real problem with heap snapshots is the stability of the heap across versions of djs
and pi. Question: can a heap snapshot survive upgrades in more situations than
"same build", without the source-dump machinery, and how small can it be?

This issue is the evidence half; 0300 (agent psnap) builds the prototype in parallel.

Prior work you must read first (deleted from the tree by 0297, in git history):
- `git show 9d001b47:docs/portable-snapshots.md` (0141: a by-meaning export, prototype
  survived a 406-commit jump)
- `git show 9d001b47:docs/goldilocks-snapshots.md` (0142: tiers; the owner's "image with
  symbolic relocations" idea and its churn numbers: breaks 28% of issues)
- `git show 9d001b47:docs/snapshot-versioning.md` if it exists, D2560, D2700, D3641,
  D3643 in docs/decisions.md.

## Acceptance criteria
- [ ] **psychoscript, read properly** (TS `src/snapshot*.ts`, `src/snapshot/codec.ts`, and
      the Rust port `rust/crates/psychoscript/src/snapshot.rs`): what a snapshot holds, how
      code, builtins, the base, host calls in flight and frames are named, what its compat
      key is (build id vs per-source bytecode hash), its size on comparable scripts (run it:
      `bun` is installed; e.g. their README's suspend example and a few of our harvested
      codemode scripts), what it refuses, how much code the codec is. What djs can take
      from it and what does not transfer (a JS-object VM vs djs's cell heap).
- [ ] **Bytecode stability over djs's history.** For the djs commits that merged an issue
      since 2026-11-01 (the `coordinator: NNNN done` commits, ~47; take a subset if builds
      are too slow, at least every merge of the last ~20 issues plus a few older ones),
      build the CLI (dev profile is fine, a separate target dir under work/scratch/0299/,
      niced, `-j 6`) and compile the corpus scripts (tests/corpus/codemode-harvest/*.js and
      tests/corpus/codemode/*.js) with `djs disasm` (or a tiny equivalent you add on a
      scratch commit if disasm is missing or prints unstable ids). Hash each function's
      bytecode, normalized only for what does not change meaning (atom ids -> names,
      intrinsic ids -> names, constant pool order if it is pure numbering). Report:
      - per consecutive pair of merges: did any script's bytecode change, how many scripts,
        which issue;
      - survival over spans of 1, 5, 10, 20, 30 issues: the fraction of scripts whose every
        function compiles to the same normalized bytecode;
      - the same for "the functions that hold a frame or a closure at an idle point"
        if you can tell them apart cheaply (otherwise all functions, say so).
      Compare with goldilocks' table (build hash breaks 70%, relocations 28% per issue).
- [ ] **Schema stability proxies** over the same commits: the cell layout table
      (js/src/layout.generated.ts or the Rust layout table), the builtin registry paths
      (tests/builtins/registry.tsv), the intrinsic names: how often each changed in a way a
      logical snapshot keyed by *names* would notice (a removed/renamed name or a field whose
      meaning changed; additions do not count).
- [ ] Write sections 1 (psychoscript) and 2 (stability evidence) of
      `docs/heap-snapshots-revisited.md` (create it with a short header; 0300 adds its own
      sections; the coordinator writes the summary). Numbers with the commands that
      reproduce them. Merge the doc (doc-only merges gate in seconds); research scripts stay
      on your branch or in work/scratch/0299.

## Notes
- This is research: no engine changes on main. Keep builds niced and at -j 6; the gate and
  agent psnap share the machine.
- Historic commits may not build with today's toolchain or need a build script; skip a
  commit that fails and say so.
- If the whole history is too slow, prefer the most recent ~20 merges: that is the
  development pace a stored snapshot faces now.
- Do not stop on scope: narrow it and report what you left out.

## Log

## Follow-ups

## Result
