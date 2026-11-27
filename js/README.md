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

A run that waits on the host can be suspended into a snapshot, bytes the host keeps, and
continued by a sandbox made with the same options, in this or another process. The
sandbox's host functions and values are named in it, not written.

```js
const sandbox = await Sandbox.create({ asyncFunctions });
const task = sandbox.start(code, params);
const snapshot = await task.suspend(); // later, anywhere: (await Sandbox.resume(snapshot, options)).result
```

A host-driven run (`sandbox.begin`) has `run.snapshot()` and `sandbox.loadSnapshot(bytes)`.
The package's module also writes continuation dumps (`run.dump()`, with frozen builtins),
JavaScript that recreates a run; to continue one in a JavaScript engine other than djs,
the package exports the dump runtime's `helpers` (and `script`, the same as a Script;
`scriptFor(dump)`, only the helpers a dump needs).

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
