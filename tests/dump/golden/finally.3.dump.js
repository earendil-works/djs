"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 8);
globalThis.$dump = {};
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
$dump.p1 = $djs.resume(7, "later", ["r"]);
var gen = function* gen() {
  try {
    yield 1;
    yield 2;
  } finally {
    yield "cleanup";
  }
};
$dump.n2 = [];
$dump.a3_0 = Array.prototype.push;
$dump.Ka3 = async function () {
const $djs_R = $djs.R;
const log = $dump.n2;
{ const $djs_c1 = "ret"; {

const $djs_t0 = $dump.a3_0, $djs_t1 = $dump.n2;
$djs_R($djs_t0, $djs_t1, [(await $dump.p1)]);
print("returned finally", log.join());
} return $djs_c1; }
};
Object.defineProperty($dump.Ka3, "$djs", { value: { f: returned, segs: [187,122,38], scaf: [["finally",48,65,2,"$djs_c1"],["k",0,141],["k",0,148],["k",0,157],["site",99,168],["tmp",0,167,"call.arg@99","array.element"],["k",0,166],["site",164,227]], cmap: [["call.callee@90",0,"$djs_t0"],["call.this@90",0,"$djs_t1"]] } });
$dump.a3 = $dump.Ka3.call(undefined);
$dump.a4_0 = print;
$dump.Ka4 = async function () {
const $djs_R = $djs.R;

const $djs_t0 = $dump.a4_0;
$djs_R($djs_t0, undefined, ["returned", (await $dump.a3)]);
print("nested", await nested());
const g = gen();
const seen = [g.next().value];
await later("g");
seen.push(g.return("early").value);
await later("g2");
seen.push(JSON.stringify(g.next()));
print("gen", seen.join(" "));
print("cases", await cases("b"), await cases("c"));
};
Object.defineProperty($dump.Ka4, "$djs", { value: { f: main, at: "await#3", segs: [132,181,32,165,216,16,182,235,30,213,268,17,231,288,35,267,326,18,286,347,36,323,386,29,353,418,51], scaf: [["k",0,72],["k",0,79],["k",0,88],["tmp",0,100,"call.arg@148","array.element"],["site",160,113],["tmp",0,112,"call.arg@160","array.element"],["k",0,99]], cmap: [["call.callee@142",0,"$djs_t0"],["call.this@142",1,undefined],["call.arg@148",1,"returned"]] } });
$dump.a4 = $dump.Ka4.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
