// The engines of the benchmarks page, behind one interface: a compiled module (loaded
// once, as a host keeps it), `create` (a new instance and VM, isolated from every other),
// `run` (set the tool answers, evaluate, drain the job queue, read the result string) and
// the instance's linear memory. scripts/bench-engines.sh fetches and builds them into
// vendor/engines of the primary checkout (DJS_ENGINES overrides the directory).

import { execFileSync } from "node:child_process";
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { WASI } from "node:wasi";
import { Sandbox } from "../../src/sandbox.ts";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "../../..");
const common = execFileSync("git", ["rev-parse", "--path-format=absolute", "--git-common-dir"], { cwd: root, encoding: "utf8" }).trim();
export const VENDOR = process.env.DJS_ENGINES ?? join(dirname(common), "vendor/engines");
const NPM = join(VENDOR, "npm/node_modules");
export const PRELUDE = readFileSync(join(here, "prelude.js"), "utf8");
export const PRELUDE_PY = readFileSync(join(here, "prelude.py"), "utf8");

export interface Vm {
  /** Runs a program (`Engine.program`'s output) with `data` as the global `__DATA`; the result string. */
  run(program: string, data: string): string | Promise<string>;
  /** Bytes of the instance's linear memory (wasm memory never shrinks: the high-water mark). */
  memory(): number;
  dispose(): void;
}

export interface Engine {
  name: string;
  /** What it is and how it is built, for the page. */
  version: string;
  language: "js" | "python";
  /** Whether it runs TypeScript itself (else the host strips the types first). */
  typescript: boolean;
  /** The wasm files an embedder ships, and the JS glue around them. */
  files: string[];
  glue: string[];
  load(): Promise<void>;
  create(opts?: { typescript?: boolean }): Vm | Promise<Vm>;
  /** The source the VM evaluates for a workload body. */
  program(body: string): string;
}

const tail = 'const __r = await (async () => {\n%BODY%\n})();\nreturn __out.join("\\n") + "\\n=> " + __s(__r);';
/** A codemode body (top-level await and return): djs runs it as given. */
export const bodyProgram = (body: string) => `${PRELUDE}\n${tail.replace("%BODY%", () => body)}`;
/** A script for engines without codemode bodies: an async function whose result lands in `__result`. */
const scriptProgram = (body: string) =>
  `(async () => {\n${bodyProgram(body)}\n})().then((v) => { globalThis.__result = v; }, (e) => { globalThis.__result = "ERROR " + ((e && e.stack) || e); });`;

const compile = (path: string) => WebAssembly.compile(readFileSync(path));
const pkgVersion = (dir: string) => JSON.parse(readFileSync(join(dir, "package.json"), "utf8")).version as string;

// ---- djs ---------------------------------------------------------------------------------

export function djs(wasm = process.env.DJS_WASM ?? join(root, "target/wasm32-unknown-unknown/wasm/djs_wasm.wasm")): Engine {
  let module: WebAssembly.Module;
  return {
    name: "djs",
    version: "djs-wasm, this commit (scripts/build-wasm.sh)",
    language: "js",
    typescript: true,
    files: [wasm],
    glue: [],
    async load() {
      module = await compile(wasm);
    },
    create(opts = {}) {
      const sb = new Sandbox({ module, typescript: opts.typescript, console: { log() {}, info() {}, warn() {}, error() {} } });
      return {
        run: (program, data) => String(sb.run(program, { __DATA: data })),
        // biome-ignore lint: the instance's exports are internal to the Sandbox
        memory: () => (sb as unknown as { x: { memory: WebAssembly.Memory } }).x.memory.buffer.byteLength,
        dispose: () => sb.dispose(),
      };
    },
    program: bodyProgram,
  };
}

// ---- quickjs-wasi (QuickJS-NG for WASI with snapshots) -------------------------------------

export function quickjsWasi(): Engine {
  const pkg = join(NPM, "quickjs-wasi");
  let QuickJS: any;
  let module: WebAssembly.Module;
  return {
    name: "quickjs-wasi",
    get version() {
      return `quickjs-wasi ${pkgVersion(pkg)} (npm; QuickJS-NG for WASI)`;
    },
    language: "js",
    typescript: false,
    files: [join(pkg, "quickjs.wasm")],
    glue: [join(pkg, "dist/index.js"), join(pkg, "dist/wasi-shim.js")],
    async load() {
      ({ QuickJS } = await import(pathToFileURL(join(pkg, "dist/index.js")).href));
      module = await compile(join(pkg, "quickjs.wasm"));
    },
    async create() {
      const vm = await QuickJS.create({ wasm: module });
      return {
        run(program, data) {
          const s = vm.newString(data);
          vm.setProp(vm.global, "__DATA", s);
          s.dispose();
          vm.evalCode(program).dispose();
          while (vm.executePendingJobs() > 0);
          const k = vm.newString("__result");
          const r = vm.getProp(vm.global, k);
          k.dispose();
          const v = vm.dump(r);
          r.dispose();
          return String(v);
        },
        memory: () => vm.exports.memory.buffer.byteLength,
        dispose: () => vm.dispose(),
      };
    },
    program: scriptProgram,
  };
}

