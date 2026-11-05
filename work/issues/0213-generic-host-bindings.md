# 0213: Generic host bindings: `tools.foo` stops being special

status: done
agent: hb
branch: issue/0213-generic-host-bindings
depends-on: -
guide: host.md (and wherever `tools` is described as special)
touches: design first (work/research/0213/); then js/src/sandbox.ts, js/pi/, crates/djs-cli (tools.rs, run.rs, dumps.rs, disasm.rs, strip.rs), crates/djs-docs/src/run.rs, vm/dump*.rs (host function naming), vm/dump_harvest.rs, docs

## Goal
Owner direction: bridge calls must be able to look different from `tools.foo(...)`. DSH wants
```js
import { myTool } from "@sdk/our-stuff";
const result = await myTool("param");
```
Imports are not needed yet, but nothing may assume host functions live on a `tools` object.
Make the host-binding model generic now so that imports later are only a new way to bind the
same host functions.

Known places where `tools` (and `console`) are special today:
- codemode bodies are compiled with parameters `(tools, console)` in the CLI (run.rs, disasm.rs,
  strip.rs), djs-docs, pi's SCRIPT_PREFIX (`return (async (tools, console) => {`), and the dump
  writes a body as `async function (tools, console) {…}` (D2617, params from Script::PARAMS);
