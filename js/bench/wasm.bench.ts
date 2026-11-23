// The boundary in wasm, both directions, on the djs-wasm `wasm` profile build:
//
// - inbound: `encode` into linear memory + `vm_decode_wire`, against `JSON.stringify` +
//   its text as a wire string + `JSON.parse` in the VM (each decode into a fresh VM);
// - outbound: `readValue` on the arena, against `JSON.stringify` in the VM + reading the
//   JSON string out of the heap + `JSON.parse`.
//
//   node js/bench/wire.bench.ts <dir>      (writes the datasets)
//   scripts/build-wasm.sh
//   node js/bench/wasm.bench.ts <dir>
//
// With `--tsv`, prints `name<TAB>value<TAB>unit` lines instead (djs-bench's wasm group).
// The datasets are `<name>.json` files; djs-bench writes its own.

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { encode } from "../src/wire.ts";
import { readValue } from "../src/reader.ts";
import { Sandbox, defaultWasmPath } from "../src/sandbox.ts";
import { provideBase, rememberBase } from "../src/base.ts";
import { djsExports } from "../src/abi.ts";

interface X {
  memory: WebAssembly.Memory;
  djs_staging(len: number): number;
  djs_out_ptr(): number;
  vm_new(limit: number, live?: number, flags?: number): number;
  vm_new_compiled?: (limit: number) => number;
  vm_free(vm: number): void;
  vm_arena_ptr(vm: number): number;
  vm_arena_len(vm: number): number;
  vm_decode_wire(vm: number, len: number): number;
  vm_compile_file(vm: number, goal: number, nameLen: number, paramsLen: number, srcLen: number): number;
  vm_call(vm: number, f: number, thisArg: number, argc: number): number;
  vm_run(vm: number, fuel: number): number;
  vm_result(vm: number): number;
  vm_handle_bits(vm: number, h: number): void;
  vm_handle_free(vm: number, h: number): void;
}

/** Median ms of `f`, with `setup` (untimed) before each run. */
function time<S>(setup: () => S, f: (s: S) => void, teardown: (s: S) => void = () => {}): number {
  const runs: number[] = [];
  const deadline = performance.now() + 1500;
  for (let i = 0; i < 3 || runs.length < 5 || (performance.now() < deadline && runs.length < 200); i++) {
    const s = setup();
    const t = performance.now();
    f(s);
    const dt = performance.now() - t;
    teardown(s);
    if (i >= 3) runs.push(dt);
  }
  runs.sort((a, b) => a - b);
  return runs[runs.length >> 1];
}

const dir = process.argv[2];
const tsv = process.argv.includes("--tsv");
if (!dir) throw new Error("usage: wasm.bench.ts <dir from wire.bench.ts>");
const path = defaultWasmPath();
const bytes = readFileSync(typeof path === "string" ? path : fileURLToPath(path));
const { module, instance } = await WebAssembly.instantiate(bytes, {});
const x = djsExports(module, instance) as unknown as X;
// An instance's base goes to its first VM as the arena (D2060, D3360): each VM here gets
// the decoded base injected first, as a host's fresh instance does.
rememberBase(module, x as never);
const vmNew = x.vm_new;
x.vm_new = (limit: number) => {
  provideBase(module, x as never);
  return vmNew(limit, 0, 0);
};
const textEncoder = new TextEncoder();
const CAP = 16 << 20;
const staging = x.djs_staging(CAP);
// The buffer detaches when memory grows: re-derive views after every call into the module.
const stagingView = () => new Uint8Array(x.memory.buffer, staging, CAP);
const arena = (vm: number) => new Uint8Array(x.memory.buffer, x.vm_arena_ptr(vm), x.vm_arena_len(vm));
const bits = (vm: number, h: number): [number, number] => {
  x.vm_handle_bits(vm, h);
  const w = new Uint32Array(x.memory.buffer, x.djs_out_ptr(), 2);
  return [w[0], w[1]];
};

function encodeIn(value: unknown): number {
  const view = stagingView();
  const out = encode(value, view);
  if (out.buffer !== view.buffer || out.byteOffset !== view.byteOffset) stagingView().set(out);
  return out.length;
}
function jsonIn(value: unknown): number {
  return encodeIn(JSON.stringify(value));
}

