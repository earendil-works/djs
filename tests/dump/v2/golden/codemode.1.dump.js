"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(2, "later", [["a", "b"]]);
$dump.a2 = $dump.resume(async function (tools, console) {
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

}, { at: "await#1", awaiting: $dump.p1, this: globalThis, args: [tools, console], bindings: { tools: tools, console: console, n: 0, f: undefined, total: undefined } });
$dump.handle("run", $dump.a2);
