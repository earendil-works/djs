# 0130: Which JS features does real codemode use? (research: what could be cut)

status: done
agent: infra
branch: issue/0130-feature-usage
depends-on: 0118, 0121 (harvester)
guide: -
touches: scripts/harvest/ (a feature-usage analyzer), docs/codemode-feature-usage.md (new report), work/scratch/0130/ (raw data, never committed)

## Goal
Owner request: scan **all** codemode executions in ~/.pi/agent/sessions (not just the
300 harvested cases) and find which scripts need "extended" JavaScript features, such as
prototypes and the prototype chain, classes, getters/setters, Proxy/Reflect, Symbols and
well-known symbols, generators/iterators protocol, WeakMap/WeakRef, tagged templates,
labels, `with`-like dynamics, eval/Function, RegExp advanced features (lookbehind,
named groups, \p, sticky), BigInt, typed arrays/ArrayBuffer/DataView, Intl/locale,
Date details, getter-heavy descriptors (defineProperty, freeze/seal), `arguments`,
sloppy-mode-only behavior, etc. Goal: an evidence base for what djs could in theory drop
(or demote to slow paths / bytecode) for codemode.

## Method
- Parse every codemode script with djs's own parser/compiler (or a feature scanner on
  the AST/tokens), not regexes alone; count per feature: number of scripts, sessions, and
  first/last date seen; distinguish direct syntax use from builtin calls.
- Also consider what the *prelude/host* needs (pi's prelude uses some features itself);
  features used only by our own prelude can be kept internal.
- Note dynamic features invisible to static scans (e.g. objects from tool results being
  iterated, JSON shapes) and estimate them where possible.

## Privacy (D021, D2281 apply)
Raw scripts never leave work/scratch/0130/. The committed report contains only feature
names, counts, dates, and at most short **synthetic** illustrative snippets written by
the agent, never real script excerpts, paths or names.

## Deliverable
docs/codemode-feature-usage.md: a table of features by usage (never / rare / common),
which ones look droppable or demotable, the size/complexity each would save in djs
(rough, with pointers to the code), and the risks (pi's prelude, test262 conformance
expectations, user expectations). No engine changes in this issue.

## Log

- extract.py records each execution's time; new scripts/harvest/features.py compiles
  every script with `djs disasm --body` and reads features off the bytecode (opcodes,
  intrinsics, globals/statics, member names, constant regex patterns), plus a token pass
  (labels, sloppy tokens, keyword counts). The same analysis runs on pi's
  PRELUDE_SOURCE for the "prelude needs" column.
- Data: 2,627 executions, 2,535 distinct scripts, 348 sessions, 2026-09-25..11-01.
  12 scripts don't compile in djs; all 12 fail in qjs too (mostly Python).
  A token-level cross-check agrees with every "never" zero.
- Result: the extended features (classes, generators, accessors, Proxy/Reflect, Symbol,
  weak refs, tagged templates, eval, BigInt, typed arrays, Intl, defineProperty/freeze,
  arguments, labels, sloppy, named groups/\p/lookahead) are used by 0 scripts.
  pi's prelude uses Proxy, Reflect, accessors, Object reflection, Symbols, generator and
  async-generator syntax and needs Int8Array.
- Size variants (scripts/size/variant.sh): base 460,886 gz; without
  regexp-unicode-props 439,929 (−21.0 KB); without unicode-normalization 441,125
  (−19.8 KB); without both 420,140 (−40.7 KB, 1.47x quickjs-wasi, under D020).
- Report: docs/codemode-feature-usage.md. No guide pages affected (research doc, not
  guide). No engine changes.

## Follow-ups

- Ship the pi/codemode wasm module without `regexp-unicode-props` and
  `unicode-normalization` (−40.7 KB gz, gets under the D020 budget); decide whether
  the size track measures that build.
- If more cuts are wanted: cargo features for typed arrays (needs a `typeof Int8Array`
  guard in pi's prelude), BigInt, Iterator helpers, decorators, `using`/DisposableStack,
  weak references; each needs a variant build to confirm the estimate, a slim-build
  test262 expectation list and corpus skip directives.
- Re-run the analyzer on other users' sessions before cutting anything beyond the
  Unicode features.

## Result

docs/codemode-feature-usage.md (usage tables never/rare/common with sessions and dates,
prelude needs, dynamic uses, candidates with sizes and code pointers, risks) and
scripts/harvest/features.py (the analyzer; README has the steps).
