// reads.dump.js (format 2) and reads.plain.js (a plain dump), written by hand for
// djs-cli's tests, are this program at idle point 1.
async function main() {
  const a = await tools.read({ path: "a" });
  const b = await tools.read({ path: "b" });
  print(a + b);
  return a.length + b.length;
}
main().then((n) => print("total", n));
