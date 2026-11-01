# 0130: Which JS features does real codemode use? (research: what could be cut)

status: in-progress
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

## Follow-ups

## Result
