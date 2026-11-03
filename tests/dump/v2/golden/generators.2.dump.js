"djs-dump 2";
$dump.p1 = $dump.pending(2, "later", [0]);
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
$dump.a2 = $dump.resume(g, { at: "yield#1", args: [1], bindings: { a: 1, x: undefined } });
$dump.a3 = Reflect.apply(g, undefined, [5]);
$dump.a4 = Object.setPrototypeOf(((g) => (g.return(), g))((function* () {})()), g.prototype);
$dump.a5 = $dump.resume(ag, { at: "yield#2" });
$dump.n6 = Promise.resolve({ value: "q1", done: false });
$dump.n7 = Promise.resolve({ value: "q2", done: false });
$dump.a8 = $dump.resume(main, { at: "await#1", awaiting: $dump.p1, bindings: { mid: $dump.a2, fresh: $dump.a3, done: $dump.a4, it: $dump.a5, r1: $dump.n6, r2: $dump.n7 } });
