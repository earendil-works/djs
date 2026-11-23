# 0297: Remove everything that exists for backwards compatibility

status: open
agent: pcompat
branch: issue/0297-no-compat
depends-on: -
touches: anything outside crates/djs-core/src/vm/dump*.rs and crates/djs-core/src/compiler/smap*.rs (0298 owns those): crates/, js/, scripts/, tests/, docs/, builtins/, TESTING.md, ARCH.md
guide: any page that describes a removed option, format, export or module (skim docs/guide/SUMMARY.md)

## Goal
Owner, 2026-11-23: djs needs **no backwards compatibility** (D024 already says so: it is
unreleased; change formats and ABI in place). Make one pass over the whole repository and
remove what exists only so that older hosts, older formats, older options, older dumps,
older images or older callers keep working: readers or writers of retired formats, ABI
exports and JS API options kept for old callers, aliases and renamed-but-kept names,
fallback paths for retired modes, tests, fixtures, gate legs and scripts that exist only to
keep such paths alive, and the documentation for them. Every byte of the shipped module
that pays for compatibility should go (scripts/size/budget.py `--update` lowers the
ceiling).

## Acceptance criteria
- [ ] An inventory in the Log: every compatibility path found (file, what it keeps
      working, for whom), and what was done with it (removed / kept because ... / asked).
- [ ] Every clear-cut item removed, with its tests, fixtures, gate legs, docs and guide text.
- [ ] Judgment calls (things that look like compatibility but may be load-bearing) listed
      and sent to the coordinator before removing them, not removed silently. Known ones:
      - the image dependency descriptor and `vm::COMPAT_VERSION` (D2265, vm/compat.rs): is it
        compatibility, or a safety check that an image is loaded only by the build that can
        read it? (Images are no persistence format since D2560/D2704.)
      - the slim module (`build-wasm.sh --slim`, gate legs "pi tests (slim without
        lockdown)", the slim ratchet) and js/pi: 0294 left them as "retire once pi is on
        host-driven runs". Find out whether anything still needs them.
      - the build:wasm-nodump module and the js tests' no-dumper cases (0294 notes).
- [ ] Format-2 remnants *outside* the dumper's sources (fixtures under tests/dump/v2/,
      `plainDumps: false` style options, CLI flags, difftest modes, docs) removed; the ones
      inside crates/djs-core/src/vm/dump*.rs are 0298's: list them for 0298 instead.
- [ ] Module size before/after (gzip), ceilings lowered on the branch.
- [ ] Decision entry recording the sweep (what kinds of things went, and that the rule is
      D024: no compatibility code, ever).

## Notes
- Start with a survey: `git grep -n -i` for format versions, "older", "retired", "until
  0NNN", "kept so", "still", "fallback", "alias", "v2", "format 2", "plain: false",
  "plainDumps", ABI versions; decisions.md entries that say "superseded" / "retired" whose
  code may still be there; exports in crates/djs-wasm/src/lib.rs and js/src that no
  current caller uses (js/src, js/pi, examples, the docs site); CLI flags; difftest and
  test262 runner modes; scripts/ that nothing calls.
- The dumper itself (crates/djs-core/src/vm/dump*.rs, compiler/smap*) is being reworked
  for size in 0298 at the same time: do not edit those files; send what you find there to
  the coordinator.
- Keep the gate meaningful: removing a gate leg is fine when what it tested is gone, never
  to get green.

## Log

### 2026-11-23: survey (inventory)
Method: git grep for the issue's terms plus five read-only survey passes (djs-wasm/js, djs-core,
tools/scripts, tests/docs, the known judgment calls), each caller checked with git grep.
JavaScript-language "legacy" (Annex B, legacy octal, experimentalDecorators, DOMException
codes, Date formats) is a feature, not compatibility, and is out of scope.

**Clear-cut (removed on this branch):**
- C1 js/src/sandbox.ts, trap.ts, wire.ts, base.ts: feature detection of ops every ABI 8
  module has (`djs_image_staging?`, `vm_new_with?`, `vm_frozen_builtins?`, `vm_dump_runtime?`,
  `vm_string_reserve?`, `vm_fuel_used?`, `vm_restore_math_random ?? vm_set_math_random_seed`,
  `vm_open_call?`, `vm_set_next_call_id?`, the base-inject ops, `__stack_pointer`/`djs_stack`/
  `djs_elastic_stack`/`vm_tables_need`), with "Since 0NNN; older modules lack it" comments:
  for modules older than the ABI check already rejects. Kept: the checks of ops that are
  feature-gated (dump ops, cold-unicode tables, compiled-prelude, docs).
- C2 js/pi/engine.ts: the same fallbacks (vm_new_reserved ?: vm_new, djs_image_staging ?:
  djs_staging, vm_fuel_used, "too old for ..." throws, `loadsDumps`).
