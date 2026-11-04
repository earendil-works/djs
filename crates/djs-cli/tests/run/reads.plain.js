"djs-dump 3";
$djs.seed(5, 7);
globalThis.$dump = {};
try {
async function main() {
  const a = await tools.read({ path: "a" });
  const b = await tools.read({ path: "b" });
  print(a + b);
  return a.length + b.length;
}
$dump.p2 = $djs.resume(2, "tools.read", [{ path: "a" }], { "at": "a" });
$dump.Ka = async function () {
const a = (await $dump.p2);
const b = await tools.read({ path: "b" });
print(a + b);
return a.length + b.length;
};
$dump.a = $dump.Ka.call(undefined);
$dump.a.then((n) => print("total", n));
$dump.a;
} finally { delete globalThis.$dump; delete globalThis.$djs; }
