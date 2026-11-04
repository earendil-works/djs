"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.n1 = new Error("x");
$dump.p2 = $dump.pending(5, "later", [$dump.n1, true]);
$dump.p3 = $dump.pending(6, "later", ["ok"]);
$dump.p4 = $dump.pending(7, "later", ["fast"]);
async function main() {
  const all = Promise.all([1, later(2), later(3)]);
  const settled = Promise.allSettled([later(new Error("e"), true), later(4)]);
  const any = Promise.any([later(new Error("x"), true), later("ok")]);
  const race = Promise.race([later("fast"), new Promise(() => {})]);
  print(JSON.stringify(await all), JSON.stringify(await settled), await any, await race);
}
$dump.p5 = Promise.any([$dump.p2, $dump.p3]);
$dump.p6 = new Promise((res, rej) => { $dump.r7 = [res, rej]; });
$dump.p8 = Promise.prototype.then.call($dump.p4, $dump.r7[0], $dump.r7[1]);
$dump.n9 = Promise.resolve([1, 2, 3]);
$dump.n11 = [{ status: "rejected", reason: new Error("e") }, { status: "fulfilled", value: 4 }];
$dump.n10 = Promise.resolve($dump.n11);
$dump.a12 = $dump.resume(main, { at: "await#3", awaiting: $dump.p5, slots: { all: $dump.n9, settled: $dump.n10, any: $dump.p5, race: $dump.p6, "call.callee@297": print, "call.this@297": undefined, "call.arg@303": "[1,2,3]", "call.arg@330": "[{\"status\":\"rejected\",\"reason\":{}},{\"status\":\"fulfilled\",\"value\":4}]" } });
Object.defineProperty($dump.n1, "stack", { value: "    at main (src.js:5:43)\n    at <eval> (src.js:9:5)\n", writable: true, configurable: true });
Object.defineProperty($dump.n11[0].reason, "stack", { value: "    at main (src.js:4:54)\n    at <eval> (src.js:9:5)\n", writable: true, configurable: true });
