"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 21);
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
$dump.p1 = $djs.resume(20, "later", ["a"]);
var gen = function* gen() {
  try {
    yield 1;
    yield 2;
  } finally {
    yield "cleanup";
  }
};
$dump.Ka2 = async function () {
var d = "b";
switch ("b") {
case (await $dump.p1):
return "is a";
case await later("b"):
return "is b";
default:
return "neither";
}
};
Object.defineProperty($dump.Ka2, "$djs", { value: { f: cases, segs: [33,28,8,48,45,5,71,74,14,86,93,21,109,122,14,124,141,7,133,156,17], scaf: [["site",50,54],["switch",28,33]], cmap: [["binary.left@45",1,"b"],["switch.discriminant@28",1,"b"]] } });
$dump.a2 = $dump.Ka2.call(undefined);
$dump.a3 = Object.setPrototypeOf(((g) => (g.return(), g))((function* () {})()), gen.prototype);
$dump.a4_0 = print;
$dump.Ka4 = async function () {
const $djs_R = $djs.R;
const g = $dump.a3;
const seen = [1, "cleanup", "{\"value\":\"early\",\"done\":true}"];
const $djs_t0 = $dump.a4_0;
$djs_R($djs_t0, undefined, ["cases", (await $dump.a2), await cases("c")]);
};
Object.defineProperty($dump.Ka4, "$djs", { value: { f: main, at: "await#7", segs: [214,451,16], scaf: [["k",0,159],["k",0,166],["k",0,175],["tmp",0,187,"call.arg@424","array.element"],["site",433,197],["tmp",0,196,"call.arg@433","array.element"],["tmp",0,214,"call.arg@451","array.element"],["k",0,186]], cmap: [["call.callee@418",0,"$djs_t0"],["call.this@418",1,undefined],["call.arg@424",1,"cases"]] } });
$dump.a4 = $dump.Ka4.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
