# @earendil-works/djs

djs is a small JavaScript and TypeScript engine compiled to WebAssembly, made for running
untrusted scripts inside a host: agent tool scripts, plugins, user code. Every sandbox is
its own wasm instance with its own heap, so sandboxes share nothing. Scripts get only the
host functions you give them, run under a fuel budget and a memory cap, and a run that
waits on the host can be suspended into a snapshot and continued later in another
process.

The module has no imports and the package no dependencies: it runs in node (20 or newer),
Bun, Deno, browsers and web workers.

## Install

```sh
npm install @earendil-works/djs
```

## Use

```js
import { Sandbox } from "@earendil-works/djs";

const sandbox = await Sandbox.create({
  asyncFunctions: { "tools.search": async (q) => search(q) },
  maxFuel: 50_000_000,
  memoryLimit: 64 << 20,
});

// A script is the body of an async function: its parameters are the keys of the second
// argument, and `await` and `return` work at the top level.
const hits = await sandbox.runAsync("return (await tools.search(query)).slice(0, 3)", { query: "djs" });
sandbox.run("return a + b", { a: 40, b: 2 }); // 42
sandbox.dispose();
```

Options include `typescript: true` (scripts are TypeScript), `console` (where
`console.log` goes), `globals`, `deterministic` (repeatable runs) and `frozenBuiltins`.
Errors are `SandboxError`s with a `kind`: `threw`, `syntax`, `fuel`, `memory`, ...

## Snapshots

A host can drive a run itself: `begin` runs the script until it waits on async host
calls, which it lists instead of calling them, and the host settles each call and
advances the run. While the run waits, `run.snapshot()` writes it as bytes the host keeps,
and a sandbox made with the same options continues it, in this or another process. The
sandbox's host functions and values are named in it, not written.

```js
const sandbox = await Sandbox.create({ asyncFunctions });
const run = sandbox.begin(code, params);    // idle: run.calls lists the open calls
const snapshot = run.snapshot();
// later, anywhere:
const again = (await Sandbox.create({ asyncFunctions })).loadSnapshot(snapshot);
again.settle(again.calls[0].id, { ok: true, value: 42 });
await again.advance();                      // to the next idle point, or the end
```

With `deterministic`, `run.deliveries` lists the results the run applied, and
`begin(code, params, { deliveries })` runs the script again to where it was, for when a
newer djs no longer loads a snapshot.

## Loading the module

`Sandbox.create` loads the package's module once per process and caches it;
`loadModule(location)` loads another one and `new Sandbox({ module })` uses it. In node,
Bun and Deno the module is read from the package's `wasm/` directory (`$DJS_WASM` names
another module). In a browser it is fetched from the URL
`new URL("../wasm/djs.wasm", import.meta.url)`, a pattern that webpack and Vite recognize
and copy into the build. In Vite's dev server, exclude the package from dependency
pre-bundling, which would move the code away from its modules:

```js
// vite.config.js
export default { optimizeDeps: { exclude: ["@earendil-works/djs"] } };
```

In Deno, allow reading the package's directory (`--allow-read`). With a bundler that does
not know the pattern, serve the `.wasm` file yourself and pass its URL:
`Sandbox.create({ module: await loadModule("/assets/djs.wasm") })`.

## Documentation

The guide, with the whole API, the language djs runs, limits, determinism and snapshots:
https://earendil-works.github.io/djs/js-api.html

## License

MIT or Apache-2.0, at your option.
