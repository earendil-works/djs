"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(3, "later", ["aa"]);
$dump.i2 = $dump.advance(Array.prototype.values.call(["a", "b"]), 1);
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

}, { at: "await#2", awaiting: $dump.p1, this: globalThis, args: [tools, console], slots: { tools: tools, console: console, n: 0, f: "a", body: undefined, total: undefined, "for-of.iterator@140": $dump.i2, "for-of.next@140": $dump.builtin("%ArrayIteratorPrototype%.next") } });
$dump.handle("run", $dump.a3);
