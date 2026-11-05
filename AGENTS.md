# AGENTS.md

Instructions for every agent working in this repository: the coordinator and the
sub-agents that implement issues.

## What this is

djs is a small JavaScript + TypeScript engine in `no_std` Rust, built mainly for wasm and
meant to replace QuickJS in pi's codemode. Read these before you start any work:

1. `ARCH.md`: the architecture. It is the design reference. Do not silently deviate from
   it. If the design needs to change, say so in your issue log and propose an edit.
2. `TESTING.md`: how we test: test262, the differential oracle and the stress modes.
3. `docs/decisions.md`: decisions made during implementation. Append to it, never
   rewrite it.
4. `work/README.md`: the work tracking process (issues, worktrees, merging).
5. Your issue file in `work/issues/`.

## Oracle

- `~/Development/dquickts` is the reference engine and is **read-only**. Never build,
  modify or `git` anything in it. Use the existing binaries:
  `~/Development/dquickts/qjs` and `~/Development/dquickts/run-test262`.
- Its tests (`tests/test_*.js`, `tests/test_typescript.ts`) and its `test262.conf` /
  `test262_errors.txt` are useful reference material.

## Code conventions

- Rust edition 2024, stable toolchain. `cargo fmt` and clippy with `-D warnings` must be
  clean. The one exception: the shipped wasm module's build (scripts/wasm-build-env.sh
  only) sets RUSTC_BOOTSTRAP=1 for `-Zlocation-detail=none` (D2333) and for core/alloc
  built from source with `-Cpanic=immediate-abort` and `-Zshare-generics=no` (cargo's
  `-Zbuild-std`, needs the rust-src component; D3090); everything else, and that build
  with DJS_STABLE_ONLY=1, compiles on plain stable.
- `djs-core` is `#![no_std]` + `alloc`. It must always build for
  `wasm32-unknown-unknown`. No `std`, no platform dependencies.
- Dependencies: none in `djs-core` without a recorded decision in `docs/decisions.md`.
  Std tool crates (`djs-cli`, `djs-test262`, `djs-bgen`) may use small, well-known crates
  where they save real work, but still keep them minimal. Dev-dependencies for tests are
  fine.
- No `unsafe` in `djs-core` unless a decision records why. Heap access is bounds checked.
- Prefer plain, direct code. Comments explain *why*. Keep modules focused; no speculative
  abstraction.
- djs is unreleased (D024): change formats and ABI in place; don't keep readers or
  exports for older versions.
- Images are keyed on their dependency descriptor (D2265), not the build. A change that
  keeps every table's numbering but alters what a stored slot, record or encoding means
  bumps `vm::COMPAT_VERSION` (TESTING.md "Image compatibility" says when).
- Every feature comes with tests. Unit tests sit next to the code; JS behavior tests go in
  `tests/` (see TESTING.md).

## Workflow (sub-agents)

You work on exactly one issue at a time, in your own git worktree under `.worktrees/`, on
the branch `issue/NNNN-slug`. The coordinator creates the worktree for you.

1. Read the issue. Set `status: in-progress` in its header and commit that.
2. Work in small steps and **commit often** (`git commit` on your branch). Message format:
   `NNNN: short imperative summary`, plus a body when the reason is not obvious.
3. Keep the issue file's `## Log` section current: what you did, what you found, open
   questions. This file is how context survives between agents and sessions.
4. If you make a design decision that others must know about, append it to
   `docs/decisions.md` on your branch (format given in that file).
5. If you discover follow-up work, list it under `## Follow-ups` in your issue. Do not
   create new issue files; the coordinator does that.
6. When the acceptance criteria are met:
   - set `status: done` in the issue header and fill in `## Result` (what landed, how to
     use it, known gaps),
   - if your change makes djs pass tests that were expected to fail, update the ratchets
     on your branch: `cargo run -p djs-difftest -- --engine djs --update-ratchet` and
     `cargo run -p djs-test262 -- run --engine djs --group active --expected
     tests/test262/expected-failures.txt --update`. The gate reports stale entries but
     does not fail on them, and a stale ratchet hides later regressions of those tests,
   - commit,
   - run `scripts/merge-to-main.sh` from your worktree. It rebases onto `main`, runs
     `scripts/gate.sh` in the gate worktree and fast-forwards `main` under the merge lock,
     rebasing and gating again (mostly from the cache) if `main` moved meanwhile (D2666).
     If the rebase conflicts, resolve it and run the script again. If the gate fails, fix
     the failure; never weaken the gate to get green.
7. Report back to the coordinator with a short summary: what landed, the commit range,
   follow-ups, anything surprising.

Rules:

- Never commit directly on `main` and never push anywhere. `main` must always pass the
  gate.
- Do not edit `work/BOARD.md` (the coordinator owns it) or other agents' issue files.
- Put scratch files, logs and large outputs in `work/scratch/<issue-id>/`, which is
  gitignored. External checkouts such as test262 go in `vendor/` **of the primary checkout** (the
  repo root, not your worktree), so that all worktrees share one copy; it is gitignored.
- Run anything that executes an engine (qjs, djs, test suites that shell out) with a
  timeout: `scripts/timeout.sh 600 cargo test ...` (macOS has no `timeout`). A hung test blocks you silently: one such hang
  cost two hours.