- C3 crates/djs-wasm vm_dump: the unreachable result 2 ("no base image", retired in 0215).
- C4 crates/djs-core/src/vm/registry.rs `resolve_reachable`: for the dump loader's
  `$dump.builtin` (0182), deleted with format 2 (D2920); no caller.
- C5 crates/djs-prelude `new_vm_reserved`, `new_vm_from`: no caller since 0139.
- C6 `Vm::encode_wire`, `Vm::set_math_random_seed`: no caller.
- C7 djs-cli REPL `SCRIPT_COMPLETION_VALUES` (the switch from the interim expressions-only
  mode, always true) and its dead branches; the repl test that `.image` stays unknown.
- C8 scripts/build-wasm.sh `--strip` (no-op alias of the old opt-in flag).
- C9 scripts/merge-to-main.sh: take/drop of the merge lock for merge-to-mains from before
  D2666.
- C10 scripts/status.py: the migration that drops the dump module's rows (already ran); usage
  text naming the dump module.
- C11 stale text about removed things: docs/cli.md (`run --backend`, `.image`/`djs heap`
  history, `.restore` of images), docs/host.md (ABI version history and "returns 6",
  "modules before 0094 lack it", `$dump.pending`/`"djs-dump 2"` in the pi section, the
  dumpModule and 0177 history notes), TESTING.md (format-2 history note, the clippy leg's
  "dump" module), gate.py's bytecode-backend comment, comments naming the dump module
  (djs-difftest/Cargo.toml, plain-fuzz), dump-costs.ts header (format 2, wasm.ts), djs-core
  comments on containers/session images/reference images (compat.rs, registry.rs,
  vm/mod.rs, vm/tests.rs), crates/djs-cli/tests/run/reads.src.js (format 2, reads.plain.js).
- C12 `ImageDeps::hash` (the session image header's hash of its base): no caller (moot if
  J2 goes).