// ---- quickjs-emscripten (Bellard's QuickJS and QuickJS-NG, built with emscripten) -----------

export function quickjsEmscripten(flavor: "bellard" | "ng"): Engine {
  const name = flavor === "ng" ? "@jitl/quickjs-ng-wasmfile-release-sync" : "@jitl/quickjs-wasmfile-release-sync";
  const pkg = join(NPM, name);
  const core = join(NPM, "quickjs-emscripten-core");
  const wasm = join(pkg, "dist/emscripten-module.wasm");
  let variant: unknown;
  let api: any;
  return {
    name: flavor === "ng" ? "quickjs-emscripten (ng)" : "quickjs-emscripten",
    get version() {
      return `quickjs-emscripten ${pkgVersion(pkg)}, ${flavor === "ng" ? "QuickJS-NG 0.12.1" : "QuickJS 2025-09-13"} (npm ${name})`;
    },
    language: "js",
    typescript: false,
    files: [wasm],
    glue: [join(pkg, "dist/emscripten-module.mjs"), join(pkg, "dist/ffi.mjs"), join(core, "dist/index.mjs")],
    async load() {
      api = await import(pathToFileURL(join(core, "dist/index.mjs")).href);
      const base = (await import(pathToFileURL(join(pkg, "dist/index.mjs")).href)).default;
      const module = await compile(wasm);
      variant = api.newVariant(base, { wasmModule: module });
    },
    async create() {
      // A module per VM: a fresh instance, isolated, its memory measured alone.
      const m = await api.newQuickJSWASMModuleFromVariant(variant);
      const ctx = m.newContext();
      return {
        run(program, data) {
          const s = ctx.newString(data);
          ctx.setProp(ctx.global, "__DATA", s);
          s.dispose();
          const r = ctx.evalCode(program);
          if (r.error) {
            const e = ctx.dump(r.error);
            r.error.dispose();
            throw new Error(JSON.stringify(e));
          }
          r.value.dispose();
          for (;;) {
            const j = ctx.runtime.executePendingJobs();
            if (j.error) {
              j.error.dispose();
              throw new Error("job failed");
            }
            if (j.value === 0) break;
          }
          const v = ctx.getProp(ctx.global, "__result");
          const out = ctx.dump(v);
          v.dispose();
          return String(out);
        },
        memory: () => m.getWasmMemory().buffer.byteLength,
        dispose: () => {
          ctx.dispose();
        },
      };
    },
    program: scriptProgram,
  };
}

// ---- QuickJS releases built with wasi-sdk (js/bench/engines/driver.c) ----------------------

export function quickjsBuilt(flavor: "bellard" | "ng"): Engine {
  const dir = join(VENDOR, "build");
  const wasm = join(dir, flavor === "ng" ? "quickjs-ng.wasm" : "quickjs.wasm");
  let module: WebAssembly.Module;
  const enc = new TextEncoder();
  const dec = new TextDecoder();
  return {
    name: flavor === "ng" ? "quickjs-ng" : "quickjs",
    get version() {
      return existsSync(wasm + ".version") ? readFileSync(wasm + ".version", "utf8").trim() : "?";
    },
    language: "js",
    typescript: false,
    files: [wasm],
    glue: [],
    async load() {
      module = await compile(wasm);
    },
    async create() {
      const wasi = new WASI({ version: "preview1", returnOnExit: true });
      const instance = await WebAssembly.instantiate(module, wasi.getImportObject() as WebAssembly.Imports);
      wasi.initialize(instance);
      const x = instance.exports as any;
      if (x.bench_new() !== 0) throw new Error("bench_new failed");
      const put = (s: string): [number, number] => {
        const b = enc.encode(s);
        const p = x.malloc(b.length + 1);
        const m = new Uint8Array(x.memory.buffer, p, b.length + 1);
        m.set(b);
        m[b.length] = 0;
        return [p, b.length];
      };
      return {
        run(program, data) {
          const [dp, dl] = put(data);
          x.bench_set_global(dp, dl);
          x.free(dp);
          const [pp, pl] = put(program);
          const ok = x.bench_eval(pp, pl);
          x.free(pp);
          const s = dec.decode(new Uint8Array(x.memory.buffer, x.bench_result_ptr(), x.bench_result_len()));
          if (ok !== 0) throw new Error(s);
          return s;
        },
        memory: () => x.memory.buffer.byteLength,
        dispose: () => x.bench_free(),
      };
    },
    program: scriptProgram,
  };
}

