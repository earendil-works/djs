"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(5, "later", ["y"]);
var letters = function* letters() {
  yield "a";
  yield undefined;
  yield "c";
};
async function objects(o) {
  const { a = await later("a"), [await later("k")]: { b = await later("b") } = {}, ...rest } = o;
  return [a, b, JSON.stringify(rest)].join(" ");
}
async function arrays() {
  const [x, y = await later("y"), [z = await later("z")] = [], ...more] = letters();
  return [x, y, z, more.length].join(" ");
}
async function assigns() {
  const t = { list: [] };
  let n;
  const r = ([t.p, t.list[await later(0)] = await later("d"), n = 1] = [1]);
  ({ q: t["q" + (await later(2))] = await later("e") } = {});
  return JSON.stringify(t) + " " + n + " " + r.length;
}
var defaults = function* defaults(v) {
  const { a = yield "need a", b: [c = yield "need c"] = [] } = v;
  return a + c;
};
async function main() {
  print("objects", await objects({ k: {}, extra: 1 }));
  print("arrays", await arrays());
  print("assigns", await assigns());
  const g = defaults({});
  const seen = [g.next().value];
  await later("g");
  seen.push(g.next("A").value);
  await later("g2");
  seen.push(g.next("C").value);
  print("defaults", seen.join(" "));
}
$dump.a2 = $dump.resume(letters, { at: "yield#2" });
$dump.a3 = $dump.resume(arrays, { at: "await#1", awaiting: $dump.p1, bindings: { x: "a", y: undefined, z: undefined, more: undefined }, operands: [["pattern.next", $dump.builtin("%GeneratorPrototype%.next")], ["pattern.iterator", $dump.a2]] });
$dump.a4 = $dump.resume(main, { at: "await#2", awaiting: $dump.a3, bindings: { g: undefined, seen: undefined }, operands: [["call.callee", print], ["call.this", undefined], ["call.arg[0]", "arrays"]] });
