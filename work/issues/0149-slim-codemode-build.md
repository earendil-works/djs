# 0149: A trimmed-down djs for codemode: minimal globals, flag or build? (research)

status: in-progress
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

## Follow-ups

## Result
