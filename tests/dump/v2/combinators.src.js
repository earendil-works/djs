// Promise.all, allSettled, any and race in flight; rejections caught in an activation
// (a try around an await) and by its caller.
// combinators.dump.js, written by hand, is this program at idle point 6: calls 1-5
// delivered (Promise.all done, allSettled half way: "x" fulfilled, "bad" rejected, the
// third open), main at its second await.
const ok = (v) => later(v);
const fail = (m) => later(new Error(m), true);
async function guarded() {
  try {
    return await fail("thrown");
  } catch (e) {
    return "caught " + e.message;
  }
}
async function rejecting(tag) {
  const v = await fail(tag);
  return v;
}
async function main() {
  const a = Promise.all([ok(1), ok(2), ok(3)]);
  const s = Promise.allSettled([ok("x"), fail("bad"), ok("y")]);
  const y = Promise.any([fail("no1"), ok("any"), fail("no2")]);
  const r = Promise.race([ok("fast"), ok("slow")]);
  const g = guarded();
  const rj = rejecting("inner");
  print(JSON.stringify(await a));
  print(JSON.stringify((await s).map((e) => e.status + ":" + (e.value ?? e.reason.message))));
  print(await y);
  print(await r);
  print(await g);
  try {
    await rj;
  } catch (e) {
    print("main caught", e.message);
  }
  const all = await Promise.allSettled([rejecting("late")]);
  print(all[0].reason.message);
}
main();
