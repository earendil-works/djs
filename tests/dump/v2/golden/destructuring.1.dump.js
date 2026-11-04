"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(1, "later", ["a"]);
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
$dump.n2 = { k: {}, extra: 1 };
$dump.a3 = $dump.resume(objects, { at: "await#1", awaiting: $dump.p1, args: [$dump.n2], slots: { o: $dump.n2, a: undefined, b: undefined, rest: undefined, "pattern.source@36": $dump.n2, "pattern.excluded@36": ["a"] } });
$dump.a4 = $dump.resume(main, { at: "await#1", awaiting: $dump.a3, slots: { g: undefined, seen: undefined, "call.callee@26": print, "call.this@26": undefined, "call.arg@32": "objects" } });
