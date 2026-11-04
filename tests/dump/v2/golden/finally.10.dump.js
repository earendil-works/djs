"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 18);
globalThis.$dump = {};
$dump.b0 = Object.getPrototypeOf(function* () {}).prototype;
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
$dump.p1 = $djs.resume(17, "later", ["g"]);
var gen = function* gen() {
  try {
    yield 1;
    yield 2;
  } finally {
    yield "cleanup";
  }
};
$dump.n3 = { __proto__: $dump.b0 };
$dump.Ka2 = function* () {

try {

(yield);
yield 2;
} finally {
    yield "cleanup";
  }
};
Object.defineProperty($dump.Ka2, "$djs", { value: { f: gen, at: "yield#1", segs: [32,43,8,51,64,26], scaf: [["try",20,16],["site",30,24],["site",55,42]] } });
$dump.Ka2.prototype = $dump.n3;
$dump.a2 = $dump.Ka2.call(undefined);
$dump.a2.next();
$dump.Ka4 = async function () {
const g = $dump.a2;
const seen = [1];
(await $dump.p1);
seen.push(g.return("early").value);
await later("g2");
seen.push(JSON.stringify(g.next()));
print("gen", seen.join(" "));
print("cases", await cases("b"), await cases("c"));
};
Object.defineProperty($dump.Ka4, "$djs", { value: { f: main, at: "await#5", segs: [76,288,35,112,326,18,131,347,36,168,386,29,198,418,51], scaf: [["site",268,59]] } });
$dump.a4 = $dump.Ka4.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
