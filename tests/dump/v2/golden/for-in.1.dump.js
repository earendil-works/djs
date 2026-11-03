"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(1, "later", [1]);
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
$dump.n2 = { inherited: 1 };
$dump.n3 = { __proto__: $dump.n2, a: 1, b: 2, c: 3 };
$dump.n4 = [];
$dump.a5 = $dump.resume(keys, { at: "await#1", awaiting: $dump.p1, bindings: { proto: $dump.n2, o: $dump.n3, seen: $dump.n4, k: "a" }, operands: [["call.callee", Array.prototype.push], ["call.this", $dump.n4], ["binary.left", "a="]], loops: [{ kind: "for-in", iterator: $dump.forIn($dump.n3, ["b", "c", "inherited"]) }] });
$dump.a6 = $dump.resume(main, { at: "await#1", awaiting: $dump.a5, bindings: { g: undefined, seen: undefined }, operands: [["call.callee", print], ["call.this", undefined], ["call.arg[0]", "keys"]] });
