# 0047: Development flow efficiency: why the machine is spinning, and proposals

status: open
agent: -
branch: issue/0047-devflow
depends-on: -
touches: work/issues/0047-devflow.md (the report), work/scratch/0047/ (measurements). No changes to shared scripts, Cargo.toml or code without coordinator approval.

## Goal
The owner reports that the machine is "spinning like mad": we recompile a lot and run
too many tests. Investigate where the CPU time goes in our development flow and come
back with concrete, measured proposals. This is an **investigation**: report first. The
coordinator decides what to adopt, and implementation follows as separate steps.

## Context (coordinator's observations)
- 18 cores. Four implementation agents work in parallel, each in its own worktree with
  its own `target/` (1.6–2.5 GB each), so every dependency and djs-core change is
  compiled four times or more.
- `scripts/gate.sh` runs on every merge (`scripts/merge-to-main.sh` rebases, then gates).
  It covers: fmt; clippy (native, all targets); `cargo test --workspace`; a wasm32 build
  of djs-core; wasm32 release clippy; bgen --check; a djs-wasm debug build; node tests;
  wire and reader cross-language tests (more cargo test invocations); difftest with
  suspension and GC stress; and the full test262 active group (32k tests) in debug.
- djs-prelude's build.rs compiles djs-core for the host to build the base image. D1330
  hashes djs-core's sources, so any djs-core change rebuilds the base and everything
  downstream (djs-cli, difftest, test262, wasm). djs-wasm builds compile djs-core twice
  (host for build.rs, and wasm32).
- `[profile.dev.package.djs-core] opt-level = 1` (D010).
- Agents also run `cargo test --workspace`, test262 and difftest themselves while
  iterating, often in release, plus benchmarks.

## What to deliver
- [ ] Measurements: wall and CPU time of each gate step (cold and warm cache), of an
      incremental rebuild after touching one file in djs-core / compiler / builtins, and
      of `cargo test --workspace`. Which crates rebuild and why (`cargo build -v`, or
      `CARGO_LOG=cargo::core::compiler::fingerprint=info` for the reasons). How many
      times djs-core is compiled per gate run. Disk use per worktree.
- [ ] Proposals, each with the expected saving and its risk. Candidates to evaluate (not
      exhaustive):
  - a shared `CARGO_TARGET_DIR` across worktrees (with its lock contention), or sccache
    (only if it is already installed; do not install anything);
  - splitting the gate into a fast tier (always, per merge) and a slow tier (periodic, or
    by the coordinator), and/or running test262/difftest in release with a cached build;
  - reducing duplicate compilation (the build.rs host compile of djs-core, profiles that
    differ only slightly so cargo rebuilds: dev vs test vs release vs wasm, debuginfo
    settings, `--all-targets` clippy vs test building the same things twice);
  - whether the D1330 source hash is too eager (for example hashing only what the image
    depends on);
  - agent habits: what agents should run while iterating (targeted tests: `cargo test -p
    djs-core <filter>`, a filtered test262 run) vs before merging (the gate). Propose a
    short "how to test while iterating" section for AGENTS.md;
  - limiting parallelism (`CARGO_BUILD_JOBS`, `nice`) so four agents don't oversubscribe
    18 cores;
  - anything else you find.
- [ ] A recommended set: what to change first, in order, with the measured or estimated
      effect.

## Notes
- You get your own worktree for experiments (`.worktrees/0047-devflow`). Measure there,
  not in other agents' worktrees, and don't kill other agents' processes.
- Machine load from other agents makes timings noisy. Repeat them and say how noisy they
  were.
- Use `scripts/timeout.sh` for anything that runs engines or test suites.

## Log

## Findings

## Proposals

## Result
