"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(6, "later", ["ok"]);
$dump.p2 = $dump.pending(7, "later", ["fast"]);
async function main() {
  const all = Promise.all([1, later(2), later(3)]);
  const settled = Promise.allSettled([later(new Error("e"), true), later(4)]);
  const any = Promise.any([later(new Error("x"), true), later("ok")]);
  const race = Promise.race([later("fast"), new Promise(() => {})]);
  print(JSON.stringify(await all), JSON.stringify(await settled), await any, await race);
}
$dump.n3 = new Error("x");
$dump.p4 = Promise.any([$dump.rejected($dump.n3), $dump.p1]);
$dump.p5 = new Promise((res, rej) => { $dump.r6 = [res, rej]; });
$dump.p7 = Promise.prototype.then.call($dump.p2, $dump.r6[0], $dump.r6[1]);
$dump.n8 = Promise.resolve([1, 2, 3]);
$dump.n10 = [{ status: "rejected", reason: new Error("e") }, { status: "fulfilled", value: 4 }];
$dump.n9 = Promise.resolve($dump.n10);
$dump.a11 = $dump.resume(main, { at: "await#3", awaiting: $dump.p4, bindings: { all: $dump.n8, settled: $dump.n9, any: $dump.p4, race: $dump.p5 }, operands: [["call.callee", print], ["call.this", undefined], ["call.arg[0]", "[1,2,3]"], ["call.arg[1]", "[{\"status\":\"rejected\",\"reason\":{}},{\"status\":\"fulfilled\",\"value\":4}]"]] });
Object.defineProperty($dump.n3, "stack", { value: "    at main (src.js:5:43)\n    at <eval> (src.js:9:5)\n", writable: true, configurable: true });
Object.defineProperty($dump.n10[0].reason, "stack", { value: "    at main (src.js:4:54)\n    at <eval> (src.js:9:5)\n", writable: true, configurable: true });
