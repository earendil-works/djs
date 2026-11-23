# 0297: Remove everything that exists for backwards compatibility

status: done
agent: pcompat
branch: issue/0297-missing-member-stack (the worktree's; merges 1-8 came from it)
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
- [x] An inventory in the Log: every compatibility path found (file, what it keeps
      working, for whom), and what was done with it (removed / kept because ... / asked).
- [x] Every clear-cut item removed, with its tests, fixtures, gate legs, docs and guide text.
- [x] Judgment calls (things that look like compatibility but may be load-bearing) listed
      and sent to the coordinator before removing them, not removed silently. Known ones:
      - the image dependency descriptor and `vm::COMPAT_VERSION` (D2265, vm/compat.rs): is it
        compatibility, or a safety check that an image is loaded only by the build that can
        read it? (Images are no persistence format since D2560/D2704.)
      - the slim module (`build-wasm.sh --slim`, gate legs "pi tests (slim without
        lockdown)", the slim ratchet) and js/pi: 0294 left them as "retire once pi is on
        host-driven runs". Find out whether anything still needs them.
      - the build:wasm-nodump module and the js tests' no-dumper cases (0294 notes).
- [x] Format-2 remnants *outside* the dumper's sources (fixtures under tests/dump/v2/,
      `plainDumps: false` style options, CLI flags, difftest modes, docs) removed; the ones
      inside crates/djs-core/src/vm/dump*.rs are 0298's: list them for 0298 instead.
- [x] Module size before/after (gzip), ceilings lowered on the branch.
- [x] Decision entry recording the sweep (what kinds of things went, and that the rule is
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

### 2026-11-23: merge 4 (J2: the descriptor)
Roots::BUILD (the build hash) replaces the descriptor; vm/compat.rs, compat_inputs.rs,
compat_tests.rs, build.rs's RECORDS_HASH/FEATURES, LoadError::Incompatible (now
OtherBuild), the COMPAT_VERSION rule in AGENTS.md and TESTING.md go (D3641).
js/src/layout.generated.ts regenerated. Guide: snapshots.md ("What an image depends on"
rewritten), arena.md, heap.md, host.md (the dump feature paragraph), testing.md.

### 2026-11-23: merge 5 (J3b: the slim build)
The `full` feature and every `FULL` guard go, with @slim-omit, prelude-slim.js, slim_tests,
the registry's absent list (registry-slim.txt, check_build, unresolved, the resolver's
absent-constructor fallback), test262-slim.toml (no-unicode/no-modules groups too) and its
ratchet, the slim size ceiling, status.py's slim rows, build-wasm.sh --slim and the slim gate
legs; one clippy combination stays (djs-core with only `unicode`). vm/dump.rs: only its six
`#[cfg(feature = "full")]` lines removed (without the feature they would have compiled the
ArrayBuffer/typed-array dump arms out; told the coordinator). The corpus comments about the
slim build go (tests/smap/codemode/redump-loops.smap re-pinned: the comment moved offsets).
docs/slim-codemode.md deleted (J9's list). Guide: builtins.md ("Two builds" section
removed), language.md ("The slim build" removed), goals.md, host.md (module table),
snapshots.md, testing.md (also the agent-host test bullets left from merge 2),
typescript.md, status.md (regenerated). TESTING.md, AGENTS.md, ARCH.md, docs/host.md,
docs/web-globals.md updated. D3642. Size, after the rebase onto 0298's D3660: 701,709 ->
701,738 B gzip (1,661,729 -> 1,661,727 raw; `Intrinsic::prototype_of` now #[inline(never)]:
without it, it was inlined twice into realm_intrinsic, +357 B raw); codemode-memory OK.

