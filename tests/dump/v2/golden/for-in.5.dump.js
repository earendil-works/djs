"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(6, "later", ["1"]);
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
$dump.n2 = ["000"];
$dump.a3 = $dump.resume(nested, { at: "await#1", awaiting: $dump.p1, slots: { out: $dump.n2, i: "0", c: "1", "for-in.iterator@46": $dump.forIn([10, 20], ["1"]), "for-in.iterator@78": $dump.forIn(undefined, []), "call.callee@100": Array.prototype.push, "call.this@100": $dump.n2, "binary.left@109": "01" } });
$dump.a4 = $dump.resume(main, { at: "await#2", awaiting: $dump.a3, slots: { g: undefined, seen: undefined, "call.callee@57": print, "call.this@57": undefined, "call.arg@63": "nested" } });
