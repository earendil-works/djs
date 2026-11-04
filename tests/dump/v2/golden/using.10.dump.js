"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 24);
globalThis.$dump = {};
function resource(name) {
  return {
    [Symbol.dispose]() {
      print("dispose", name);
    },
  };
}
function asyncResource(name, fail) {
  return {
    async [Symbol.asyncDispose]() {
      const v = await later(name);
      print("async dispose", v);
      if (fail) throw new Error("dispose " + name + " failed");
    },
  };
}
async function block() {
  {
    using a = resource("a");
    await using b = asyncResource("b");
    await using n = null;
    using c = resource("c");
    print("in block", await later("body"));
  }
  print("after block");
}
async function failing() {
  try {
    await using x = asyncResource("x", true);
    await using y = asyncResource("y", true);
    print("failing body");
  } catch (e) {
    print("caught", e.constructor.name, e.message, e.suppressed && e.suppressed.message);
  }
}
async function loop() {
  for (await using r of [asyncResource("r1"), asyncResource("r2")]) {
    print("iteration", await later(1));
  }
}
async function early() {
  await using e = asyncResource("e");
  return "early " + (await later("ret"));
}
async function main() {
  await block();
  await failing();
  await loop();
  print(await early());
  const it = gen();
  const seen = [it.next().value];
  await later("gen");
  seen.push(JSON.stringify(it.return("stop")));
  print("gen", seen.join(" "));
}
$dump.p1 = $djs.resume(23, "later", ["e"]);
var gen = function* gen() {
  using g = resource("g");
  yield 1;
  yield 2;
};
$dump.s2 = (() => {
  let name = "e";
  let fail = undefined;
  return { $f3: $djs.fn({ async [Symbol.asyncDispose]() {
      const v = await later(name);
      print("async dispose", v);
      if (fail) throw new Error("dispose " + name + " failed");
    } }), $Ka4: async function () {

const v = (await $dump.p1);
print("async dispose", v);
if (fail) throw new Error("dispose " + name + " failed");
} };
})();
$dump.n3 = { [Symbol.asyncDispose]: $dump.s2.$f3 };
Object.defineProperty($dump.s2.$Ka4, "$djs", { value: { f: $dump.s2.$f3, segs: [27,44,1,49,73,26,76,106,57], scaf: [["site",48,32]] } });
$dump.a4 = $dump.s2.$Ka4.call($dump.n3);
$dump.Ka5 = async function () {
const $djs_U = $djs.U;
{

const $djs_t0 = $dump.a4;
{
await using $djs_t1 = $djs_U(undefined, () => $djs_t0, 4);
return "early ret";
}
}
};
Object.defineProperty($dump.Ka5, "$djs", { value: { f: early, at: "await-using#1", scaf: [["using",39,86],["site",106,154],["site",106,158]], cmap: [["completion.value@39",1,"early ret"]] } });
$dump.a5 = $dump.Ka5.call(undefined);
$dump.a6_0 = print;
$dump.Ka6 = async function () {
const $djs_R = $djs.R;

const $djs_t0 = $dump.a6_0;
$djs_R($djs_t0, undefined, [(await $dump.a5)]);
const it = gen();
const seen = [it.next().value];
await later("gen");
seen.push(JSON.stringify(it.return("stop")));
print("gen", seen.join(" "));
};
Object.defineProperty($dump.Ka6, "$djs", { value: { f: main, at: "await#4", segs: [120,102,17,138,122,31,170,156,19,190,178,45,236,226,29], scaf: [["k",0,72],["k",0,79],["k",0,88],["site",84,101],["tmp",0,100,"call.arg@84","array.element"],["k",0,99]], cmap: [["call.callee@78",0,"$djs_t0"],["call.this@78",1,undefined]] } });
$dump.a6 = $dump.Ka6.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
