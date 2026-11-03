"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(1, "later", [2]);
$dump.p2 = $dump.pending(2, "later", [3]);
$dump.n3 = new Error("e");
$dump.p4 = $dump.pending(3, "later", [$dump.n3, true]);
$dump.p5 = $dump.pending(4, "later", [4]);
$dump.n6 = new Error("x");
$dump.p7 = $dump.pending(5, "later", [$dump.n6, true]);
$dump.p8 = $dump.pending(6, "later", ["ok"]);
$dump.p9 = $dump.pending(7, "later", ["fast"]);
async function main() {
  const all = Promise.all([1, later(2), later(3)]);
  const settled = Promise.allSettled([later(new Error("e"), true), later(4)]);
  const any = Promise.any([later(new Error("x"), true), later("ok")]);
  const race = Promise.race([later("fast"), new Promise(() => {})]);
  print(JSON.stringify(await all), JSON.stringify(await settled), await any, await race);
}
$dump.p10 = Promise.all([1, $dump.p1, $dump.p2]);
$dump.p11 = Promise.allSettled([$dump.p4, $dump.p5]);
$dump.p12 = Promise.any([$dump.p7, $dump.p8]);
$dump.p13 = new Promise((res, rej) => { $dump.r14 = [res, rej]; });
$dump.p15 = Promise.prototype.then.call($dump.p9, $dump.r14[0], $dump.r14[1]);
$dump.a16 = $dump.resume(main, { at: "await#1", awaiting: $dump.p10, bindings: { all: $dump.p10, settled: $dump.p11, any: $dump.p12, race: $dump.p13 }, operands: [["call.callee", print], ["call.this", undefined], ["call.callee", JSON.stringify], ["call.this", JSON]] });
Object.defineProperty($dump.n3, "stack", { value: "    at main (src.js:4:54)\n    at <eval> (src.js:9:5)\n", writable: true, configurable: true });
Object.defineProperty($dump.n6, "stack", { value: "    at main (src.js:5:43)\n    at <eval> (src.js:9:5)\n", writable: true, configurable: true });
