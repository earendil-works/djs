"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 19);
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
$dump.p1 = $djs.resume(18, "later", ["g2"]);
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

{ const $djs_c1 = "early"; {

(yield);
} return $djs_c1; }
};
Object.defineProperty($dump.Ka2, "$djs", { value: { f: gen, at: "yield#3", scaf: [["finally",20,16,2,"$djs_c1"],["site",70,47],["site",90,56]] } });
$dump.Ka2.prototype = $dump.n3;
$dump.a2 = $dump.Ka2.call(undefined);
$dump.a2.next();
$dump.Ka4 = async function () {
const g = $dump.a2;
const seen = [1, "cleanup"];
(await $dump.p1);
seen.push(JSON.stringify(g.next()));
print("gen", seen.join(" "));
print("cases", await cases("b"), await cases("c"));
};
Object.defineProperty($dump.Ka4, "$djs", { value: { f: main, at: "await#6", segs: [87,347,36,124,386,29,154,418,51], scaf: [["site",326,70]] } });
$dump.a4 = $dump.Ka4.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
