"djs-dump 2";
$dump.p1 = $dump.pending(7, "later", [0]);
$dump.s2 = (() => {
  let n = 4;
  return { total: () => n, $c4: $dump.cell(() => n) };
})();
$dump.a3 = $dump.resume(async function (tools, console) {
// A codemode body (pi's Sandbox): tools and console are its parameters.
console.log("start");
let n = 0;
for (const f of await tools.list()) {
  const body = await tools.read(f);
  n += body.length;
  console.log(f, body);
}
const total = () => n;
await later(0);
console.log("total", total());
return n;

}, { at: "await#3", awaiting: $dump.p1, this: globalThis, args: [tools, console], bindings: { tools: tools, console: console, n: $dump.s2.$c4, total: $dump.s2.total } });
$dump.handle("run", $dump.a3);
