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

## Follow-ups

## Result
