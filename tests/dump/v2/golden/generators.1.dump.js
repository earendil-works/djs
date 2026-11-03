"djs-dump 2";
$dump.p1 = $dump.pending(1, "later", ["q1"]);
$dump.p2 = $dump.pending(2, "later", [0]);
var g = function* g(a) { const x = yield a; yield x + 1; };
var ag = async function* ag() { yield await later("q1"); yield "q2"; };
async function main() {
  const mid = g(1); mid.next();
  const fresh = g(5);
  const done = g(0); done.next(); done.next(); done.next();
  const it = ag();
  const r1 = it.next(), r2 = it.next();
  await later(0);
  print(mid.next(10).value, fresh.next().value, done.next().done);
  print((await r1).value, (await r2).value);
}
$dump.p3 = new Promise(() => {});
$dump.p4 = new Promise(() => {});
$dump.a5 = $dump.resume(ag, { at: "await#1", awaiting: $dump.p1, queue: [{ type: "next", value: undefined, promise: $dump.p3 }, { type: "next", value: undefined, promise: $dump.p4 }] });
$dump.a6 = $dump.resume(g, { at: "yield#1", args: [1], bindings: { a: 1, x: undefined } });
$dump.a7 = Reflect.apply(g, undefined, [5]);
$dump.a8 = Object.setPrototypeOf(((g) => (g.return(), g))((function* () {})()), g.prototype);
$dump.a9 = $dump.resume(main, { at: "await#1", awaiting: $dump.p2, bindings: { mid: $dump.a6, fresh: $dump.a7, done: $dump.a8, it: $dump.a5, r1: $dump.p3, r2: $dump.p4 } });
