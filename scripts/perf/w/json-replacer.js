// JSON.stringify with a replacer function and with an array of keys (0090).
// bench: expect 1289230
const rows = [];
for (let i = 0; i < 2000; i++) {
  rows.push({ id: i, name: "item " + i, path: "/src/file_" + i + ".ts", size: i * 37, tags: ["a", "b"], secret: "x" + i });
}
let total = 0;
for (let r = 0; r < 5; r++) {
  total += JSON.stringify(rows, (k, v) => (k === "secret" ? undefined : v)).length;
  total += JSON.stringify(rows, ["id", "name", "size"]).length;
}
return total;