### 2026-11-23: J1 (dumps under D024)
D3643 amends D3596/D2619: the later-build guarantee for dumps is suspended until the owner
declares the format stable; helpers, hooks and goldens change with the code, goldens are
regenerated (delete the dumps, `DJS_GOLDEN_WRITE=1`). TESTING.md "Dumps across builds" ->
"Golden dumps"; ARCH.md, docs/host.md Gaps, docs/continuation-dumps.md "Compatibility",
Sandbox.dump's doc, golden-host.test.ts's header. Guide: dumps.md ("What later builds keep"
-> "Dumps and later builds"), continuations.md, testing.md, goals.md, introduction.md,
index.md, snapshots.md, suspension.md, pi.md. djs-cli tools.rs: only `print` names the
printing function (console.* identities dropped, test updated). dump_runtime.rs untouched.

### 2026-11-23: J6 (names without "v2"/"plain")
tests/dump/v2/ -> tests/dump/ (dump_resume_tests.rs: the fixture_dir line only; its doc
comments and the refusal files' generated headers still say tests/dump/v2, and the files
stay tests/dump/plain-refusals.txt and plain-chain-refusals.txt: renaming them needs more
lines of that file, pdiet's). difftest `--plain-dumps/--plain-out/--plain-from` ->
`--dumps/--dump-out/--dump-from`, `// difftest: plain-xfail` -> `dump-xfail` (and the
out dir's marker file), tests/plain-refusals.txt -> tests/dump-refusals.txt,
scripts/plain-node.mjs -> scripts/dump-node.mjs (summary `dump-node:`),
scripts/plain-fuzz/ -> scripts/dump-fuzz/, gate leg "dumps (djs and node)", dump-size's
`v2` suite -> `programs`. Left: the corpus directories `*/plain-fuzz/` (their names are
case keys of dump-size.tsv and are cited in pdiet's dump*.rs), DJS_PLAIN_OUT and the
`plain_*` test names in dump_resume_tests.rs, and docs/plain-js-dumps.md (J9).

### 2026-11-23: J7 (the registry)
D3644: ids are positions; `Registry::update` drops what no longer resolves and appends the
new (no REASON); the file has three columns and lost its 306 tombstones (1,753 -> 1,447);
scripts/registry.py and the gate's `registry` leg are gone; registry.tsv merges as
`djs-generated` and regen.sh runs the update. Tests: the tombstone/never-reused test became
`update_drops_the_gone_and_appends_the_new`. TESTING.md "Builtin registry", the gate leg
list and the always-run list; guide builtins.md ("Builtins by name: the builtin registry")
and testing.md.

### 2026-11-23: J8, J10
J8 (D3645): crates/djs-embed/CHANGES.md deleted, djs-embed takes the workspace version (no
0.x semver promise); ARCH.md's "its own semver" goes. J10a: the heap report builds text
only: `Section` lost `key` and `json`, every section's J value and the helpers only it used
(`Tally::json`, `Scope::json`) went; heap/tests.rs checks the report's text (small table
and line helpers). heap/json.rs moved to src/json.rs (the tools' answers use it) without
`J::obj`, the `From` impls and the test accessors. J10b: `crate::MODULES` -> `crate::EMBED`
(27 sites); lib.rs's EMBED doc says it carries ES modules.

### 2026-11-23: J9 (retired designs)
Deleted docs/snapshot-versioning.md, portable-snapshots.md, goldilocks-snapshots.md,
shared-segment.md, empty-heap.md, plain-js-dumps.md, functions-as-code.md (and
slim-codemode.md in merge 5); scripts/snapshot-research/ except its cases, which moved to
scripts/dump-costs/cases (dump-costs.ts reads one directory now; status.py, native/records.js).
docs/continuation-dumps.md: status line, short answer (one module, images in process only),
no "pi's shape" section, no pi test row, no History table, no format-2 asides.
ARCH.md: no "Superseded" (the status paragraph, the vm_snapshot plan sentence, the deltas
and containers bullet, P4's mention, D3600's "superseding"). References fixed in
djs-core/Cargo.toml, djs-wasm lib.rs, djs-prelude base-families.rs, sandbox.ts, js tests,
dump-fuzz, size/patches/touch.py, docs/cli.md, frozen-builtins.md (also its stale "nothing
is built" status), lessons-from-pico.md §2.6 (status note: built and removed again),
architecture-bloat-audit.md. Left for pdiet (their files): the doc comments of vm/dump.rs,
dump_cont.rs, dump_harvest.rs, dump_origin.rs and dump_plain.rs, and dump_runtime.rs's
module_text header (generated into js/src/dump-runtime.js, so that one stays too), which
cite the deleted plain-js-dumps.md / functions-as-code.md.

## Follow-ups
- For 0298 (pdiet's files): the doc comments of vm/dump.rs, dump_cont.rs, dump_harvest.rs,
  dump_origin.rs, dump_plain.rs and dump_runtime.rs's `module_text` header (and so the
  generated js/src/dump-runtime.js) cite the deleted docs/plain-js-dumps.md and
  docs/functions-as-code.md; dump.rs's base64 comment still says "which the slim build does
  not have"; dump_resume_tests.rs's doc comments and the refusal files' generated headers
  say tests/dump/v2, and tests/dump/plain-refusals.txt / plain-chain-refusals.txt,
  DJS_PLAIN_OUT and the `plain_*` test names keep "plain" (renaming needs that file).
- The corpus directories `tests/corpus/*/plain-fuzz/` keep their names (they are case keys
  of tests/scores/dump-size.tsv and the smap pins, and pdiet's files cite them).
- Kept although unused by the wasm hosts: `Vm::set_math_random_seed` and `wire::encode`
  (djs-embed uses the encoder; check the seed setter's callers again).
- docs/host.md's measured tables were taken with exports that are gone (vm_compile,
  vm_seal, ...): re-measure them when someone touches that page.
- TESTING.md "Size budget" still quotes 767.6 KB / 2.68x (the D3601 numbers); the module is
  701.7 KB gzip now.
- docs/decisions-index.md does not list D3640-D3645 yet (the coordinator's index).
- The full gate (`scripts/gate.sh --full`) was due (65 h since the last).

## Result
Everything that existed only for backwards compatibility is gone (D3640 records the rule
and the sweep; D3641-D3645 the larger pieces). Merges into main, from d481d1fe:
1. b141172f: clear-cut items and J4 (the feature detection in js/src, vm_dump's result 2,
   registry::resolve_reachable, new_vm_from/new_vm_reserved, the REPL's completion values,
   build-wasm --strip, the old merge lock, status.py's migration; the nodump module, its
   js cases and the Sandbox's no-dumper misuse errors).
2. 61bd6898: J3a: js/pi, tests/pi, the pi tests, benches and gate legs.
3. 6d558410: J5/J8 and the compile cache: ABI 9 (one vm_new), the unused exports, the
   compile-cache feature, smap_codec, WeakRefs::Absent (D3640).
4. e7f66413: J2: the image dependency descriptor; images load only into their build
   (D3641).
5. d2440ddd: J3b: the slim build and djs-core's `full` feature, with every guard, test,
   ratchet, gate leg and doc (D3642).
6. ac77f7cc: J1 and J6: dumps under D024 until the format is declared stable (D3643);
   tests/dump/v2 -> tests/dump, difftest's `--dumps/--dump-out/--dump-from`, `dump-xfail`,
   tests/dump-refusals.txt, scripts/dump-node.mjs, scripts/dump-fuzz.
7. 6533b035: J7, J8 and J10: the registry without tombstones or the append-only check
   (D3644), djs-embed without a version promise (D3645), the heap report's test-only JSON,
   `MODULES` -> `EMBED`.
8. (this merge) J9: the docs of retired designs, scripts/snapshot-research, ARCH.md's
   superseded history; the dump-size score rewritten.

Size of the shipped module (gzip -9): 767,573 B at the start -> 750,859 B after merge 3
(1,875,103 -> 1,827,977 raw); 0298 then took it to 701,709 B, and the slim removal on top
measured 701,738 B (1,661,727 raw, 2 B smaller; gzip noise), so the ceiling stays 702,733.
codemode-memory: OK, re-recorded in merge 3.
