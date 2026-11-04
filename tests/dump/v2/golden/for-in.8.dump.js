"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(10, "later", ["g"]);
async function keys() {
  const proto = { inherited: 1 };
  const o = Object.create(proto);
  o.a = 1;
  o.b = 2;
  o.c = 3;
  const seen = [];
  for (const k in o) {
    seen.push(k + "=" + (await later(o[k])));
    if (k === "a") {
      delete o.b;
      o.d = 4;
    }
  }
  return seen.join(" ");
}
async function nested() {
  const out = [];
  for (const i in [10, 20]) {
    for (const c in "xy") out.push(i + c + (await later(c)));
  }
  return out.join(" ");
}
var walk = function* walk(o) {
  for (const k in o) yield k;
};
async function main() {
  print("keys", await keys());
  print("nested", await nested());
  const g = walk({ p: 1, q: 2, r: 3 });
  const seen = [g.next().value];
  await later("g");
  seen.push(g.next().value, g.next().value);
  print("walk", seen.join(" "));
}
$dump.n2 = { p: 1, q: 2, r: 3 };
$dump.a3 = $dump.resume(walk, { at: "yield#1", args: [$dump.n2], slots: { o: $dump.n2, k: "p", "for-in.iterator@22": $dump.forIn($dump.n2, ["q", "r"]) } });
$dump.a4 = $dump.resume(main, { at: "await#3", awaiting: $dump.p1, slots: { g: $dump.a3, seen: ["p"] } });
