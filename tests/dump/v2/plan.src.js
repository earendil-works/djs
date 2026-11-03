// The example of docs/continuation-dumps.md, with `later` as the tools' transport.
// plan.dump.js, written by hand, is this program at idle point 5: no jobs, no frames;
// host calls issued 5, delivered 4. todos shares `seen` with main's frame; the for-of's
// iterator is past the first batch; Promise.all over the second batch has element 0
// resolved to 1.
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
main().then((r) => print(JSON.stringify(r)));
