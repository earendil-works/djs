"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(2, "later", [1]);
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
$dump.i2 = $dump.advance(Array.prototype.values.call(["a1", "a2"]), 1);
$dump.a3 = $dump.resume(outer, { at: "yield*#2", bindings: { r: "i done" }, loops: [{ kind: "yield*", iterator: $dump.i2, next: $dump.builtin("%ArrayIteratorPrototype%.next") }] });
$dump.a4 = $dump.resume(main, { at: "await#2", awaiting: $dump.p1, bindings: { g: $dump.a3, seen: ["i1", "i2:v", "a1"], ag: undefined, ag2: undefined, g2: undefined } });
