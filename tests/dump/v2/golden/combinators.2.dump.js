"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(2, "later", [3]);
$dump.n2 = new Error("e");
$dump.p3 = $dump.pending(3, "later", [$dump.n2, true]);
$dump.p4 = $dump.pending(4, "later", [4]);
$dump.n5 = new Error("x");
$dump.p6 = $dump.pending(5, "later", [$dump.n5, true]);
$dump.p7 = $dump.pending(6, "later", ["ok"]);
$dump.p8 = $dump.pending(7, "later", ["fast"]);
async function main() {
  const all = Promise.all([1, later(2), later(3)]);
  const settled = Promise.allSettled([later(new Error("e"), true), later(4)]);
  const any = Promise.any([later(new Error("x"), true), later("ok")]);
  const race = Promise.race([later("fast"), new Promise(() => {})]);
  print(JSON.stringify(await all), JSON.stringify(await settled), await any, await race);
}
$dump.p9 = Promise.all([1, 2, $dump.p1]);
$dump.p10 = Promise.allSettled([$dump.p3, $dump.p4]);
$dump.p11 = Promise.any([$dump.p6, $dump.p7]);
$dump.p12 = new Promise((res, rej) => { $dump.r13 = [res, rej]; });
$dump.p14 = Promise.prototype.then.call($dump.p8, $dump.r13[0], $dump.r13[1]);
$dump.a15 = $dump.resume(main, { at: "await#1", awaiting: $dump.p9, bindings: { all: $dump.p9, settled: $dump.p10, any: $dump.p11, race: $dump.p12 }, operands: [["call.callee", print], ["call.this", undefined], ["call.callee", JSON.stringify], ["call.this", JSON]] });
Object.defineProperty($dump.n2, "stack", { value: "    at main (src.js:4:54)\n    at <eval> (src.js:9:5)\n", writable: true, configurable: true });
Object.defineProperty($dump.n5, "stack", { value: "    at main (src.js:5:43)\n    at <eval> (src.js:9:5)\n", writable: true, configurable: true });
