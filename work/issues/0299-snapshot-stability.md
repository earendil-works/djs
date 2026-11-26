# 0299: Heap snapshots revisited (1): what psychoscript does, and how often a bytecode-keyed snapshot would break

status: done
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
- [x] **psychoscript, read properly** (TS `src/snapshot*.ts`, `src/snapshot/codec.ts`, and
      the Rust port `rust/crates/psychoscript/src/snapshot.rs`): what a snapshot holds, how
      code, builtins, the base, host calls in flight and frames are named, what its compat
      key is (build id vs per-source bytecode hash), its size on comparable scripts (run it:
      `bun` is installed; e.g. their README's suspend example and a few of our harvested
      codemode scripts), what it refuses, how much code the codec is. What djs can take
      from it and what does not transfer (a JS-object VM vs djs's cell heap).
- [x] **Bytecode stability over djs's history.** For the djs commits that merged an issue
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
- [x] **Schema stability proxies** over the same commits: the cell layout table
      (js/src/layout.generated.ts or the Rust layout table), the builtin registry paths
      (tests/builtins/registry.tsv), the intrinsic names: how often each changed in a way a
      logical snapshot keyed by *names* would notice (a removed/renamed name or a field whose
      meaning changed; additions do not count).
- [x] Write sections 1 (psychoscript) and 2 (stability evidence) of
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
- Read the prior reports (portable-snapshots, goldilocks, D2560, D2700, D3641, D3643) and
  psychoscript's TS snapshot (snapshot.ts, codec.ts, snapshot/snapshot.ts,
  intrinsic-names.ts) and the Rust port's snapshot.rs. Ran the TS reference under bun on
  the README example and all 300 harvested cases (first and last idle point, FIFO answers
  like djs's runner): 286/286 x 2 restore and continue the same, modulo wall-clock prints.
- Merge points: the last commit of each issue on main (linear history), 0102..0298 = 196
  points. Built djs-cli (dev, niced, -j 6, one worktree and target dir) at all of them:
  none failed, 4-18 s each. Disassembled today's 331 corpus scripts with every build.
- disasm prints regexp literal constants as `internal@<heap offset>`: normalized.
- Hash levels exact / pc / frames (await-holding functions need pc equality, the others
  their interface). Schema proxies with git show: layout names/types, object class slot
  counts and docs, registry paths, closure kinds, NewRecord sizes, state constants, plus
  op/intrinsic numbering and layout offsets for contrast.
- Found: 10 of 195 pairs changed any corpus bytecode, 6 a frame-holding function; since
  0200, 4 of 95 and 1 (0259). Five of the ten were changes made for continuation dumps
  (0170, 0192-0194, 0205). Raw-byte hashing would break on op renumbering (5 pairs) and
  intrinsic id moves (15). Named schemas: no codemode-visible removal survived a hand
  triage; the weak spot is positional object/record slots (0151 moved one, 0236 added one).
- Wrote sections 1-2 of docs/heap-snapshots-revisited.md, with placeholders 3-5 for 0300.
  Guide pages: none affected.

## Follow-ups
- A gate leg that pins the corpus's name-normalized bytecode per function (the hash of
  scripts/research/0299/analyze.py), so a change of a waiting function's bytecode is a
  visible decision. Only worth it if heap snapshots are adopted.
- A machine-readable slot table for object classes and builtin closure records (today
  prose in heap/object.rs and positional `RecordGet` indexes): what a by-name snapshot needs.

## Result
docs/heap-snapshots-revisited.md sections 1 (psychoscript) and 2 (stability evidence).
Research scripts on this branch only, `scripts/research/0299/` (points.sh, build.sh,
disasm_all.py, analyze.py, schema.py; removed before the merge, see the branch history);
the psychoscript harness and the size script in work/scratch/0299 (psy/exp/harvest.ts,
sizes.py). Headline: keyed by per-function name-normalized bytecode (required only for
functions with a frame) and names, a snapshot of a corpus script would have broken at
3% of merges (0102..0298) and 1% since 0200 (one merge, 0259, 17 of 331 scripts), vs 75-81%
for a build key; every corpus script survives a 10-issue span in 78-88% of windows.
Left out: issues before 0102, atom-id churn, the Rust port's sizes, capture descriptors
and flags in the hash, semantic changes that keep every name.