/** Calls `f` with the argument handles `args` and returns the result's handle. */
function callIn(vm: number, f: number, args: number[]): number {
  new Uint32Array(x.memory.buffer, staging, args.length).set(args);
  if (x.vm_call(vm, f, 0, args.length) !== 0 || x.vm_run(vm, 1e9) !== 0) throw new Error("call failed");
  return x.vm_result(vm);
}

/** A function of the VM: the completion value of the Script `src`. */
function fnIn(vm: number, src: string): number {
  const n = textEncoder.encodeInto(src, stagingView()).written;
  const script = x.vm_compile_file(vm, 0, 0, 0, n);
  if (script === 0) throw new Error("compile failed");
  const f = callIn(vm, script, []);
  x.vm_handle_free(vm, script);
  return f;
}

const names = ["10k-objects", "1mib-string", "1mib-text-unicode", "tool-results", "500k-numbers"];
const inbound: string[] = [];
const outbound: string[] = [];
const rows: string[] = [];
const row = (name: string, ms: number) => rows.push(`wasm/${name}\t${ms.toFixed(4)}\tms`);

// Startup: a new VM (realm built from scratch) and a new instance, in ns.
{
  const ns = (f: () => void) => {
    const runs: number[] = [];
    for (let i = 0; i < 210; i++) {
      const t = performance.now();
      f();
      if (i >= 10) runs.push(performance.now() - t);
    }
    runs.sort((a, b) => a - b);
    return runs[runs.length >> 1] * 1e6;
  };
  const module = new WebAssembly.Module(bytes);
  rows.push(`wasm/instantiate\t${ns(() => new WebAssembly.Instance(module, {})).toFixed(0)}\tns/op`);
  // The module above carries what building the realm from scratch needs (the
  // compiled-prelude feature, for `vm_new compiling the prelude`), several times the
  // shipped module's data segment, and instantiation copies the data: the shipped
  // module's number is the one hosts pay (0224).
  const shipped = process.env.DJS_WASM_SHIPPED;
  if (shipped) {
    const m = new WebAssembly.Module(readFileSync(shipped));
    rows.push(`wasm/instantiate shipped\t${ns(() => new WebAssembly.Instance(m, {})).toFixed(0)}\tns/op`);
  }
  // Batches of 100: one copy of the base is close to the timer's resolution.
  const vmNewNs = ns(() => {
    for (let i = 0; i < 100; i++) x.vm_free(x.vm_new(0, 0, 0));
  }) / 100;
  rows.push(`wasm/vm_new\t${vmNewNs.toFixed(0)}\tns/op`);
  if (x.vm_new_compiled !== undefined) {
    const compiled = x.vm_new_compiled;
    rows.push(`wasm/vm_new compiling the prelude\t${ns(() => x.vm_free(compiled(0))).toFixed(0)}\tns/op`);
  }
  // The interpreter: a loop with arithmetic, property reads and string concatenation
  // (only what the compiler on main supports).
  const sb = new Sandbox({ module, fuelSlice: 1e9 });
  const code =
    "let s = 0; let o = {a: 1, b: 2}; let arr = []; for (let i = 0; i < 300000; i++) " +
    "{ s = (s + i * o.a) % 1000003; if (i % 100 === 0) arr[arr.length] = 'x' + i; } return s + arr.length";
  const runs: number[] = [];
  for (let i = 0; i < 13; i++) {
    const t = performance.now();
    sb.run(code);
    if (i >= 2) runs.push(performance.now() - t);
  }
  runs.sort((a, b) => a - b);
  row("interp loop", runs[runs.length >> 1]);
  sb.dispose();
}

