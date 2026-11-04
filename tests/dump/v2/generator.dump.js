"djs-dump 2";
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
  slots: { limit: 4, total: 0, i: 1, got: $dump.TDZ },
});
$dump.handle("main", $dump.resume(main, {
  at: "await#1",
  awaiting: $dump.pending(2, "later", [10]),
  slots: { g: $dump.g, r: { value: 1, done: false }, v: $dump.TDZ },
}));
