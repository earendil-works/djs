// Reads the heap images written by djs-core's `reader_images` test and checks the reader
// against the original values: the wire corpus (expected dumps from gen-cross.ts) and the
// hand-built extra cases.
//
//   node js/test/gen-cross.ts <wire>
//   DJS_WIRE_CROSS=<wire> DJS_READER_DIR=<heap> cargo test -p djs-core reader_images -- --ignored
//   node js/test/reader-cross.ts <wire> <heap>

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { LayoutMismatch, Opaque, readValue } from "../src/reader.ts";
import { dump } from "./dump.ts";

const [wireDir, heapDir] = process.argv.slice(2);
if (!wireDir || !heapDir) throw new Error("usage: reader-cross.ts <wire dir> <heap dir>");

const tsv = (path: string) =>
  readFileSync(path, "utf8").trimEnd().split("\n").map((l) => l.split("\t"));
const heap = (name: string) => new Uint8Array(readFileSync(join(heapDir, name)));
const bits = (hex: string) => BigInt("0x" + hex);

// ---- The wire corpus: what node encoded comes back out of the heap. -------------------
const expected = new Map(tsv(join(wireDir, "cases.tsv")).map(([n, d]) => [n, d]));
const corpus = heap("corpus.heap");
const cache: (string | undefined)[] = [];
let n = 0;
for (const [name, hex] of tsv(join(heapDir, "corpus.tsv"))) {
  const got = dump(readValue(corpus, bits(hex), { atomCache: cache }));
  assert.equal(got, expected.get(name), `case ${name}`);
  n++;
}
assert.ok(n > 1000);

// The arena at an offset within a larger buffer (as in wasm memory), aligned and not.
for (const base of [1024, 3]) {
  const big = new Uint8Array(corpus.length + base + 8);
  big.set(corpus, base);
  const view = big.subarray(base, base + corpus.length);
  for (const [name, hex] of tsv(join(heapDir, "corpus.tsv")).slice(0, 200)) {
    assert.equal(dump(readValue(view, bits(hex))), expected.get(name), `case ${name} at base ${base}`);
  }
}

// ---- Hand-built cases. ------------------------------------------------------------------
const extra = heap("extra.heap");
const cases = new Map(tsv(join(heapDir, "extra.tsv")).map(([name, hex, d]) => [name, { hex, d }]));
const read = (name: string) => readValue(extra, bits(cases.get(name)!.hex));
for (const [name, { d }] of cases) {
  if (d !== "-") assert.equal(dump(read(name)), d, name);
}

const mib = 1 << 20;
const fromUnits = (f: (i: number) => number) => {
  const u = new Uint16Array(mib);
  for (let i = 0; i < mib; i++) u[i] = f(i);
  let s = "";
  for (let i = 0; i < mib; i += 0x2000) s += String.fromCharCode(...u.subarray(i, i + 0x2000));
  return s;
};
assert.equal(read("big-ascii"), "a".repeat(mib));
assert.equal(read("big-latin1"), fromUnits((i) => 0xa0 + (i % 96)));
assert.equal(read("big-latin1-c1"), fromUnits((i) => i % 256));
assert.equal(read("big-two-byte"), fromUnits((i) => 0x400 + (i % 0x400)));
assert.equal(read("big-lone-surrogate"), fromUnits((i) => (i === mib / 2 ? 0xdc00 : 0x400 + (i % 0x400))));

const sym = read("symbol");
assert.ok(sym instanceof Opaque && sym.type === "symbol" && sym.description === "tag");

const props = read("props") as Record<string, unknown>;
assert.deepEqual(Object.keys(props), ["2", "10", "b", "a", "__proto__", "sym"]);
assert.equal(Object.getPrototypeOf(props), Object.prototype);
assert.equal(Object.getOwnPropertyDescriptor(props, "__proto__")!.value, 4);
assert.deepEqual([props["2"], props["10"], props.b, props.a], [3, 10, 1, 2]);
assert.ok(props.sym instanceof Opaque && (props.sym as Opaque).type === "symbol");

const shared = read("shared") as Record<string, unknown>[];
assert.equal(shared[0], shared[1]);
assert.equal(shared[0].x, 1);

const cyc = read("cycle") as Record<string, unknown> & { list: unknown[] };
assert.equal(cyc.self, cyc);
assert.equal(cyc.list[0], cyc);
assert.equal(cyc.list[1], cyc.list);

const sparse = read("sparse") as unknown[];
assert.equal(sparse.length, 1000);
assert.deepEqual(Object.keys(sparse), ["0", "2", "500"]);
assert.deepEqual([sparse[0], sparse[2], sparse[500]], [1, 3, 500]);

// Other value forms.
assert.equal(readValue(extra, [0, 0]), 0);
assert.equal(readValue(extra, [0, 0x40040000]), 2.5);
assert.deepEqual(read("negzero"), -0);
assert.ok(Number.isNaN(read("nan")));

// ---- The layout check. ------------------------------------------------------------------
const bad = extra.slice(0, 4096);
bad[12] ^= 1; // Roots.VERSION
assert.throws(() => readValue(bad, [0, 0]), LayoutMismatch);
const notHeap = new Uint8Array(64);
assert.throws(() => readValue(notHeap, [0, 0]), /bad magic/);

console.log(`reader: ${n} corpus cases and ${cases.size} extra cases match`);
