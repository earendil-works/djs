"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 14);
globalThis.$dump = {};
$dump.b0 = Object.getPrototypeOf([][Symbol.iterator]())["next"];
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
$dump.p1 = $djs.resume(13, "later", [-2]);
var gen = function* gen() {
  try {
    yield 1;
    yield 2;
  } finally {
    yield "cleanup";
  }
};
$dump.n2 = ["inner 1 1", "outer 1 -1", "body 2", "inner 2 2"];
$dump.i3 = $djs.advance(Array.prototype.values.call([1, 2, 3]), 2);
$dump.a4_0 = Array.prototype.push;
$dump.Ka4 = async function () {
const $djs_R = $djs.R, $djs_L = $djs.L;
const out = $dump.n2;
{ let $djs_f1 = true; for (const $djs_v1 of $djs_L($dump.i3, $dump.b0)) { if ($djs_f1) { $djs_f1 = false; {
const i = 2;

{

{ const $djs_c3 = undefined; {

const $djs_t0 = $dump.a4_0, $djs_t1 = $dump.n2;
$djs_R($djs_t0, $djs_t1, [("outer 2 " + (await $dump.p1))]);
}  }
}
} } else { const i = $djs_v1; {
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
  } } } }
return out.join(", ");
};
Object.defineProperty($dump.Ka4, "$djs", { value: { f: nested, at: "await#2", segs: [366,51,7,385,73,278,670,354,22], scaf: [["for-of",46,104],["finally",79,207,0,"$djs_c3"],["k",0,287],["k",0,294],["k",0,303],["site",323,328],["tmp",0,313,"call.arg@301","array.element"],["k",0,312],["site",347,349],["site",351,354],["site",351,356]], cmap: [["call.callee@292",0,"$djs_t0"],["call.this@292",0,"$djs_t1"],["binary.left@301",1,"outer 2 "]] } });
$dump.a4 = $dump.Ka4.call(undefined);
$dump.a5_0 = print;
$dump.Ka5 = async function () {
const $djs_R = $djs.R;

const $djs_t0 = $dump.a5_0;
$djs_R($djs_t0, undefined, ["nested", (await $dump.a4)]);
const g = gen();
const seen = [g.next().value];
await later("g");
seen.push(g.return("early").value);
await later("g2");
seen.push(JSON.stringify(g.next()));
print("gen", seen.join(" "));
print("cases", await cases("b"), await cases("c"));
};
Object.defineProperty($dump.Ka5, "$djs", { value: { f: main, at: "await#4", segs: [130,216,16,147,235,30,178,268,17,196,288,35,232,326,18,251,347,36,288,386,29,318,418,51], scaf: [["k",0,72],["k",0,79],["k",0,88],["tmp",0,100,"call.arg@187","array.element"],["site",197,111],["tmp",0,110,"call.arg@197","array.element"],["k",0,99]], cmap: [["call.callee@181",0,"$djs_t0"],["call.this@181",1,undefined],["call.arg@187",1,"nested"]] } });
$dump.a5 = $dump.Ka5.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
