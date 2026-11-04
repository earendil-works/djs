"djs-dump 2";
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
const P = (id, ...args) => $dump.pending(id, "later", args);
$dump.s = Promise.allSettled(["x", Promise.reject(new Error("bad")), P(6, "y")]);
$dump.y = Promise.any([P(7, new Error("no1"), true), P(8, "any"), P(9, new Error("no2"), true)]);
$dump.r = Promise.race([P(10, "fast"), P(11, "slow")]);
$dump.g = $dump.resume(guarded, { at: "await#1", awaiting: P(12, new Error("thrown"), true) });
$dump.rj = $dump.resume(rejecting, {
  at: "await#1",
  awaiting: P(13, new Error("inner"), true),
  args: ["inner"],
  slots: { tag: "inner", v: $dump.TDZ },
});
$dump.handle("main", $dump.resume(main, {
  at: "await#2",
  awaiting: $dump.s,
  slots: {
    a: Promise.resolve([1, 2, 3]),
    s: $dump.s,
    y: $dump.y,
    r: $dump.r,
    g: $dump.g,
    rj: $dump.rj,
    all: $dump.TDZ,
    "call.callee@345": print,
    "call.this@345": undefined,
    "call.callee@351": JSON.stringify,
    "call.this@351": JSON,
  },
}));
$dump.issued(14);
