# 0149: A trimmed-down djs for codemode: minimal globals, flag or build? (research)

status: done
agent: slim
branch: issue/0149-slim-codemode
depends-on: 0130, 0144, 0145, 0146, 0147
guide: -
touches: docs/slim-codemode.md (report), scripts/harvest/ (analyzer extensions), work/scratch/0149/ (raw data, prototypes; never committed); no engine changes on main

## Owner
"Investigate what a build of this engine looks like that is trimmed down to the minimum set
of things to make it work for the current uses of codemode in ~/.pi/agent/sessions and
~/.pi/p2 sessions. Presumably we can get away with much fewer globals. Investigate, don't
make the change yet. Come up with a reduced set of globals, a minimal version that works
well enough for codemode without pulling everything in. It might even be acceptable to
remove functions in that mode. Also investigate whether this is a separate build of djs or
just a flag, and the impact on the wasm binding."

## Data and privacy
- Inputs: codemode executions in ~/.pi/agent/sessions (0118/0130's harvester reads them)
  and in ~/.pi/p2/sessions (the p2 app's session store: find its format; read-only).
- **Only read the session data.** ~/.pi/p2 also holds credentials and keys (auth.json,
  owner-key, owner-token.txt, radius.json, device.json, the sqlite databases): never open,
  copy, print or commit anything but ~/.pi/p2/sessions.
- Raw scripts stay in work/scratch/0149; the report has feature/global names, counts and
  dates only (D021, D2281 rules).

## Questions
1. Usage, at the level of **globals and their members** (not just syntax): which globals,
   constructors, static functions and prototype methods do real scripts call, across both
   session stores (extend 0130's analyzer: features.py; plus a dynamic count by running
   the harvested corpus with a call-counting build, as 0143 did with touch.py)? Which are
   never used? Include what pi's own prelude needs (Proxy for `tools`, Reflect, Symbols,
   etc.), and p2's prelude/host if it differs.
2. A proposed **minimal global set** in tiers: required / keep-because-cheap-or-expected
   by models / drop. For each dropped family: what a script would see (ReferenceError at
   use, missing method TypeError), and the risk that models write code using it (how often
   do LLM-written scripts reach for it in general, not only in this sample?).
3. **What dropping buys**: base image size and cell count (now the immutable prefix),
   module code size (Rust natives, bgen fast paths, Unicode tables, regexp features,
   Intl-ish, BigInt, typed arrays, Proxy?, Temporal?...), vm_new, snapshot size; measure
   with variant builds (cargo features where they exist; prototypes in scratch otherwise).
4. **Flag or build**: which savings need a separate module (code size: Rust code can only
   go away at compile time) vs which a VM option can give (fewer globals installed, a
   smaller prefix: two base images in one module? one base with a slim realm view?).
   Interaction with frozen builtins (0146), the immutable prefix (0147), the registry
   (0145: ids of absent builtins), snapshots (a slim VM's image must not restore into a
   full one or vice versa?), determinism and the gate (test262 can't run on a slim build:
   what tests it instead?).
5. **Impact on the wasm binding** (djs-wasm exports, js/src Sandbox, js/pi adapter): one
   module with a mode, two modules, or features chosen at build; what changes for hosts.
6. A recommendation with numbers.

## Deliverable
docs/slim-codemode.md.

## Log
- Extended scripts/harvest/extract.py to read p2's SQLite session databases (pi-durable
  `entries`; D2410), from a read-only copy in work/scratch/0149. 2,757 agent-store + 25 p2
  executions, 2,687 distinct scripts. Nothing else under ~/.pi/p2 opened.
- New scripts/harvest/members.py: usage per global and member, joined with the registry
  and the touch build's per-case counts.
- Dynamic: 0143's touch.py on a scratch copy + a `touched-names` example (registry
  paths); 310 codemode cases.
- Prototypes (scratch): slim_patch.py prunes families from the base before
  prepare_base_image; leaf arms aliased away; decorator/`using` syntax cut. Measured
  main 458,952 gz → A full 399,605 (−59.3 KB), B full 395,626. Base 325,008 → 238,712 B.
  Prototype A passes all 310 codemode cases as main does, and runs pi's lockdown-free
  prelude; pi's lockdown needs Int8Array.
- A run-time flag can't free the prefix (D2064): pruning a VM from the base keeps the
  image at 324,872 B; deriving from Vm::new costs ~10 ms native.
- Guide pages: none affected (research; no engine change).

## Follow-ups
- Ship pi's module without the Unicode cargo features (−34.5 KB gz); pi-private: frozen
  builtins on, lockdown off.
- `slim` cargo feature phase 1 (image + leaves + syntax, clear "not available in
  codemode" errors, second wasm build, surface test, corpora and pi tests on it).
- Phase 2: BigInt and typed arrays out of the engine core; filtered test262 for slim.

## Result
docs/slim-codemode.md: usage by global and member over both session stores, the proposed
tiers (drop: typed arrays/ArrayBuffer/DataView, Iterator helpers, BigInt, WeakRef/
FinalizationRegistry, DisposableStack/`using`, decorators, the Unicode features; Proxy and
Reflect only with a pi change), measured prototype sizes, flag vs build (a build: a `slim`
cargo feature and a second module), binding impact and a phased recommendation.
scripts/harvest: extract.py reads p2 databases, members.py. D2410.
