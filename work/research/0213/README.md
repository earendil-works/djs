# 0213: Generic host bindings — inventory and proposal

Status: design, for the coordinator's OK. Nothing implemented.

## 1. Inventory: where `tools` / `console` are special today

### Engine (djs-core)
| place | what is special |
| --- | --- |
| `compiler::Goal::AsyncBody` | generic already: parameters are whatever `CompileOptions::params` says. Only the doc comment says `(tools, console)`. |
| `Script::PARAMS`, dump.rs `Sources::fn_text` (D2617) | generic: the body is written as `async function (<Script::PARAMS>) {…}`; `$djs.source(name, hash, params)` (D2882) carries the names. Nothing to change. |
| dump.rs `DumpOptions::host` + `host_values` (1003) | a host global's value, and its own data members down to depth 2, are written as a **path expression** evaluated at load (`tools`, `tools.read`, `console.log`). Golden: `$dump.a3_0 = tools;` for the body's parameter. |
| dump.rs `ObjectClass::HostFunction` (3942) | a host function is written as `host_fn_name(id)`, which **must be a dotted identifier path** that the loading host defines as a global; else refused ("a host function without a global name"). This is what rules out a bare `myTool` binding. |
| dump.rs `is_host_value` + 3236 (D2618) | a captured variable holding the host's global of the same name is left to the global (`let tools = tools;` would be a TDZ error): a special case caused by path expressions. |
| dump.rs 3954 `$dump.host(name, async)` | format-2 leftover (pjd is deleting it). |
| dump_harvest.rs `HOST`, `host_fn_name`, `(async (tools, console) => …)` | pi's corpus shape; host fns `output`/`done`/`later` as globals. |
| dump_perf.rs, compiler/tests.rs `BODY_PARAMS`, smap_tests, tests/it/slices.rs, tests/e2e.rs | test fixtures that use `tools, console` as body params; harmless. |

### Sandbox (js/src/sandbox.ts)
- `functions` / `asyncFunctions` are keyed by name; `fnNames[id-1]` is both the dump name and the binding.
  `defineGlobalFunction`: `a.b` → property of a namespace object `a` made on first use (D2632), else a global.
- `console` is built as host functions `console.log` … on a global object; journal and `consoleArg` key off the `console.` prefix.
- Body params of `run`/`begin`/`start` are **data** (wire values); no way to bind a host function as a body binding.
- `dumpHere`: host globals = `console` + first segment of each fn name; fn names line = `fnNames`.
- `loadPlain`: `$djs.resume(id, name, …)` looks `name` up in `fnNames`. No `host` hook.

### pi adapter (js/pi/vm.ts, tests/pi/prelude-next.ts)
- `SCRIPT_PREFIX = "return (async (tools, console) => {"`; the prelude calls `fn(toolsProxy, console)`.
- Tools are prelude JS closures over one async host fn `call(kind, name, json)`; the engine sees only `bridge` and `call`.
- `PLAIN_FN_NAMES = ["$djs.bridge", "$djs.call"]` (D2893): since a held host fn must be a global path, they are
  smuggled into `$djs` while a dump loads — as **new** host function objects, not the prelude's.
- `hostGlobals()`: fixed list `performance, tools, ALL_TOOLS, console, text, image, exit, store, load` + globals' first segments.

### CLI (crates/djs-cli)
- run.rs, disasm.rs, strip.rs: `--body` compiles with params `["tools", "console"]`; run.rs passes the globals' values.
- tools.rs: `tools` global = Proxy whose `get` calls `make(name)` → async host fn, identity `tools.<name>`;
  `id_for_call_name` / `call_name` hard-code `tools.`, `later`, `print`, `console.*`; `rescan` finds the `tools` global.
- dumps.rs `HOST_GLOBALS = ["console", "tools", "print", "later"]`; `serve_hook` resolves `$djs.resume` names via `id_for_call_name`.
- REPL uses the same Tools.

### Others
- djs-docs run.rs: `PARAMS = ["tools", "console"]`, `tools` passed as `undefined`.
- djs-difftest djs.rs: `host_fn_name` → `output`/`done`/`later` as globals.
- djs-wasm `vm_dump` staging: line 1 host globals, line 3 fn names (must be global paths).
- node loaders (scripts/plain-node.mjs, examples/sleep-dump): host fns as plain globals; `$djs` = runtime + seed/resume/source.
- Docs: docs/host.md (Sandbox, pi), docs/continuation-dumps.md "`$djs`" and exporter, docs/plain-js-dumps.md §4,
  docs/cli.md, guide host.md.

