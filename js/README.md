# @earendil-works/djs

djs is a small JavaScript and TypeScript engine compiled to WebAssembly, made for running
untrusted scripts inside a host: agent tool scripts, plugins, user code. Every sandbox is
its own wasm instance with its own heap, so sandboxes share nothing. Scripts get only the
host functions you give them, run under a fuel budget and a memory cap, and a run that
waits on the host can be suspended into a dump, a plain JavaScript file, and continued
later in another process.

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

## Entry points

| import | module | for |
| --- | --- | --- |
| `@earendil-works/djs` | `wasm/djs.wasm` | every host |
| `@earendil-works/djs/slim` | `wasm/djs-slim.wasm` | hosts that count kilobytes: no typed arrays, BigInt, iterator helpers and other families scripts rarely use |
| `@earendil-works/djs/dump` | `wasm/djs-dump.wasm` | hosts that suspend runs into dumps: the main module plus the dumper |
| `@earendil-works/djs/dump-runtime` | | the helpers a dump needs to continue in a JavaScript engine other than djs |

Each entry point exports the same API, with its own module as the default. To dump runs
while running them in the smaller main module, give the sandbox the dump module as its
`dumpModule`:

```js
import { Sandbox } from "@earendil-works/djs";
import { loadModule as loadDumpModule } from "@earendil-works/djs/dump";

const sandbox = await Sandbox.create({ frozenBuiltins: true, dumpModule: await loadDumpModule(), asyncFunctions });
const task = sandbox.start(code, params);
const dump = await task.suspend(); // later, anywhere: (await Sandbox.resume(dump, options)).result
```

## Loading the module

`Sandbox.create` loads its entry point's module once per process and caches it;
`loadModule(location)` loads another one and `new Sandbox({ module })` uses it. In node,
Bun and Deno the module is read from the package's `wasm/` directory (`$DJS_WASM` names
another main module). In a browser it is fetched from the URL
`new URL("../wasm/djs.wasm", import.meta.url)`, a pattern that webpack, Vite, Rollup and
Parcel recognize and copy into the build. In Vite's dev server, exclude the package from
dependency pre-bundling, which would move the code away from its modules:

```js
// vite.config.js
export default { optimizeDeps: { exclude: ["@earendil-works/djs"] } };
```

Elsewhere, serve the `.wasm` file yourself and pass its URL:
`Sandbox.create({ module: await loadModule("/assets/djs.wasm") })`.

## Documentation

The guide, with the whole API, the language djs runs, limits, determinism and dumps:
https://earendil-works.github.io/djs/js-api.html

## License

MIT or Apache-2.0, at your option.
