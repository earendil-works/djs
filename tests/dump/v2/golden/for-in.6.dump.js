"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(7, "later", ["0"]);
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
$dump.n2 = ["000", "011"];
$dump.a3 = $dump.resume(nested, { at: "await#1", awaiting: $dump.p1, bindings: { out: $dump.n2, i: "1", c: "0" }, operands: [["call.callee", Array.prototype.push], ["call.this", $dump.n2], ["binary.left", "10"]], loops: [{ kind: "for-in", iterator: $dump.forIn([10, 20], []) }, { kind: "for-in", iterator: $dump.forIn(undefined, ["1"]) }] });
$dump.a4 = $dump.resume(main, { at: "await#2", awaiting: $dump.a3, bindings: { g: undefined, seen: undefined }, operands: [["call.callee", print], ["call.this", undefined], ["call.arg[0]", "nested"]] });
