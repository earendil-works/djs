"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 2);
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
$dump.p1 = $djs.resume(1, "later", [2]);
var gen = function* gen() {
  try {
    yield 1;
    yield 2;
  } finally {
    yield "cleanup";
  }
};
$dump.a2_0 = print;
$dump.Ka2 = async function () {
const $djs_R = $djs.R;
var x = 2;
{ const $djs_c1 = undefined; {

const $djs_t0 = $dump.a2_0;
$djs_R($djs_t0, undefined, ["normal finally", (await $dump.p1)]);
}  }
return x;
};
Object.defineProperty($dump.Ka2, "$djs", { value: { f: normal, segs: [185,115,9], scaf: [["finally",29,54,0,"$djs_c1"],["k",0,114],["k",0,121],["k",0,130],["tmp",0,142,"call.arg@74","array.element"],["site",92,161],["tmp",0,160,"call.arg@92","array.element"],["k",0,141],["site",112,181]], cmap: [["call.callee@68",0,"$djs_t0"],["call.this@68",1,undefined],["call.arg@74",1,"normal finally"]] } });
$dump.a2 = $dump.Ka2.call(undefined);
$dump.a3_0 = print;
$dump.Ka3 = async function () {
const $djs_R = $djs.R;

const $djs_t0 = $dump.a3_0;
$djs_R($djs_t0, undefined, ["normal", (await $dump.a2)]);
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
};
Object.defineProperty($dump.Ka3, "$djs", { value: { f: main, segs: [130,62,77,208,142,36,245,181,32,278,216,16,295,235,30,326,268,17,344,288,35,380,326,18,399,347,36,436,386,29,466,418,51], scaf: [["k",0,72],["k",0,79],["k",0,88],["tmp",0,100,"call.arg@32","array.element"],["site",42,111],["tmp",0,110,"call.arg@42","array.element"],["k",0,99]], cmap: [["call.callee@26",0,"$djs_t0"],["call.this@26",1,undefined],["call.arg@32",1,"normal"]] } });
$dump.a3 = $dump.Ka3.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