for (const name of names) {
  const value = JSON.parse(readFileSync(join(dir, name + ".json"), "utf8"));
  const wireLen = encodeIn(value);
  const jsonLen = jsonIn(value);

  // Inbound.
  const fresh = () => x.vm_new(0, 0, 0);
  const free = (vm: number) => x.vm_free(vm);
  const tEncode = time(() => 0, () => void encodeIn(value));
  const tStringify = time(() => 0, () => void jsonIn(value));
  encodeIn(value);
  const tDecode = time(fresh, (vm) => {
    if (x.vm_decode_wire(vm, wireLen) === 0) throw new Error("decode failed");
  }, free);
  // JSON.parse in the VM, of the JSON text (a wire string).
  const tParse = time(() => {
    const vm = fresh();
    const parse = fnIn(vm, "(s) => JSON.parse(s)");
    jsonIn(value);
    return [vm, parse];
  }, ([vm, parse]) => {
    const s = x.vm_decode_wire(vm, jsonLen);
    if (s === 0 || callIn(vm, parse, [s]) === 0) throw new Error("parse failed");
  }, ([vm]) => free(vm));
  row(`decode ${name}`, tDecode);
  row(`parse_json ${name}`, tParse);
  inbound.push(
    `| ${name} | ${tEncode.toFixed(2)} | ${tStringify.toFixed(2)} | ${tDecode.toFixed(2)} | ${tParse.toFixed(2)} | ` +
      `**${(tEncode + tDecode).toFixed(2)}** | ${(tStringify + tParse).toFixed(2)} |`,
  );

  // Outbound: the value lives in a VM.
  const vm = x.vm_new(0, 0, 0);
  const stringify = fnIn(vm, "(v) => JSON.stringify(v)");
  encodeIn(value);
  const h = x.vm_decode_wire(vm, wireLen);
  const vb = bits(vm, h);
  const tRead = time(() => 0, () => void readValue(arena(vm), vb));
  const tStr = time(() => 0, () => {
    const s = callIn(vm, stringify, [h]);
    if (s === 0) throw new Error("stringify failed");
    x.vm_handle_free(vm, s);
  });
  const sh = callIn(vm, stringify, [h]);
  const sb = bits(vm, sh);
  const tJson = time(() => 0, () => void JSON.parse(readValue(arena(vm), sb) as string));
  row(`readValue ${name}`, tRead);
  row(`json_stringify ${name}`, tStr);
  outbound.push(
    `| ${name} | **${tRead.toFixed(2)}** | ${tStr.toFixed(2)} | ${tJson.toFixed(2)} | ${(tStr + tJson).toFixed(2)} |`,
  );
  x.vm_free(vm);
}
// A 1 MiB image (0039): bytes as a Uint8Array on the wire (decoded to a Uint8Array in the
// VM, read back as a copy) against the same image as a base64 string, both ways.
const image = new Uint8Array(1 << 20);
for (let i = 0, v = 1; i < image.length; i++) {
  v = (v * 1103515245 + 12345) >>> 0;
  image[i] = v >>> 24;
}
const imageB64 = image.toBase64();
const images: [string, unknown][] = [["image 1mib bytes", image], ["image 1mib base64", imageB64]];
const imageRows: string[] = [];
for (const [name, value] of images) {
  const tEnc = time(() => 0, () => void encodeIn(value));
  const len = encodeIn(value);
  const tDecode = time(() => x.vm_new(0, 0, 0), (vm) => {
    if (x.vm_decode_wire(vm, len) === 0) throw new Error("decode failed");
  }, (vm) => x.vm_free(vm));
  const vm = x.vm_new(0, 0, 0);
  encodeIn(value);
  const vb = bits(vm, x.vm_decode_wire(vm, len));
  const tRead = time(() => 0, () => void readValue(arena(vm), vb));
  x.vm_free(vm);
  row(`encode ${name}`, tEnc);
  row(`decode ${name}`, tDecode);
  row(`readValue ${name}`, tRead);
  imageRows.push(`| ${name} | ${tEnc.toFixed(3)} | ${tDecode.toFixed(3)} | ${tRead.toFixed(3)} |`);
}
if (tsv) {
  console.log(rows.join("\n"));
  process.exit(0);
}
console.log("| dataset | encode | stringify + encode | decode | JSON.parse in the VM | wire total | JSON total |");
console.log("| --- | ---: | ---: | ---: | ---: | ---: | ---: |");
console.log(inbound.join("\n"));
console.log();
console.log("| dataset | readValue | JSON.stringify in the VM | read JSON + JSON.parse | JSON total |");
console.log("| --- | ---: | ---: | ---: | ---: |");
console.log(outbound.join("\n"));
console.log();
console.log("| 1 MiB image | encode | vm_decode_wire | readValue |");
console.log("| --- | ---: | ---: | ---: |");
console.log(imageRows.join("\n"));
