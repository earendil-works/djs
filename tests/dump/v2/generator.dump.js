// djs-dump 2: 2 activations, 1 pending host call.
// at idle point 2: counter at its yield (i = 1), main waits on later(10).
function* counter(limit) {
  let total = 0;
  for (let i = 0; i < limit; i++) {
    const got = yield i;
    total += got;
  }
  return total;
}
async function main() {
  const g = counter(4);
  let r = g.next();
  while (!r.done) {
    const v = await later(r.value * 10);
    print("got", v);
    r = g.next(v);
  }
  print("total", r.value);
}
$dump.g = $dump.resume(counter, {
  at: "yield#1",
  args: [4],
  bindings: { limit: 4, total: 0, i: 1, got: $dump.TDZ },
});
$dump.handle("main", $dump.resume(main, {
  at: "await#1",
  awaiting: $dump.pending(2, "later", [10]),
  bindings: { g: $dump.g, r: { value: 1, done: false }, v: $dump.TDZ },
}));
