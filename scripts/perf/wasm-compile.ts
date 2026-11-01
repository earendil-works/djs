// wasm-compile.ts [--qjs] [--reps N] [--wasm X.wasm]... files...: compile-only throughput in
// wasm (0092; wcompile.py counts its instructions).
// djs: vm_compile (Script goal, or AsyncBody for *.body.js) on a fresh Sandbox per rep,
// timing only the vm_compile call. qjs: quickjs-wasi evalCode(COMPILE_ONLY).
// ONCE=1: one compile per file and engine. SAME=1: every compile in one Sandbox (steady
// state, for node --cpu-prof).
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { Sandbox, loadModule } from "../../js/src/sandbox.ts";

const args = process.argv.slice(2);
let reps = Number(process.env.REPS ?? 5);
const wasms: string[] = [];
let qjs = false;
const files: string[] = [];
for (let i = 0; i < args.length; i++) {
  if (args[i] === "--qjs") qjs = true;
  else if (args[i] === "--reps") reps = Number(args[++i]);
  else if (args[i] === "--wasm") wasms.push(args[++i]);
  else files.push(args[i]);
}
const once = process.env.ONCE === "1";
if (once) reps = 1;
const enc = new TextEncoder();

async function djs(wasm: string, code: string, body: boolean): number {
  const module = await loadModule(wasm);
  let best = Infinity;
  const same = process.env.SAME === "1" ? (new Sandbox({ module } as never) as any) : null;
  for (let i = 0; i < (once ? 1 : reps + 1); i++) {
    const sb = same ?? (new Sandbox({ module } as never) as any);
    const x = sb.x;
    const src = enc.encode(code);
    const at = x.djs_staging(src.length);
    new Uint8Array(x.memory.buffer).set(src, at);
    const t = performance.now();
    const f = x.vm_compile(sb.vm, body ? 1 : 0, 0, src.length);
    const dt = performance.now() - t;
    if (f === 0) {
      const p = x.djs_out_ptr(), n = x.djs_out_len();
      throw new Error(new TextDecoder().decode(new Uint8Array(x.memory.buffer, p, n)));
    }
    x.vm_handle_free(sb.vm, f);
    if (!same) sb.dispose();
    else if (i % 20 === 0) x.vm_collect(sb.vm);
    if (i > 0 || once) best = Math.min(best, dt);
  }
  return best;
}

async function quick(code: string, body: boolean): number {
  const pkg = join(process.env.HOME!, "Development/pi-private/node_modules/quickjs-wasi");
  const { QuickJS } = await import(join(pkg, "dist/index.js"));
  const wasm = await WebAssembly.compile(readFileSync(join(pkg, "quickjs.wasm")));
  const src = body ? "(async () => {" + code + "\n})" : code;
  let best = Infinity;
  for (let i = 0; i < (once ? 1 : reps + 1); i++) {
    const vm = await QuickJS.create({ wasm });
    const t = performance.now();
    const r = vm.evalCode(src, "x.js", 32 /* COMPILE_ONLY */);
    const dt = performance.now() - t;
    r.dispose?.();
    vm.dispose?.();
    if (i > 0 || once) best = Math.min(best, dt);
  }
  return best;
}

for (const file of files) {
  const code = readFileSync(file, "utf8");
  const body = file.endsWith(".body.js");
  const mb = Buffer.byteLength(code) / 1e6;
  const row: string[] = [file.split("/").pop()!, (mb * 1000).toFixed(0) + " KB"];
  for (const w of wasms) {
    const ms = await djs(w, code, body);
    row.push(`${ms.toFixed(2)} ms ${(mb / (ms / 1000)).toFixed(1)} MB/s`);
  }
  if (qjs) {
    try {
      const ms = await quick(code, body);
      row.push(`qjs ${ms.toFixed(2)} ms ${(mb / (ms / 1000)).toFixed(1)} MB/s`);
    } catch (e) {
      row.push("qjs: " + String(e).slice(0, 40));
    }
  }
  console.log(row.join("\t"));
}
