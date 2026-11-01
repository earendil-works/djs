// Encoder benchmark: `encode` against `JSON.stringify` (plus `TextEncoder.encodeInto`,
// which a wasm host needs to put the JSON text into linear memory). Also writes the
// datasets in both encodings for the decoder benchmark (djs' `bench_wire`):
//
//   node js/bench/wire.bench.ts <dir>
//   DJS_WIRE_BENCH=<dir> cargo test -p djs-core --release bench_wire -- --ignored --nocapture

import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { encode } from "../src/wire.ts";

let seed = 7;
const rnd = (n: number) => {
  seed = (Math.imul(seed, 1103515245) + 12345) >>> 0;
  return seed % n;
};
const pick = <T>(a: T[]): T => a[rnd(a.length)];
const words = ["alpha", "beta", "gamma", "delta", "config", "server", "request", "handler", "error",
  "value", "größe", "naïve", "日本", "user", "data", "index", "parse", "result"];
const sentence = (n: number) => Array.from({ length: n }, () => pick(words)).join(" ");

function similarObjects(n: number) {
  return Array.from({ length: n }, (_, i) => ({
    id: i,
    uuid: `${(i * 2654435761 >>> 0).toString(16)}-${rnd(65536).toString(16)}`,
    name: `${pick(words)} ${pick(words)}`,
    email: `user${i}@example.com`,
    active: rnd(2) === 0,
    score: rnd(100000) / 100,
    tags: [pick(words), pick(words)],
    created: `2024-${String(1 + rnd(12)).padStart(2, "0")}-${String(1 + rnd(28)).padStart(2, "0")}T10:00:00Z`,
    parent: rnd(4) === 0 ? null : rnd(n),
  }));
}

function sourceText(bytes: number) {
  const lines: string[] = [];
  let size = 0;
  while (size < bytes) {
    const l = `${" ".repeat(rnd(4) * 4)}let ${pick(words)} = ${pick(words)}(${rnd(100)}); // ${sentence(4)}`;
    lines.push(l);
    size += l.length + 1;
  }
  return lines.join("\n");
}

function toolResults() {
  return {
    query: "handler error",
    results: Array.from({ length: 2000 }, (_, i) => ({
      path: `src/${pick(words)}/${pick(words)}_${i}.ts`,
      line: rnd(5000),
      text: sentence(8 + rnd(12)),
      score: rnd(1000) / 1000,
      matches: Array.from({ length: 1 + rnd(3) }, () => ({ start: rnd(80), end: 80 + rnd(40) })),
    })),
    stats: { files: 12873, matched: 2000, elapsedMs: 412.5, truncated: true },
    file: { path: "src/server/handler.ts", content: sourceText(200_000) },
  };
}

// Tool results reach the host as parsed JSON, so the data goes through JSON.parse: that
// gives flat strings like real data (template literals would make V8 cons strings, which
// are much slower to read character by character).
const viaJson = (v: unknown) => JSON.parse(JSON.stringify(v));
const datasets: [string, unknown][] = [
  ["10k-objects", viaJson(similarObjects(10_000))],
  ["1mib-string", "x".repeat(1 << 20)],
  ["1mib-text-unicode", sourceText(1 << 20)],
  ["tool-results", viaJson(toolResults())],
  ["500k-numbers", Array.from({ length: 500_000 }, (_, i) => (i % 2 ? rnd(1_000_000) : rnd(1_000_000_000) / 1000))],
];

function time(f: () => unknown): number {
  for (let i = 0; i < 3; i++) f();
  const runs: number[] = [];
  const deadline = performance.now() + 1000;
  while (runs.length < 5 || (performance.now() < deadline && runs.length < 200)) {
    const t = performance.now();
    f();
    runs.push(performance.now() - t);
  }
  runs.sort((a, b) => a - b);
  return runs[runs.length >> 1];
}

const dir = process.argv[2];
if (dir) mkdirSync(dir, { recursive: true });
const textEncoder = new TextEncoder();
const scratch = new Uint8Array(64 << 20);
console.log("dataset              encode ms   JSON.stringify ms   +encodeInto ms   wire KB   JSON KB");
for (const [name, value] of datasets) {
  const wire = encode(value);
  const json = JSON.stringify(value);
  const jsonBytes = textEncoder.encode(json);
  if (dir) {
    writeFileSync(join(dir, name + ".wire"), wire);
    writeFileSync(join(dir, name + ".json"), jsonBytes);
  }
  const tEnc = time(() => encode(value, scratch));
  const tStr = time(() => JSON.stringify(value));
  const tStrInto = time(() => textEncoder.encodeInto(JSON.stringify(value), scratch));
  console.log(
    `${name.padEnd(20)} ${tEnc.toFixed(2).padStart(9)} ${tStr.toFixed(2).padStart(19)} ${tStrInto.toFixed(2).padStart(16)} ` +
      `${(wire.length / 1024).toFixed(0).padStart(9)} ${(jsonBytes.length / 1024).toFixed(0).padStart(9)}`,
  );
}
