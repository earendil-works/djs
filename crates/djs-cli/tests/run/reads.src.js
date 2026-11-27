// A program for djs-cli's tests that reads two files through tools, one at a time:
// two idle points.
async function main() {
  const a = await tools.read({ path: "a" });
  const b = await tools.read({ path: "b" });
  print(a + b);
  return a.length + b.length;
}
main().then((n) => print("total", n));
