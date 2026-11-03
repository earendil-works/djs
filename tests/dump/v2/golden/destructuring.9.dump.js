"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(12, "later", ["g"]);
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
$dump.n2 = {};
$dump.a3 = $dump.resume(defaults, { at: "yield#1", args: [$dump.n2], bindings: { v: $dump.n2, a: undefined, c: undefined }, operands: [["pattern.source", $dump.n2]] });
$dump.a4 = $dump.resume(main, { at: "await#4", awaiting: $dump.p1, bindings: { g: $dump.a3, seen: ["need a"] } });
