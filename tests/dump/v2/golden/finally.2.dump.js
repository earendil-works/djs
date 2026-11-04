"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 5);
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
$dump.p1 = $djs.resume(4, "later", ["t"]);
var gen = function* gen() {
  try {
    yield 1;
    yield 2;
  } finally {
    yield "cleanup";
  }
};
$dump.n2 = new Error("thrown");
$dump.a3_0 = print;
$dump.Ka3 = async function () {
const $djs_R = $djs.R;

{ const $djs_c1 = $dump.n2; {

const $djs_t0 = $dump.a3_0;
$djs_R($djs_t0, undefined, ["thrown finally", (await $dump.p1)]);
} throw $djs_c1; }
};
Object.defineProperty($dump.Ka3, "$djs", { value: { f: thrown, scaf: [["finally",28,44,1,"$djs_c1"],["k",0,103],["k",0,110],["k",0,119],["tmp",0,131,"call.arg@89","array.element"],["site",107,150],["tmp",0,149,"call.arg@107","array.element"],["k",0,130],["site",129,170]], cmap: [["call.callee@83",0,"$djs_t0"],["call.this@83",1,undefined],["call.arg@89",1,"thrown finally"]] } });
$dump.a3 = $dump.Ka3.call(undefined);
$dump.Ka4 = async function () {

try {

(await $dump.a3);
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
};
Object.defineProperty($dump.Ka4, "$djs", { value: { f: main, at: "await#2", segs: [55,99,1,58,102,37,96,142,36,133,181,32,166,216,16,183,235,30,214,268,17,232,288,35,268,326,18,287,347,36,324,386,29,354,418,51], scaf: [["try",62,21],["site",72,29],["site",91,47]] } });
$dump.a4 = $dump.Ka4.call(undefined);
Object.defineProperty($dump.n2, "stack", { value: "    at thrown (src.js:14:20)\n    at main (src.js:66:17)\n", writable: true, configurable: true });
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
