"djs-dump";
"use strict";
$djs.seed(2135587861,-1640531527);
async function main() {
  const files = await tools.glob({ pattern: "src/*.ts" });
  const seen = new Map();
  async function todos(path) {
    const body = await tools.read({ path });
    const n = body.split("\n").filter((l) => l.includes("TODO")).length;
    seen.set(path, n);
    return n;
  }
  const report = [];
  for (const batch of [files.slice(0, 2), files.slice(2)]) {
    report.push(await Promise.all(batch.map(todos)));
  }
  print(`${report.flat().reduce((a, b) => a + b, 0)} TODOs in ${seen.size} files`);
  return report;
}
const contents = { "src/a.ts": "TODO one\nTODO two\nok", "src/b.ts": "fine", "src/c.ts": "TODO three", "src/d.ts": "TODO four\nok\nTODO five" };
const tools = { glob: (args) => later(Object.keys(contents)), read: ({ path }) => later(contents[path]) };
$djs.k(async function (files, report, $i) {
const seen = new Map([["src/a.ts", 2]]);
async function todos(path) {
    const body = await tools.read({ path });
    const n = body.split("\n").filter((l) => l.includes("TODO")).length;
    seen.set(path, n);
    return n;
  }
report.push(await Promise.all([2, $djs.k(async function (path) {
const body = await $djs.resume(3,"later",["fine"]);
const n = body.split("\n").filter((l) => l.includes("TODO")).length;
seen.set(path, n);
return n;
},todos)("src/b.ts")]));
for (const batch of $i) {
    report.push(await Promise.all(batch.map(todos)));
  }
print(`${report.flat().reduce((a, b) => a + b, 0)} TODOs in ${seen.size} files`);
return report;
},main,2)(["src/a.ts","src/b.ts","src/c.ts","src/d.ts"],[],[["src/c.ts","src/d.ts"]].values()).then((r) => print(JSON.stringify(r)))
