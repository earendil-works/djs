# 0121: Codemode corpus expansion; shorter ratchet reasons

status: in-progress
agent: infra
branch: issue/0121-corpus-expansion
depends-on: 0118
guide: docs/guide/testing.md (if the corpus description changes)
touches: scripts/harvest/, tests/corpus/codemode-harvest/, crates/djs-difftest/ (ratchet reason format, structured fake tools), TESTING.md

## Goal
0118's follow-ups:
1. Harvest more of the ~1,800 codemode runs that 0118 had to skip, by giving the fake tools
   structured synthetic results for MCP tools, `models`, edit/write and `load` (stored
   values), under exactly the same privacy rules as 0118 (D2281; redaction report;
   manual review; the coordinator spot-checks before merge).
2. Shorter ratchet reasons in difftest: a short reason (the first differing line, truncated)
   instead of whole output diffs (the 5 harvest entries total ~28 KB).

## Acceptance criteria
- [ ] More harvested cases (target: double the coverage of language features and tool
      shapes, not just the count), djs vs qjs results clustered and reported.
- [ ] The ratchet reason format shortened, with the full diff still available via the
      runner's normal output.

## Log

- Part 2 first: ratchet reasons are an excerpt around the first difference (D2282,
  `diff::excerpt`), the corpus ratchet went from 28.7 KB to 1.2 KB.
- Part 1: extract.py now collects per run the declared result types of the called tools
  (`toolsAdded` descriptions, `describeTool` output) and the stored values of literal
  `load()` keys (`codemode-store` entries). New synth.py: a TS type parser plus a value
  synthesizer. build.py fakes MCP tools, structured bash, edit/write/transcribe_file,
  `models`, `image` and `load` (D2283), with extra redaction: literals over 40 chars,
  opaque ids, non-ASCII words, admin-tool runs out, report lists renamed tool names
  (D2284). New options `--skip REPORT` and `--start N`.
- Selection: 2,620 executions → 900 candidates → 150 cases h151–h300. 29 picks were
  excluded for a SyntaxError in qjs (mostly scrubbing that broke a template or regex),
  32 rejected in manual review (private service/project names, a private app's layout,
  recognizable prose, a poem); the selection reran until every pick passed. Automated
  audit on the written cases: no /Users, emails, URLs, IPs, long hex, key formats or
  owner/org names; capitalized pairs are all synthetic words.
- djs vs qjs (both backends): 295/300 pass. The 5 failures are 0118's qjs string-spread
  bug (h075, h082, h084, h124, h150); all 150 new cases match qjs. Stress modes
  (`--suspend-every 97 --gc-stress`) identical on both backends. 25 new cases end in a
  qjs-matching uncaught error (synthetic data the script does not expect).
- Coverage (build.py's features, measured on the committed files): language features
  36 → 46 of 51, feature occurrences 808 → 1,845 (2.3×), tool-shape features 0 → 6 of 7
  (structured, models, classify, edit-write, load, image; mcp prefix is renamed so the
  file-level check misses it, the build counted 55). Tool kinds: bash/read only →
  + structured 89, models 25, load 22, image 20, edit 21, write 18, transcribe 9.
  Newly covered: reduce, sort, Set, Map, padStart, Number, Date, case, labels,
  localeCompare. Still uncovered: flatMap, class, function declarations, generators.
- Docs: TESTING.md source #1, guide testing.md (harvest paragraph), scripts/harvest/
  README.md (fake tools, options, redaction), D2282–D2284.

## Follow-ups

- Runs using tool discovery (searchTools/describeTool) or `fetch` are the largest
  remaining excluded group (404); faking discovery from the recorded declarations would
  add them.
- 429 runs call tools without a declared result type in the session; a generic
  `CallToolResult` with text content could cover them, at the cost of realism.

## Result

- `tests/corpus/codemode-harvest/` has 300 cases (h151–h300 new), REDACTION.md has round 1
  and round 2 sections. All new cases match qjs on both backends.
- Harvester: `scripts/harvest/{extract,build,synth,anon}.py`, README has the steps.
- Ratchet reasons are short (D2282).
- Known gaps: see Follow-ups; no class/generator/flatMap use in any harvested run.
