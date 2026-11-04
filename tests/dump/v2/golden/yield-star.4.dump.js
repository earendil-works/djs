"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(4, "later", ["x1"]);
var inner = function* inner(tag) {
  const got = yield tag + "1";
  yield tag + "2:" + got;
  return tag + " done";
};
var outer = function* outer() {
  const r = yield* inner("i");
  yield* ["a1", "a2"];
  return r;
};
var aouter = async function* aouter() {
  yield* ainner();
  yield* ["s1", later("s2")];
};
var ainner = async function* ainner() {
  yield await later("x1");
  try {
    yield "x2";
  } finally {
    print("ainner closed");
  }
};
async function main() {
  const g = outer();
  const seen = [g.next().value];
  await later(0);
  seen.push(g.next("v").value, g.next().value);
  await later(1);
  seen.push(g.next().value, JSON.stringify(g.next()));
  const ag = aouter();
  for await (const v of ag) {
    seen.push(v);
    await later(v);
  }
  const ag2 = aouter();
  seen.push((await ag2.next()).value, (await ag2.next()).value);
  await later(2);
  seen.push(JSON.stringify(await ag2.return("early")));
  const g2 = outer();
  g2.next();
  try {
    g2.throw(new Error("boom"));
  } catch (e) {
    seen.push("caught " + e.message);
  }
  print(seen.join(" "));
}
$dump.a2 = Object.setPrototypeOf(((g) => (g.return(), g))((function* () {})()), outer.prototype);
$dump.a3 = $dump.resume(ainner, { at: "yield#1" });
$dump.a4 = $dump.resume(aouter, { at: "yield*#1", slots: { "yield*.iterator@29": $dump.a3, "yield*.next@29": $dump.builtin("%AsyncGeneratorPrototype%.next") } });
$dump.a5 = $dump.resume(main, { at: "await#3", awaiting: $dump.p1, slots: { g: $dump.a2, seen: ["i1", "i2:v", "a1", "a2", "{\"value\":\"i done\",\"done\":true}", "x1"], ag: $dump.a4, v: "x1", ag2: undefined, g2: undefined, "for-await.iterator@242": $dump.a4, "for-await.next@242": $dump.builtin("%AsyncGeneratorPrototype%.next") } });
