// A generator suspended at a yield, driven by an async function that awaits between
// steps.
// generator.dump.js, written by hand, is this program at idle point 2: counter at its
// yield (i = 1), main waits on later(10).
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
main();
