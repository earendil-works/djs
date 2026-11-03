// The wasm twin of crates/djs-difftest/src/bin/djs-abidrive.rs: the same export calls in
// the same order, so native and wasm images can be compared (0072).
// usage: node abidrive.mjs <wasm> <body.js> <slice> [trace-every] [raw-out]
import { readFileSync, writeFileSync } from "node:fs";

const [wasmPath, srcPath, sliceS, traceS, rawOut] = process.argv.slice(2);
const mod = new WebAssembly.Module(readFileSync(wasmPath));
const inst = new WebAssembly.Instance(mod, {});
const x = inst.exports;
// The coded base is the module's custom section `djs_base`, handed over (D2541).
const [coded] = WebAssembly.Module.customSections(mod, "djs_base");
if (coded) {
  const at = x.djs_base_coded_ptr(coded.byteLength); // may grow (and detach) the memory
  new Uint8Array(x.memory.buffer, at, coded.byteLength).set(new Uint8Array(coded));
}
const src = readFileSync(srcPath);
const slice = Number(sliceS);
const trace = Number(traceS ?? 0);
const enc = new TextEncoder();
const dec = new TextDecoder();

const stage = (b) => {
  const p = x.djs_staging(Math.max(b.length, 1));
  new Uint8Array(x.memory.buffer, p, b.length).set(b);
};
const out = () => new Uint8Array(x.memory.buffer, x.djs_out_ptr(), x.djs_out_len()).slice();
const words = () => {
  const o = out();
  return Array.from(new Uint32Array(o.buffer, 0, o.length / 4));
};
const arena = (vm) => new Uint8Array(x.memory.buffer, x.vm_arena_ptr(vm), x.vm_arena_len(vm)).slice();
const fnv = (b) => {
  let h = 0x811c9dc5;
  for (let i = 0; i < b.length; i++) {
    h ^= b[i];
    h = Math.imul(h, 0x01000193);
  }
  return (h >>> 0).toString(16).padStart(8, "0");
};

const vm = x.vm_new(0);
x.vm_set_now(vm, 1_700_000_000_000);
x.vm_set_time_zone_offset(vm, 90);
const g = x.vm_global(vm);
const consoleH = x.vm_new_object(vm);
stage(enc.encode("log"));
const log = x.vm_new_host_function(vm, 1, 3);
stage(enc.encode("log"));
x.vm_define(vm, consoleH, 3, log);
stage(enc.encode("console"));
x.vm_define(vm, g, 7, consoleH);
stage(enc.encode("tool"));
const tool = x.vm_new_async_host_function(vm, 2, 4);
stage(enc.encode("tool"));
x.vm_define(vm, g, 4, tool);
stage(src);
const f = x.vm_compile(vm, 1, 0, src.length);
if (f === 0) throw new Error("compile: " + dec.decode(out()));
stage(new Uint8Array(0));
x.vm_call(vm, f, 0, 0);
let stops = 0;
const pending = [];
let logs = "";
let result;
for (;;) {
  stops++;
  const st = x.vm_run(vm, slice);
  if (trace > 0 && stops % trace === 0) console.log(`stop ${stops} ${fnv(arena(vm))}`);
  const n = x.vm_new_host_calls(vm);
  if (n > 0) pending.push(...words());
  if (st === 0 || st === 1) {
    const r = x.vm_result(vm);
    const ps = x.vm_promise_state(vm, r);
    const h = ps >= 2 ? words()[0] : r;
    x.vm_display(vm, h);
    result = `${ps === 3 || st === 1 ? "threw" : "ok"} ${dec.decode(out())}`;
    break;
  } else if (st === 2) {
  } else if (st === 3) {
    const argc = x.vm_host_call(vm);
    const w = words();
    const parts = [];
    for (let i = 0; i < argc; i++) {
      const bits = (BigInt(w[5 + 2 * i]) << 32n) | BigInt(w[4 + 2 * i]);
      parts.push(bits.toString(16).padStart(16, "0"));
    }
    logs += parts.join(" ") + "\n";
    x.vm_resolve_host_call(vm, w[0], 1, 0);
  } else if (st === 4) {
    if (pending.length === 0) {
      result = "idle";
      break;
    }
    for (const id of pending.splice(0)) x.vm_resolve_host_call(vm, id, 1, 0);
  } else throw new Error("status " + st);
}
const raw = arena(vm);
const rawh = fnv(raw);
if (x.vm_canonical_hash(vm) !== 0) throw new Error("canonical hash");
const [lo, hi] = words();
const canon = hi.toString(16).padStart(8, "0") + lo.toString(16).padStart(8, "0");
x.vm_collect(vm);
const col = fnv(arena(vm));
console.log(`build ${(x.djs_build_hash() >>> 0).toString(16).padStart(8, "0")}`);
console.log(`logs ${fnv(enc.encode(logs))}`);
console.log(`raw ${rawh} len ${raw.length} collected ${col} canonical ${canon} stops ${stops}`);
console.log(`result ${result}`);
if (rawOut) writeFileSync(rawOut, raw);