- dumps refer to a host function held as a value by its name *as an expression* evaluated at
  load (`tools.read`), and to open calls by that name (`$djs.resume(id, "tools.sleep", …)`);
  host globals that are never dumped are a host-given list (`DumpOptions::host`, the CLI's
  HOST_GLOBALS, dump_harvest's list);
- the Sandbox makes a namespace object for dotted names (`tools.read`);
- pi's next mode implements tools through a prelude and the `$djs.bridge`/`$djs.call` names
  (PLAIN_FN_NAMES);
- the CLI's Tools (tools.rs) looks up the `tools` global.

## Plan
1. **Inventory and design** (work/research/0213/README.md), sent to the coordinator before
   building. It should answer:
   - what identifies a host function (a host-chosen name, e.g. `"@sdk/our-stuff#myTool"` or
     `"tools.read"`), independent of where script code finds it;
   - how scripts get host functions: globals, namespace objects, codemode body parameters
     chosen by the host (a stand-in for imports: `myTool` bound as a body binding), and later
     real import bindings, all resolving to the same identity;
   - how dumps refer to them: e.g. a `$djs.host(name)` hook (a plain function, D2834) instead
     of evaluating a name expression, so a dump doesn't depend on where the function was bound;
     open calls keep `$djs.resume(id, name, args, state)` with the identity as name;
   - which hosts change and how (Sandbox options, pi adapter and prelude, CLI, docs runner), and
     what `tools`/`console` become (just the default bindings a host chooses);
   - how this maps to imports later (module specifier + export name → identity; a dumped
     binding re-imported or re-bound), without building imports now.
2. **Implement** after the coordinator's OK, in a few batched merges, with tests that use a
   non-`tools` binding end to end (a bare `myTool` body binding, a namespace other than `tools`)
   including dumps at every idle point and a resume in node.

## Constraints
Other agents: pjr (pi's `$djs.source` in js/pi/vm.ts and hosts), pjs (dump_cont.rs,
dump_plain.rs), pjd (dead format-2 code in dump.rs/dump_flight.rs), pjz (fuzzer). Design now;
before implementing in js/pi or the dump files, ask the coordinator for sequencing. Batch
merges (at most one every 1-2 hours), run the gate's checks locally first. Decision IDs
D2970-D2989.

## Log
- Phase 1 (design): inventory and proposal in work/research/0213/README.md. Finding: the engine is
  already generic about bodies (Script::PARAMS, D2617/D2882); the coupling is that dumps name host values
  by global path expressions (dump.rs HostFunction writer, `host_values`), from which pi's `$djs.call`
  (D2893), the D2618 `let tools = tools` case and the hosts' global lists follow. Proposal: identities +
  one object per identity, bindings (globals, namespaces, body bindings, later imports) to identities,
  `$djs.host(name)` hook in dumps, Sandbox `globals`/`bindings` options, CLI `--bind`. Sent to the
  coordinator with open questions; waiting for OK before implementing.
- Coordinator OK with answers: `$djs.host` covers roots too; Sandbox bindings per sandbox (namespaces
  bindable as body bindings); dotted-name globals kept as documented sugar for an explicit `globals` entry;
  CLI answers by full name only; batch 1 (engine, wasm, Rust hosts) now, rebased onto 0212 before merging;
  batch 3 (pi, CLI --bind) after pjr's merge.

- Batch 1 (engine, wasm, hosts' loaders): dumps write host values as `$djs.host(name)`
  (functions by identity, roots and their members as reads on the root); `DumpOptions::host_roots`,
  `vm_dump` sixth line, ABI 5; the identifier refusal and D2618's captured-global special case gone.
  Every loader had to serve the new hook in the same merge (a dump with any host value calls it),
  so batch 1 also carries: CLI (`Tools::host_value`), difftest, e2e/unit test loaders,
  scripts/plain-node.mjs, examples/sleep-dump, the Sandbox (namespaces incl. `console` are roots,
  one function handle per identity, `host` hook) and pi's adapter (identities `bridge`/`call`,
  supersedes D2893 as D2973; `host` answers with the prelude's own functions and its globals).
  `Vm::global_path` for hosts that bind by path. Tests: dump_tests (myTool bound only as a
  parameter, an `sdk` root that is no global, identity after load), Sandbox dumpresume case
  "held host values" (`sdk.fetch`), CLI every-idle-point case with held tools. dump-size score
  +0.7% raw, +0.5% gzip (updated). Decisions D2970, D2971, D2973. Docs: host.md,
  plain-js-dumps.md, continuation-dumps.md, guide host.md. pi --next --dump-every-idle on a debug
  module: 2 timing failures (a 5 s timeout, debug speed), not dump-related.
- Batch 1 merged (main 1537eda6) after a rebase that needed the dump-size score regenerated.
- Batch 2 (with what was batch 3's CLI part; pi was done in batch 1): Sandbox `globals` and
  `bindings` (identity-keyed functions; `ns.member` makes namespace `ns`; the written default rule;
  misuse for unknown identities or a parameter/binding clash; one handle per identity carried through
  the dump module's image), js/test/bindings.test.ts (explicit globals, the default rule, a bare
  `myTool` + `sdk` namespace as body bindings dumped at every idle point and loaded, a suspended task
  resumed in a sandbox and in node with identity kept). CLI: identities everywhere (`tools` table
  keyed by identity, Proxy maps `k` to `tools.k`, `has` trap), `--bind NAME=ID`, answers keyed by
  identity (fixtures and docs/cli.md updated), `BODY_PARAMS` shared by run/disasm/strip, a CLI test
  of a bound `myTool` at every idle point. djs-docs runner: bodies without parameters (guide
  disassembly regenerated; compiler.md's sentence on `console` rewritten). D2972. Guide pages
  updated: host.md (host calls, dumps), snapshots.md (the example dump), compiler.md.

## Follow-ups

## Result
Host functions are no longer special as `tools.foo`:
- **Identity** (D2970): a host-chosen string; one function object per identity per VM.
- **Dumps** (D2971): host values are `$djs.host(name)` (functions by identity, roots such as
  namespaces and pi's `tools` Proxy by name, their members as reads on the root); `DumpOptions::host_roots`,
  `vm_dump`'s roots line, ABI 5; every loader (Sandbox, pi, CLI, difftest, node scripts, tests) serves
  the hook. pi's functions are `bridge`/`call` (D2973, supersedes D2893).
- **Bindings** (D2972): Sandbox `globals` and `bindings` (body parameters, the stand-in for imports;
  namespaces bindable too), the dotted-name rule documented as the default; CLI `--bind NAME=ID`,
  answers by identity; the docs runner has no body parameters.
- Imports later: a resolver (specifier, export) → identity (default `specifier#export`), the binding
  holds that identity's object; dumps need nothing new.
Known gaps: per-run bindings (left out until needed); the CLI's `print` and `console.log` share one
id, so a held `console.log` comes back as `print`'s object after a load (as before); pi's own prelude
still binds `tools`/`console` (pi's choice).
