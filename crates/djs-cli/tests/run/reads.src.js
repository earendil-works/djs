// reads.dump.js, written by hand for djs-cli's tests, is this program at idle point 1.
async function main() {
  const a = await tools.read({ path: "a" });
  const b = await tools.read({ path: "b" });
  print(a + b);
  return a.length + b.length;
}
main().then((n) => print("total", n));
