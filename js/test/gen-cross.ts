// Writes the cross-language corpus for djs' `wire_cross` test into the directory given as
// the first argument: NNNN.wire (encode output), NNNN.json (JSON.stringify output, when the
// value survives JSON unchanged) and cases.tsv (`NNNN \t dump \t json?`). The dump format
// is the one of crates/djs-core/src/wire/tests.rs (`dump`).
//
//   node js/test/gen-cross.ts <dir>
//   DJS_WIRE_CROSS=<dir> cargo test -p djs-core wire_cross -- --ignored

import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { isDeepStrictEqual } from "node:util";
import { encode } from "../src/wire.ts";

function quote(s: string): string {
  let out = '"';
  for (let i = 0; i < s.length; i++) {
    const u = s.charCodeAt(i);
    if (u >= 0x20 && u < 0x7f && u !== 0x22 && u !== 0x5c) out += s[i];
    else out += "\\u" + u.toString(16).padStart(4, "0");
  }
  return out + '"';
}

function dump(v: unknown): string {
  if (v === undefined) return "undefined";
  if (v === null) return "null";
  if (typeof v === "boolean") return String(v);
  if (typeof v === "number") return Object.is(v, -0) ? "n:-0" : "n:" + String(v);
  if (typeof v === "string") return "s:" + quote(v);
  if (v instanceof Uint8Array) return "[" + Array.from(v, (b) => "n:" + b).join(",") + "]";
  if (Array.isArray(v)) {
    const parts: string[] = [];
    for (let i = 0; i < v.length; i++) parts.push(i in v ? dump(v[i]) : "<hole>");
    return "[" + parts.join(",") + "]";
  }
  const o = v as Record<string, unknown>;
  return "{" + Object.keys(o).map((k) => quote(k) + ":" + dump(o[k])).join(",") + "}";
}

/** True if JSON.parse(JSON.stringify(v)) gives back the same value. */
function jsonSafe(v: unknown): boolean {
  if (v === undefined) return false;
  const text = JSON.stringify(v);
  if (text === undefined) return false;
  return isDeepStrictEqual(JSON.parse(text), v) && dump(JSON.parse(text)) === dump(v);
}

let seed = 2024;
const rnd = (n: number) => {
  seed = (Math.imul(seed, 1103515245) + 12345) >>> 0;
  return seed % n;
};
const words = ["", "a", "id", "name", "é", "Ünïcödé", "日本語", "😀", "\uD83D", "\uDE00x", "\u0000",
  "\u2028", "tab\t", "quote\"", "back\\slash", "x".repeat(50), "y".repeat(5000), "0", "42"];
function gen(d: number): unknown {
  switch (rnd(d > 3 ? 7 : 10)) {
    case 0: return [null, undefined, true, false][rnd(4)];
    case 1: return [0, -0, 1.5, -2.25, 1e300, 5e-324, NaN, Infinity, 2 ** 53][rnd(9)];
    case 2: return rnd(200000) - 100000;
    case 3: return rnd(256) - 128;
    case 4: case 5: return words[rnd(words.length)] + words[rnd(words.length)];
    case 6: return new Uint8Array(Array.from({ length: rnd(4) }, () => rnd(256)));
    case 7: {
      const a: unknown[] = [];
      const n = rnd(6);
      for (let i = 0; i < n; i++) if (rnd(8) !== 0) a[i] = gen(d + 1); else a.length = i + 1;
      return a;
    }
    default: {
      const o: Record<string, unknown> = {};
      for (let i = rnd(6); i > 0; i--) o[words[rnd(words.length)] + rnd(3)] = gen(d + 1);
      return o;
    }
  }
}

const values: unknown[] = [
  undefined, null, true, false, 0, -0, 1, -1, 127, -128, 128, 2 ** 31 - 1, -(2 ** 31), 2 ** 31,
  0.1, NaN, -Infinity, "", "plain", "é", "😀", "\uD800", "a".repeat(100000),
  [], {}, [1, [2, [3]]], { a: { b: { c: {} } } }, [, 1, , 2],
  { "0": 0, "10": 10, "2": 2, b: "b", a: "a" },
  Array.from({ length: 300 }, (_, i) => ({ id: i, name: "item " + i, price: i * 0.25, tags: ["t" + (i % 7)] })),
  { files: [{ path: "src/main.rs", size: 1234, lines: ["fn main() {", "}"] }], truncated: false },
  new Uint8Array([0, 128, 255]),
  Object.assign(Object.create(null), { nullProto: 1 }),
];
for (let i = 0; i < 1500; i++) values.push(gen(0));

const dir = process.argv[2];
if (!dir) throw new Error("usage: gen-cross.ts <dir>");
mkdirSync(dir, { recursive: true });
const lines: string[] = [];
values.forEach((v, i) => {
  const name = String(i).padStart(4, "0");
  writeFileSync(join(dir, name + ".wire"), encode(v));
  const safe = jsonSafe(v);
  if (safe) writeFileSync(join(dir, name + ".json"), JSON.stringify(v));
  lines.push([name, dump(v), safe ? "1" : "0"].join("\t"));
});
writeFileSync(join(dir, "cases.tsv"), lines.join("\n") + "\n");
console.log(`wrote ${values.length} cases to ${dir}`);
