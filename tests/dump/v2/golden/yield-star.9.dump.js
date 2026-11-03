"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(10, "later", ["x1"]);
var inner = function* inner(tag) {
  const got = yield tag + "1";
  yield tag + "2:" + got;
  return tag + " done";
};
var ainner = async function* ainner() {
  yield await later("x1");
  try {
    yield "x2";
  } finally {
    print("ainner closed");
  }
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
$dump.p2 = new Promise(() => {});
$dump.a3 = $dump.resume(ainner, { at: "await#1", awaiting: $dump.p1, queue: [{ type: "next", value: undefined, promise: $dump.p2 }] });
$dump.p4 = new Promise(() => {});
$dump.a5 = $dump.resume(aouter, { at: "yield*.await#5", awaiting: $dump.p2, loops: [{ kind: "yield*", iterator: $dump.a3, next: $dump.builtin("%AsyncGeneratorPrototype%.next") }], queue: [{ type: "next", value: undefined, promise: $dump.p4 }] });
$dump.a6 = Object.setPrototypeOf(((g) => (g.return(), g))((function* () {})()), outer.prototype);
$dump.n7 = ["i1", "i2:v", "a1", "a2", "{\"value\":\"i done\",\"done\":true}", "x1", "x2", "s1", "s2"];
$dump.a8 = Object.setPrototypeOf(((g) => (g.return(), g))((async function* () {})()), aouter.prototype);
$dump.a9 = $dump.resume(main, { at: "await#4", awaiting: $dump.p4, bindings: { g: $dump.a6, seen: $dump.n7, ag: $dump.a8, ag2: $dump.a5, g2: undefined }, operands: [["call.callee", Array.prototype.push], ["call.this", $dump.n7]] });
