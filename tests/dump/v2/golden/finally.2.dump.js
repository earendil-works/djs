"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(4, "later", ["t"]);
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
$dump.n2 = new Error("thrown");
$dump.a3 = $dump.resume(thrown, { at: "await#1", awaiting: $dump.p1, slots: { "finally.value@69": $dump.n2, "finally.return@69": 1, "call.callee@83": print, "call.this@83": undefined, "call.arg@89": "thrown finally" } });
$dump.a4 = $dump.resume(main, { at: "await#2", awaiting: $dump.a3, slots: { g: undefined, seen: undefined } });
Object.defineProperty($dump.n2, "stack", { value: "    at thrown (src.js:14:20)\n    at main (src.js:66:17)\n", writable: true, configurable: true });
