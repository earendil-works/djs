"djs-dump 2";
$dump.rejected = (e) => {
  const p = Promise.reject(e);
  p.catch(() => {});
  return p;
};
$dump.p1 = $dump.pending(4, "later", [4]);
$dump.n2 = new Error("x");
$dump.p3 = $dump.pending(5, "later", [$dump.n2, true]);
$dump.p4 = $dump.pending(6, "later", ["ok"]);
$dump.p5 = $dump.pending(7, "later", ["fast"]);
async function main() {
  const all = Promise.all([1, later(2), later(3)]);
  const settled = Promise.allSettled([later(new Error("e"), true), later(4)]);
  const any = Promise.any([later(new Error("x"), true), later("ok")]);
  const race = Promise.race([later("fast"), new Promise(() => {})]);
  print(JSON.stringify(await all), JSON.stringify(await settled), await any, await race);
}
$dump.n6 = new Error("e");
$dump.p7 = Promise.allSettled([$dump.rejected($dump.n6), $dump.p1]);
$dump.p8 = Promise.any([$dump.p3, $dump.p4]);
$dump.p9 = new Promise((res, rej) => { $dump.r10 = [res, rej]; });
$dump.p11 = Promise.prototype.then.call($dump.p5, $dump.r10[0], $dump.r10[1]);
$dump.n12 = Promise.resolve([1, 2, 3]);
$dump.a13 = $dump.resume(main, { at: "await#2", awaiting: $dump.p7, bindings: { all: $dump.n12, settled: $dump.p7, any: $dump.p8, race: $dump.p9 }, operands: [["call.callee", print], ["call.this", undefined], ["call.arg[0]", "[1,2,3]"], ["call.callee", JSON.stringify], ["call.this", JSON]] });
Object.defineProperty($dump.n2, "stack", { value: "    at main (src.js:5:43)\n    at <eval> (src.js:9:5)\n", writable: true, configurable: true });
Object.defineProperty($dump.n6, "stack", { value: "    at main (src.js:4:54)\n    at <eval> (src.js:9:5)\n", writable: true, configurable: true });