// ---- monty (pydantic's Python subset), its wasm component ---------------------------------

/**
 * Monty's wasm build: the component `@pydantic/monty/wasm` runs in worker threads,
 * instantiated here on the calling thread the way its worker does (no filesystem,
 * environment or network), so a run is timed without a thread hop. The run is one `feed`
 * of the program, its value the last expression's.
 */
export function monty(): Engine {
  const pkg = join(NPM, "@pydantic/monty");
  const dir = join(pkg, "dist/worker/component");
  let modules: Record<string, WebAssembly.Module>;
  let instantiate: any;
  let WASIShim: any;
  let names: string[];
  return {
    name: "monty",
    get version() {
      return `monty ${pkgVersion(pkg)} (npm @pydantic/monty, its wasm component)`;
    },
    language: "python",
    typescript: false,
    get files() {
      return readdirSync(dir).filter((f) => f.endsWith(".wasm")).map((f) => join(dir, f));
    },
    glue: [join(dir, "monty.component.js")],
    async load() {
      ({ COMPONENT_MODULE_NAMES: names } = await import(pathToFileURL(join(pkg, "dist/worker/componentModules.js")).href));
      ({ instantiate } = await import(pathToFileURL(join(dir, "monty.component.js")).href));
      ({ WASIShim } = await import(pathToFileURL(join(NPM, "@bytecodealliance/preview2-shim/dist/common/instantiation.js")).href));
      modules = Object.fromEntries(await Promise.all(names.map(async (n) => [n, await compile(join(dir, n))])));
    },
    async create() {
      const memories: WebAssembly.Memory[] = [];
      const core = async (m: WebAssembly.Module, imports: WebAssembly.Imports) => {
        const i = await WebAssembly.instantiate(m, imports);
        const mem = Object.values(i.exports).find((e) => e instanceof WebAssembly.Memory);
        if (mem) memories.push(mem as WebAssembly.Memory);
        return i;
      };
      const shim = new WASIShim({ sandbox: { preopens: {}, env: {}, args: [] } }).getImportObject();
      const deny = () => {
        throw new Error("monty requested process exit");
      };
      const imports = { ...shim, "wasi:cli/exit": { exit: deny, exitWithCode: deny } };
      const c = await instantiate((p: string) => modules[p.replace(/^\.\//, "")], imports, core);
      const dispatch = c.worker.dispatch;
      const ok = (r: any) => {
        const bad = r.events.find((e: any) => e.tag === "error" || e.tag === "fatal-error" || e.tag === "typing-error");
        if (bad) throw new Error(typeof bad.val === "string" ? bad.val : bad.val.traceback || bad.val.message);
        return r;
      };
      ok(dispatch({ tag: "configure", val: { scriptName: "bench.py", typeCheck: false, typeCheckFormat: "concise", typeCheckColor: false } }));
      return {
        run(program, data) {
          const r = ok(dispatch({ tag: "feed", val: { code: program, inputs: [{ name: "__DATA", value: 0 }], values: { nodes: [{ tag: "text", val: data }] }, skipTypeCheck: true, cwd: "/" } }));
          const done = r.events.find((e: any) => e.tag === "complete");
          if (!done) throw new Error(`monty stopped with ${r.events.map((e: any) => e.tag).join(", ")}`);
          const node = done.val.values.nodes[done.val.value];
          if (node.tag !== "text") throw new Error(`monty returned ${node.tag}`);
          return node.val;
        },
        memory: () => memories.reduce((s, m) => s + m.buffer.byteLength, 0),
        dispose() {},
      };
    },
    program: (body) =>
      `${PRELUDE_PY}\n__result = None\n${body}\n"\\n".join(__out) + "\\n=> " + ("undefined" if __result is None else __s(__result))`,
  };
}

export function allJs(): Engine[] {
  return [djs(), quickjsWasi(), quickjsBuilt("ng"), quickjsBuilt("bellard"), quickjsEmscripten("ng"), quickjsEmscripten("bellard")];
}
