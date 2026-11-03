// A sync generator at a yield, one not started, one finished; an async generator with
// a request queue.
function* g(a) { const x = yield a; yield x + 1; }
async function* ag() { yield await later("q1"); yield "q2"; }
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
main();