**Summary.** The engine is already generic about bodies. What ties host functions to `tools.x` is one thing:
*dumps refer to host values by a global path expression*. Everything else (pi's `$djs.call` trick, the
`let tools = tools` case, the dotted-name rule, the hosts' global lists) follows from it.

## 2. Proposal

### 2.1 Identity
A host function's **identity** is a host-chosen string, opaque to the engine, unique per host:
`tools.read`, `console.log`, `later`, `@sdk/our-stuff#myTool`, pi's `call`. It is `host_fn_name(fn_id)` (as
today, minus the "global path" requirement), the `fn` in `run.calls`, journals and `HostCall`, and the name in
`$djs.resume(id, identity, args, state)`. Convention (documented, not enforced): `specifier#export` for
functions that imports will provide, dotted for namespace members.

**One object per identity per VM.** A host creates the function object for an identity once and binds that
object wherever it appears (global, namespace member, body parameter, later import), so `===` holds across
bindings and across a dump/load.

Non-function host values (namespace objects, pi's `tools` Proxy, `performance`, `ALL_TOOLS`) are **host
roots**: (identity, value) pairs the host names for the dump. By default every global in `DumpOptions::host`
is a root under its global name (today's list keeps working).

### 2.2 Binding: where scripts find them
All bindings resolve to identities; none is privileged.
1. **Globals**: global name → identity (function or root).
2. **Namespace objects**: an object whose members are bound to identities (`tools`, `console`); itself a root.
3. **Body bindings** (the stand-in for imports): extra parameters of the codemode body, bound to identities.
   `myTool` in `await myTool("param")` is a parameter whose value is the host function `@sdk/our-stuff#myTool`.
   The body is still `Goal::AsyncBody` with `params = data params ++ bound names`; D2617/D2882 need nothing new.
4. **Imports (later)**: `import { myTool } from "@sdk/our-stuff"` → a host resolver
   `(specifier, export) → identity` (default `specifier + "#" + export`); the import binding is an immutable
   binding to that identity's object. Nothing else changes: same identity, same dump form.

`tools` and `console` become just the default bindings some hosts choose: the CLI and pi bind them as body
parameters and globals; the Sandbox binds `console` as a global and dotted identities as namespaces by default.

### 2.3 Dumps: `$djs.host(identity)`
A new hook beside `seed`/`resume`/`source` (a plain function, D2834): `$djs.host(name)` returns the host value
`name` — a host function by identity, or a root.
- A host function held as a value: `$djs.host("tools.read")` (any string; the identifier check and its refusal go).
- A root: `$djs.host("tools")`; a data member of a root (today's depth-2 walk): `$djs.host("tools").read`,
  i.e. an expression on the resolved root, so the host only resolves root names. Host functions are always
  written by identity even when also reachable from a root.
- Body parameters bound to host values: `$dump.a3_0 = $djs.host("tools");` (was `= tools`).
- D2618's "captured `tools` left to the global" special case goes: such a variable is written
  `let tools = $djs.host("tools")` like any other.
- Open calls: unchanged, `$djs.resume(id, identity, args, state)`.
- Global bindings: `DumpOptions::host` keeps its meaning ("globals the loading host defines again"; D2618's
  lexical-shadowing rule unchanged). A host-global property whose value is not that host's is dumped as before.
- Hosts: djs hosts serve `host` as a sync hook (returns the cached object for the identity, or the root's
  handle); node loaders add `host: (n) => table[n]`. Unknown name → throws, the load fails with the name.

Example (golden codemode.2): `$dump.a3_0 = $djs.host("tools"); $dump.a3_1 = $djs.host("console");`.
Size: ~13 bytes more per host reference; negligible next to data (to be measured on the corpus score, D2901).

`COMPAT_VERSION` is not affected (images unchanged). The dump format changes (unreleased, D024).

### 2.4 Host API changes
**djs-core**: `DumpOptions` gains `host_roots: &[(&str, Value)]` (explicit roots; `host` globals stay implicit
roots). `host_fn_name` keeps its type; its result is an identity. Exporter changes in dump.rs only (the
HostFunction writer, `host_values` → roots, drop the D2618 special case). New refusals: none; one removed.

**djs-wasm**: `vm_dump` staging gets a root line (`handle:name`, tab-separated, like the handles line);
`djs_abi_version` 4 → 5.

**Sandbox**:
```ts
Sandbox.create({
  asyncFunctions: { "tools.read": read, "@sdk/our-stuff#myTool": myTool }, // keyed by identity
  globals: { tools: "tools", console: "console" }, // global name → identity/namespace; default: derived (below)
  bindings: { myTool: "@sdk/our-stuff#myTool" },   // body parameters of every run → identity; default none
});
await sb.runAsync('return await myTool("param")');
```
- Default `globals` (when absent): today's rule — a plain-identifier identity is a global of that name, `a.b` is
  member `b` of namespace `a` (a root named `a`), `console` as now; identities that are not identifier paths are
  not bound globally.
- `bindings` (sandbox-wide; `RunOptions.bindings` overrides per run — open question): names after the data
  params. Values: an identity of a function or a namespace.
- Function objects cached per identity (handles held, carried through `viaDumper` like `namespaces`).
- `loadPlain` serves `host`; `dumpHere` passes roots (namespaces) and the global names actually bound.
- Console/journal: key off "built by the sandbox for console" instead of the `console.` prefix.

**pi adapter**: identities `bridge` and `call` (not `$djs.*`); the `host` hook returns the prelude's own
`bridge`/`call` handles (one object per identity, which D2893 did not give); `$djs.bridge`/`$djs.call` and
PLAIN_FN_NAMES' special meaning go (supersedes D2893). `hostGlobals()` stays as pi's chosen globals (roots by
name, resolved by reading the global). SCRIPT_PREFIX and the prelude are pi's choice and stay; a pi with
imports would change its prelude, not djs.

**CLI**: body bindings instead of hard-coded params: `--body` uses `tools,console` by default and
`--bind NAME=IDENTITY` adds one (e.g. `--bind myTool=@sdk/our-stuff#myTool`); disasm/strip take the same
default from one constant. tools.rs's `made` map is keyed by identity (`tools.<name>` via the Proxy, any other
identity via `--bind` or `$djs.host`), answers files keyed by identity (`tools.read` and also bare `read` for
`tools.` members — open question), `HOST_GLOBALS` stays as the CLI's chosen globals. `serve_hook` serves
`host`.

**djs-docs runner**: no body params (`console` is a global; `tools` was `undefined`).

**difftest / harvest / node loaders**: serve `host` (`output`, `done`, `later`), in Rust and in
scripts/plain-node.mjs, examples/sleep-dump.

## 3. Tests (phase 2)
- dump unit tests: a host function bound only as a body parameter (`myTool`), and a namespace other than
  `tools` (`sdk.fetch`): dump at every idle point, the dump reads `$djs.host(...)`, loads in djs and continues.
- Sandbox: `bindings: { myTool: … }` end to end: task suspend/resume, `begin`/`dump`/`loadRun`, identity
  (`myTool === $djs.host(...)` after load), a resume in node (sleep-dump style).
- CLI: `djs run --body --bind myTool=… --dump-every-idle`.
- pi: the existing next/dump-every-idle suites with the new names.

## 4. Merge plan (after OK; sequencing with pjs/pjd/pjr)
1. Engine + wasm + Rust hosts (dump.rs HostFunction/roots, `host` hook in CLI, difftest, harvest; ABI 5),
   goldens regenerated. Touches dump.rs (pjd: dead format-2 code there) — needs a slot.
2. Sandbox (`globals`/`bindings`, `host` hook) + node loaders + docs/host.md, continuation-dumps.md.
3. pi adapter (`bridge`/`call` identities) after pjr's js/pi/vm.ts work; CLI `--bind`; decision entries.

## 5. Decisions to record (D2970–)
- D2970: a host function is identified by a host-chosen string, one object per identity per VM; bindings
  (globals, namespaces, body parameters, later imports) refer to identities.
- D2971: dumps refer to host values by `$djs.host(name)` (functions by identity, roots by name, members as
  property reads on a root), never by a global expression; supersedes the path rule of D2890 and D2893.
- D2972: body bindings are the stand-in for imports: host-chosen parameters of the codemode body.

## 6. Open questions
1. `$djs.host` for roots too (proposed), or only for functions, keeping global path expressions for
   non-function host objects (smaller diff, but pi's `tools` Proxy stays a global path)?
2. Sandbox body bindings: sandbox-wide `bindings` only, or also per run? Should namespaces be bindable as body
   parameters (`bindings: { sdk: "sdk" }`)?
3. Default global binding of dotted identities: keep (convenient, all tests/examples use `tools.sleep`), or
   make globals explicit only?
4. CLI answers files: keyed by identity only (`"tools.read"`), or keep bare tool names for `tools.*`?
5. Sequencing of the dump.rs change against pjd/pjs.
