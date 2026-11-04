"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(10, "later", ["e"]);
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
$dump.n2 = { list: ["d"], p: 1 };
$dump.n3 = {};
$dump.a4 = $dump.resume(assigns, { at: "await#4", awaiting: $dump.p1, slots: { t: $dump.n2, n: 1, r: [1], "assign.value@142": $dump.n3, "pattern.source@142": $dump.n3, "assign.object@147": $dump.n2, "assign.key@147": "q2" } });
$dump.a5 = $dump.resume(main, { at: "await#3", awaiting: $dump.a4, slots: { g: undefined, seen: undefined, "call.callee@117": print, "call.this@117": undefined, "call.arg@123": "assigns" } });
