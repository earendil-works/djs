"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(7, "later", ["fast"]);
async function main() {
  const all = Promise.all([1, later(2), later(3)]);
  const settled = Promise.allSettled([later(new Error("e"), true), later(4)]);
  const any = Promise.any([later(new Error("x"), true), later("ok")]);
  const race = Promise.race([later("fast"), new Promise(() => {})]);
  print(JSON.stringify(await all), JSON.stringify(await settled), await any, await race);
}
$dump.p2 = new Promise((res, rej) => { $dump.r3 = [res, rej]; });
$dump.p4 = Promise.prototype.then.call($dump.p1, $dump.r3[0], $dump.r3[1]);
$dump.n5 = Promise.resolve([1, 2, 3]);
$dump.n7 = [{ status: "rejected", reason: new Error("e") }, { status: "fulfilled", value: 4 }];
$dump.n6 = Promise.resolve($dump.n7);
$dump.n8 = Promise.resolve("ok");
$dump.a9 = $dump.resume(main, { at: "await#4", awaiting: $dump.p2, bindings: { all: $dump.n5, settled: $dump.n6, any: $dump.n8, race: $dump.p2 }, operands: [["call.callee", print], ["call.this", undefined], ["call.arg[0]", "[1,2,3]"], ["call.arg[1]", "[{\"status\":\"rejected\",\"reason\":{}},{\"status\":\"fulfilled\",\"value\":4}]"], ["call.arg[2]", "ok"]] });
Object.defineProperty($dump.n7[0].reason, "stack", { value: "    at main (src.js:4:54)\n    at <eval> (src.js:9:5)\n", writable: true, configurable: true });
