"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.n1 = new Error("e");
$dump.p2 = $dump.pending(3, "later", [$dump.n1, true]);
$dump.p3 = $dump.pending(4, "later", [4]);
$dump.n4 = new Error("x");
$dump.p5 = $dump.pending(5, "later", [$dump.n4, true]);
$dump.p6 = $dump.pending(6, "later", ["ok"]);
$dump.p7 = $dump.pending(7, "later", ["fast"]);
async function main() {
  const all = Promise.all([1, later(2), later(3)]);
  const settled = Promise.allSettled([later(new Error("e"), true), later(4)]);
  const any = Promise.any([later(new Error("x"), true), later("ok")]);
  const race = Promise.race([later("fast"), new Promise(() => {})]);
  print(JSON.stringify(await all), JSON.stringify(await settled), await any, await race);
}
$dump.p8 = Promise.allSettled([$dump.p2, $dump.p3]);
$dump.p9 = Promise.any([$dump.p5, $dump.p6]);
$dump.p10 = new Promise((res, rej) => { $dump.r11 = [res, rej]; });
$dump.p12 = Promise.prototype.then.call($dump.p7, $dump.r11[0], $dump.r11[1]);
$dump.n13 = Promise.resolve([1, 2, 3]);
$dump.a14 = $dump.resume(main, { at: "await#2", awaiting: $dump.p8, slots: { all: $dump.n13, settled: $dump.p8, any: $dump.p9, race: $dump.p10, "call.callee@297": print, "call.this@297": undefined, "call.arg@303": "[1,2,3]", "call.callee@330": JSON.stringify, "call.this@330": JSON } });
Object.defineProperty($dump.n1, "stack", { value: "    at main (src.js:4:54)\n    at <eval> (src.js:9:5)\n", writable: true, configurable: true });
Object.defineProperty($dump.n4, "stack", { value: "    at main (src.js:5:43)\n    at <eval> (src.js:9:5)\n", writable: true, configurable: true });
