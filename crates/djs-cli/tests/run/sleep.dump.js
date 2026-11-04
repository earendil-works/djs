"djs-dump";
$djs.seed(5, 7);
globalThis.$dump = {};
try {
async function main() {
  await sleep(100);
  print("woke");
}
$dump.p2 = $djs.resume(2, "sleep", [100]);
$dump.Ka = async function () {
(await $dump.p2);
print("woke");
};
$dump.a = $dump.Ka.call(undefined);
$dump.a;
} finally { delete globalThis.$dump; delete globalThis.$djs; }
