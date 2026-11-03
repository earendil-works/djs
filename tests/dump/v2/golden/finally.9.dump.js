"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(15, "later", [-3]);
async function normal(x) {
  try {
    x = x + 1;
  } finally {
    print("normal finally", await later(x));
  }
  return x;
}
async function thrown() {
  try {
    throw new Error("thrown");
  } finally {
    print("thrown finally", await later("t"));
  }
}
async function returned() {
  const log = [];
  try {
    return "ret";
  } finally {
    log.push(await later("r"));
    print("returned finally", log.join());
  }
}
async function nested() {
  const out = [];
  for (const i of [1, 2, 3]) {
    try {
      try {
        if (i === 1) continue;
        if (i === 3) break;
        out.push("body " + i);
      } finally {
        out.push("inner " + i + " " + (await later(i)));
      }
    } finally {
      out.push("outer " + i + " " + (await later(-i)));
    }
  }
  return out.join(", ");
}
var gen = function* gen() {
  try {
    yield 1;
    yield 2;
  } finally {
    yield "cleanup";
  }
};
async function cases(d) {
  switch (d) {
    case await later("a"):
      return "is a";
    case await later("b"):
      return "is b";
    default:
      return "neither";
  }
}
async function main() {
  print("normal", await normal(1));
  try {
    await thrown();
  } catch (e) {
    print("caught", e.message);
  }
  print("returned", await returned());
  print("nested", await nested());
  const g = gen();
  const seen = [g.next().value];
  await later("g");
  seen.push(g.return("early").value);
  await later("g2");
  seen.push(JSON.stringify(g.next()));
  print("gen", seen.join(" "));
  print("cases", await cases("b"), await cases("c"));
}
$dump.n2 = ["inner 1 1", "outer 1 -1", "body 2", "inner 2 2", "outer 2 -2", "inner 3 3"];
$dump.i3 = $dump.advance(Array.prototype.values.call([1, 2, 3]), 3);
$dump.a4 = $dump.resume(nested, { at: "await#2", awaiting: $dump.p1, bindings: { out: $dump.n2, i: 3 }, operands: [["finally.value", undefined], ["finally.return", 1], ["call.callee", Array.prototype.push], ["call.this", $dump.n2], ["binary.left", "outer 3 "]], loops: [{ kind: "for-of", iterator: $dump.i3, next: $dump.builtin("%ArrayIteratorPrototype%.next") }] });
$dump.a5 = $dump.resume(main, { at: "await#4", awaiting: $dump.a4, bindings: { g: undefined, seen: undefined }, operands: [["call.callee", print], ["call.this", undefined], ["call.arg[0]", "nested"]] });