- If you are blocked or an issue turns out to be much bigger than described, stop and tell
  the coordinator instead of expanding the scope on your own.
- Stay inside the areas listed in your issue's `touches:` field. If you need to change
  something outside them, say so in the log and keep the change minimal: other agents may
  be working there in parallel.

## Keeping the guide current

`docs/guide/` is the human-facing explanation of djs: what a user sees and can do, and the
architecture, how it works and why (`docs/guide/README.md` for how to write it). It must
stay true as the code changes:

- If your change alters what a guide page says (the issue's `guide:` field names the
  pages; otherwise skim `docs/guide/SUMMARY.md`), update that page's prose on your
  branch. Prose is not checked by any tool, so this is on you; say in the issue log which
  pages you updated, or that none were affected.
- Never edit generated regions (`<!-- gen:... -->` ... `<!-- /gen -->`), `docs/guide/gen/`
  or `docs/guide/ref-*.md` by hand: `cargo run -q -p djs-docs -- regen` rewrites them,
  and merge-to-main does it after the rebase (scripts/regen.sh).
- `cargo run -q -p djs-docs -- check` (part of the gate) fails on stale generated parts,
  broken links, the writing rules below, and an example whose output (`gen:run`,
  `gen:dump`) changed. If the new output is right, accept it with `regen --examples`
  and reread the text around it.
- The guide is self-contained and describes djs as it is (owner's rules, D2102, D3051): a
  page explains what it needs in its own words. No mentions of or links to ARCH.md,
  TESTING.md, docs/*.md (the decision log included), work/… or issue numbers; no decision
  IDs: explain the reason in words; no history ("used to", "was replaced", format
  versions); pi named only on `pi.md`, hosts described in general elsewhere; goals by name
  with a link to goals.md, never "G2"; code only as links to repository files, which the
  site turns into source links. `check` enforces this; docs/guide/README.md has the
  details.
- If a change makes a whole chapter wrong or calls for a new one, do not rewrite it as a
  side job: list it under `## Follow-ups`; the coordinator schedules it.

## Testing while iterating

The gate runs once, inside `scripts/merge-to-main.sh`. Don't run `scripts/gate.sh` yourself
before merging; while iterating, run only what your change can break (0047, D012). The
gate caches each leg by its inputs (TESTING.md "The gate", D2662): a change to docs or
js/ gates in seconds, one to Rust reruns every Rust leg, so a gate that fails costs a
full rerun of what you touched. A new test suite is a new leg in scripts/gate.py.

- Unit tests of one area: `cargo test -p djs-core --lib compiler::` (2–3 s).
- e2e: `DJS_E2E_MODE=plain cargo test -p djs-core --test e2e`; the stress modes once
  before merging.
- test262: dev build, filtered: `cargo run -q -p djs-test262 -- run --engine djs
  built-ins/RegExp`. djs-core is opt-level 1 in dev; the active group takes ~4 s in dev vs
  ~3 s in release, and a release rebuild costs more than that. Don't run the unfiltered
  default (all groups, ~300 CPU-s) to check a change.
- difftest: `cargo run -q -p djs-difftest -- --engine djs <filter>`; add the stress flags
  for the cases you touched.
- qjs results don't change: read `tests/test262/oracle-failures.txt` and the corpus
  `.expected` files instead of re-running qjs.
- Never re-run gate steps by hand: the full stress difftest (`--ratchet --suspend-every 97
  --gc-stress` over the whole corpus) and the full active test262 run are exactly what the
  gate does at merge time (0047/0053 measured them as a third of all agent CPU). Run the
  cases or directories your change touches instead.
- Use `--release` only for benchmarks (`scripts/bench.sh`) and timing work, and
  `scripts/bench.sh --record` only on a quiet machine (load below ~4): numbers recorded
  under load are not comparable.
- Leave cores for the others: the repo's `.cargo/config.toml` caps cargo at 6 jobs and 6
  test threads; pass `-j 6` to the test262/difftest runners. Everything that runs an
  engine goes under `scripts/timeout.sh`.
- Yield to the gate: `scripts/timeout.sh` runs its command under `nice -n 10` (D1793),
  so the gate, the serial bottleneck of every merge, gets the CPU first. Run your cargo
  builds and test loops through it too (`scripts/timeout.sh 900 cargo test -p djs-core
  ...`), or prefix them with `nice -n 10`. merge-to-main.sh and the gate are not niced.

## Coordinator

The coordinator (the main agent) owns `work/BOARD.md`, creates issues and worktrees,
assigns agents, reviews merged work and keeps ARCH/TESTING/decisions and the guide
coherent. Every issue names the guide pages it can affect (`guide:`, or `-`); after a
merge the coordinator checks that those pages still read true and turns larger guide
updates into issues of their own. It works
directly on `main` only for coordination files (issues, board, docs) and commits right
away, so `main` is never left dirty. It commits through `scripts/with-merge-lock.sh`, so
that main never moves under an agent's fast-forward (which then rebases and gates again).
It runs the
full gate (`scripts/gate.sh --full`, no cache, plus the full-only legs) on main at least
daily and before a release (D2664).
