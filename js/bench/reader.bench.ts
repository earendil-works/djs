// Reader benchmark: `readValue` on heap images against the JSON path (VM-side
// `json::stringify_fast`, timed natively by the image writer, then reading the JSON
// string out of the heap and `JSON.parse`).
//
//   node js/bench/wire.bench.ts <wire>
//   DJS_WIRE_BENCH=<wire> DJS_READER_BENCH=<dir> cargo test -p djs-core --release reader_bench_images -- --ignored
//   node js/bench/reader.bench.ts <dir>

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { readValue } from "../src/reader.ts";

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
if (!dir) throw new Error("usage: reader.bench.ts <dir>");
console.log("dataset              read ms   stringify_fast ms   +read JSON +JSON.parse ms   JSON total ms");
for (const line of readFileSync(join(dir, "index.tsv"), "utf8").trimEnd().split("\n")) {
  const [name, valueHex, jsonHex, stringifyMs] = line.split("\t");
  const mem = new Uint8Array(readFileSync(join(dir, name + ".heap")));
  const value = BigInt("0x" + valueHex);
  const json = BigInt("0x" + jsonHex);
  const tRead = time(() => readValue(mem, value));
  const tJson = time(() => JSON.parse(readValue(mem, json) as string));
  const tStr = Number(stringifyMs);
  console.log(
    `${name.padEnd(20)} ${tRead.toFixed(2).padStart(7)} ${tStr.toFixed(2).padStart(19)} ${tJson.toFixed(2).padStart(28)} ${(tStr + tJson).toFixed(2).padStart(15)}`,
  );
}
// memcpy baseline for the 1 MiB strings.
const block = new Uint8Array(1 << 20);
console.log(`memcpy 1 MiB: ${time(() => block.slice()).toFixed(3)} ms; 2 MiB: ${time(() => new Uint8Array(2 << 20).slice()).toFixed(3)} ms`);
