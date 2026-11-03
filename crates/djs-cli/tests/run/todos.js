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
console.log(`${report.flat().reduce((a, b) => a + b, 0)} TODOs in ${seen.size} files`);