- C13 scripts/__pycache__/*.pyc committed by accident (not compatibility; stale copies of
  gate.py show up in every grep).

**Judgment calls (sent to the coordinator, not removed yet):**
- J1 D3596: plain dumps as an exception to D024 (any build's dump loads in every later
  build; dump-runtime helpers never change or go; goldens never rewritten). The biggest
  compatibility promise left.
- J2 the image dependency descriptor and `vm::COMPAT_VERSION` (vm/compat.rs,
  compat_inputs.rs, compat_tests.rs, build.rs's RECORDS_HASH/FEATURES, Roots' descriptor
  words, LoadError::Incompatible, AGENTS.md's bump rule, TESTING.md "Image compatibility",
  guide snapshots.md). No image crosses builds any more: vm_from_bytes's only caller is js/pi
  (image cache keyed per WebAssembly.Module), the base is asserted BUILT_BY == BUILD_HASH,
  djs-embed snapshots stay in process, D3600 removed the module transport. A BUILD_HASH (or
  layout hash) check in the header would do.
- J3 the slim build (feature `full`, `--slim`, @slim-omit, prelude-slim.js, slim_tests,
  registry-slim, test262-slim, slim ratchets and ceiling, its gate and clippy legs, guide
  "two builds") and js/pi + tests/pi (+ pi legs, pi-dump-every-idle.sh, pi bench, pi
  determinism/dump-costs scripts; vm_from_bytes/djs_image_staging/vm_seal then unused). pi
  still runs quickjs-wasi; js/pi is a test harness only.
- J4 the module without the dumper (build:wasm-nodump, DJS_WASM_NODUMP js test cases, the
  Sandbox's misuse errors): D3600 keeps it as a supported configuration.
- J5 layered ABI exports: vm_new / vm_new_reserved / vm_new_with (one would do),
  vm_compile (= vm_compile_file with no name; no host calls it), vm_parse_json /
  vm_json_stringify (only the bench's JSON baseline and tests).
- J6 names left over from format 2: tests/dump/v2/ (paths in 0298's dump_resume_tests.rs
  too), the `v2` suite in dump-size.tsv; difftest's `--plain-dumps`/`--plain-out`/
  `--plain-from`, `plain-xfail`, plain-refusals.txt.
- J7 the builtin registry's append-only numbering and 306 tombstones (registry.py check):
  ids nobody persists any more.
- J8 `WeakRefs::Absent` (pi's quickjs parity) and the djs-embed semver/CHANGES.md policy.
- J9 research docs about retired designs (docs/snapshot-versioning.md,
  portable-snapshots.md, goldilocks-snapshots.md, shared-segment.md, empty-heap.md,
  plain-js-dumps.md, functions-as-code.md, continuation-dumps.md status) and
  scripts/snapshot-research/ (cases/ and src/pi.rs still used by a heap test, dump-costs and
  status.py); ARCH.md's "Superseded" history bullets.
- J10 djs-cli: heap report sections still build the JSON of the removed `djs heap --json`
  (tests read it); tools.rs accepts `console.*` as identities of `print`, which the exporter
  never writes; `crate::MODULES` alias of `EMBED`.

**For 0298 (inside vm/dump*.rs, compiler/smap*.rs; not touched):**
- `DumpOptions.base`: djs-wasm passes an empty base since 0215 (names come from BaseNames).
- `#[cfg(feature = "full")]` in dump.rs (only for the slim build, J3).
- dump_resume_tests.rs's tests/dump/v2 paths (J6).

**Not compatibility (kept):** D3592's "older module ignores an extra argument" (no code);
difftest's unused directives `ts`/`fuel`/`timeout`/`plain-xfail` (tools); standalone
measurement scripts without callers (merge-stats.sh, wasm-leb-padding.py, ...).

### 2026-11-23: answers (coordinator, owner)
- J1 (owner): "In the future sure, but not for formats that were made in the dev past."
  D3596's guarantee is suspended until the owner declares the dump format stable: dumps
  fall under D024 (helpers, hooks and goldens change with the code; goldens are
  regenerated). Decision amending D3596/D2619 is mine; drop the CLI's console.* identities.
- J2 yes: a BUILD_HASH word in the roots cell; descriptor machinery and its rule go.
- J3 (a) js/pi and everything around it go (own merge); (b) (owner) the slim build goes
  completely; dump*.rs's `full` cfgs are pdiet's (tell the coordinator when the feature is
  gone).
- J4 the dumper-less support goes (gate leg, js cases, the Sandbox's misuse errors); the
  `dump` cargo features stay as build knobs.
- J5 yes: one vm_new, no vm_compile, vm_parse_json, vm_json_stringify; ABI bumped in place.
- J6 yes (the path constant in dump_resume_tests.rs may change).
- J7 yes: no append-only numbering, no tombstones; path coverage stays.
- J8 (owner) WeakRefs::Absent and the Sandbox's weakRefs option go entirely; djs-embed's
  semver/CHANGES.md promise goes.
- J9 yes, last and its own merge.
- J10 yes (heap JSON, MODULES alias; console.* with J1).

### 2026-11-23: merge 1 (clear-cut + J4)
C1 (js/src feature detection), C3, C4, C5, C7, C8, C9, C10, C11 (the parts not rewritten
by later steps), C13, and J4. Not done: C2 (js/pi goes, J3a), C6 (unused helpers, not
compatibility: kept), C12 (compat.rs goes, J2). Guide: js-api.md (callSite), dumps.md
(no dumper-less build). Checked: tsc, js tests on the dev module, djs-cli tests, clippy of
the touched crates, djs-docs check.

### 2026-11-23: merge 2 (J3a: js/pi)
Removed js/pi (vm.ts, engine.ts, protocol.ts, pi-tests.ts), tests/pi, js/test/pi-vm and
pi-pool, js/bench/pi.bench.ts, scripts/determinism/pi_polls.ts,
scripts/dump-costs/pi-sessions.ts, scripts/pi-dump-every-idle.sh, the gate legs "pi tests
(slim without lockdown)", "pi tests (next mode)" and the full-only "with a dump at every
idle point", the gate's `pi` external, status.py's `pi` command and the pi/* rows of
status.tsv, djs-docs's pi conformance line, the djs-cli heap test that built a pi session
(from scripts/snapshot-research/src/pi.rs; a waiting VM keeps its own test). `fuel/slice-us`
(limits.md) is now measured by js/bench/sandbox.bench.ts (`status.py startup`). Guide:
pi.md rewritten around the durable code mode (the adapter, next mode and pi's test
sections went). docs/host.md: its "pi codemode" section went. vm_from_bytes,
djs_image_staging and vm_seal go with the ABI change (J5).

### 2026-11-23: merge 3 (J5, J8, compile cache)
ABI 9 in place: one vm_new(limit, live, flags); vm_compile, vm_parse_json,
vm_json_stringify, vm_from_bytes, djs_image_staging, vm_seal, vm_set_math_random_seed,
vm_compile_cached, vm_dump_maps_export/import gone (coordinator: remove the compile cache
too). djs-core: the `compile-cache` feature, vm/compile_cache.rs and its tests, the
heap's intern log, compiler/smap_codec.rs and the cfg(feature = "compile-cache") items of
vm/dump.rs (pure deletion, agreed with the coordinator; hunks in
work/scratch/0297/dump-compile-cache.diff), difftest's cache check. WeakRefs::Absent and
the Sandbox's `weakRefs` option gone (owner). The wasm bench's JSON baseline calls the VM's
JSON.parse/JSON.stringify. Size: 767,573 -> 750,859 B gzip; ceiling 751,883;
codemode-memory.tsv re-recorded. Guide: host.md (operations table), snapshots.md,
builtins.md, gc.md, determinism.md, js-api.md, continuations.md (the dumper's caches).
D3640 records the sweep.

## Follow-ups

## Result
