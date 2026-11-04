"djs-dump 2";
const contents = {
  "src/a.ts": "TODO one\nTODO two\nok",
  "src/b.ts": "fine",
  "src/c.ts": "TODO three",
  "src/d.ts": "TODO four\nok\nTODO five",
};
const tools = {
  glob: (args) => later(Object.keys(contents)),
  read: ({ path }) => later(contents[path]),
};
$dump.files = ["src/a.ts", "src/b.ts", "src/c.ts", "src/d.ts"];
$dump.report = [[2, 0]];
$dump.batches = [$dump.files.slice(0, 2), $dump.files.slice(2)];

$dump.s1 = (() => {
  let seen = new Map([["src/a.ts", 2], ["src/b.ts", 0], ["src/c.ts", 1]]);
  async function todos(path) {
    const body = await tools.read({ path });
    const n = body.split("\n").filter((l) => l.includes("TODO")).length;
    seen.set(path, n);
    return n;
  }
  return { todos, $cells: { seen: $dump.cell(() => seen) } };
})();

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

$dump.it1 = $dump.batches[Symbol.iterator]();
$dump.it1.next();
$dump.it1.next();

$dump.p5 = $dump.pending(5, "later", [contents["src/d.ts"]]);

$dump.a2 = $dump.resume($dump.s1.todos, {
  at: "await#1",
  awaiting: $dump.p5,
  this: undefined,
  args: ["src/d.ts", 1, $dump.batches[1]],
  slots: { path: "src/d.ts", body: $dump.TDZ, n: $dump.TDZ },
});

$dump.p6 = Promise.all([1, $dump.a2]);

$dump.run = $dump.resume(main, {
  at: "await#2",
  awaiting: $dump.p6,
  this: undefined,
  args: [],
  slots: {
    files: $dump.files,
    seen: $dump.s1.$cells.seen,
    todos: $dump.s1.todos,
    report: $dump.report,
    batch: $dump.batches[1],
    "for-of.iterator@322": $dump.it1,
    "for-of.next@322": Object.getPrototypeOf($dump.it1).next,
    "call.callee@385": Array.prototype.push,
    "call.this@385": $dump.report,
  },
});
$dump.run.then((r) => print(JSON.stringify(r)));

$dump.handle("run", $dump.run);
