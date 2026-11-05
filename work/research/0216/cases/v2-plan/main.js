"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 4);
globalThis.$dump = {};
$dump.s2 = {};
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
$dump.p1 = $djs.resume(3, "later", ["fine"]);
const contents = { "src/a.ts": "TODO one\nTODO two\nok", "src/b.ts": "fine", "src/c.ts": "TODO three", "src/d.ts": "TODO four\nok\nTODO five" };
const tools = { glob: (args) => later(Object.keys(contents)), read: ({ path }) => later(contents[path]) };
$dump.n3 = new Map([["src/a.ts", 2]]);
$dump.n4 = ["src/a.ts", "src/b.ts"];
$dump.n7 = [];
$dump.i8 = $djs.advance(Array.prototype.values.call([$dump.n4, ["src/c.ts", "src/d.ts"]]), 1);
$dump.ga9 = () => {
$djs.k($dump.s2.$Ka5, { f: $dump.s2.todos, segs: [30,39,13,53,78,69,122,151,19,141,174,9], scaf: [["site",46,37]] });
$dump.a5 = $dump.s2.$Ka5.call(undefined, "src/b.ts");
$dump.p6 = Promise.all([2, $dump.a5]);
};
$dump.a9 = $djs.k(async function (files, report) {
const $djs_I = $djs.I;
const seen = $dump.n3;
Object.assign($dump.s2, { todos: async function todos(path) {
    const body = await tools.read({ path });
    const n = body.split("\n").filter((l) => l.includes("TODO")).length;
    seen.set(path, n);
    return n;
  }, $Ka5: async function (path) {
const body = await $dump.p1;
const n = body.split("\n").filter((l) => l.includes("TODO")).length;
seen.set(path, n);
return n;
} });
var todos = $dump.s2.todos;
$dump.ga9();
const $djs_i1 = $dump.i8;
report.push(await $dump.p6);
for (const batch of $djs_I($djs_i1)) {
    report.push(await Promise.all(batch.map(todos)));
  }
print(`${report.flat().reduce((a, b) => a + b, 0)} TODOs in ${seen.size} files`);
return report;
}, { f: main, at: "await#2", segs: [531,385,18,557,432,3,560,322,16,597,379,60,657,441,82,739,525,16], scaf: [["for-of",322,560,0,"$djs_i1"],["site",397,543]] })(["src/a.ts", "src/b.ts", "src/c.ts", "src/d.ts"], $dump.n7);
$dump.p10 = Promise.prototype.then.call($dump.a9, (r) => print(JSON.stringify(r)), undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
