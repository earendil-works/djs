"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 2);
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
$dump.p1 = $djs.resume(1, "later", ["body"]);
var gen = function* gen() {
  using g = resource("g");
  yield 1;
  yield 2;
};
$dump.s3 = (() => {
  let name = "a";
  return { $f10: $djs.fn({ [Symbol.dispose]() {
      print("dispose", name);
    } }) };
})();
$dump.n2 = { [Symbol.dispose]: $dump.s3.$f10 };
$dump.s5 = (() => {
  let name = "b";
  let fail = undefined;
  return { $f11: $djs.fn({ async [Symbol.asyncDispose]() {
      const v = await later(name);
      print("async dispose", v);
      if (fail) throw new Error("dispose " + name + " failed");
    } }) };
})();
$dump.n4 = { [Symbol.asyncDispose]: $dump.s5.$f11 };
$dump.s7 = (() => {
  let name = "c";
  return { $f12: $djs.fn({ [Symbol.dispose]() {
      print("dispose", name);
    } }) };
})();
$dump.n6 = { [Symbol.dispose]: $dump.s7.$f12 };
$dump.a8_0 = print;
$dump.Ka8 = async function () {
const $djs_R = $djs.R, $djs_U = $djs.U;

{
const a = $dump.n2;
const b = $dump.n4;
const n = null;
const c = $dump.n6;
using $djs_t0 = $djs_U($dump.n2, $dump.s3.$f10, 0);
await using $djs_t1 = $djs_U($dump.n4, $dump.s5.$f11, 4);
await using $djs_t2 = null;
using $djs_t3 = $djs_U($dump.n6, $dump.s7.$f12, 0);
const $djs_t4 = $dump.a8_0;
$djs_R($djs_t4, undefined, ["in block", (await $dump.p1)]);
}
print("after block");
};
Object.defineProperty($dump.Ka8, "$djs", { value: { f: block, segs: [419,203,21], scaf: [["using",39,145],["k",0,357],["k",0,364],["k",0,373],["tmp",0,385,"call.arg@163","array.element"],["site",175,398],["tmp",0,397,"call.arg@175","array.element"],["k",0,384],["site",200,418]], cmap: [["call.callee@157",0,"$djs_t4"],["call.this@157",1,undefined],["call.arg@163",1,"in block"]] } });
$dump.a8 = $dump.Ka8.call(undefined);
$dump.Ka9 = async function () {

(await $dump.a8);
await failing();
await loop();
print(await early());
const it = gen();
const seen = [it.next().value];
await later("gen");
seen.push(JSON.stringify(it.return("stop")));
print("gen", seen.join(" "));
};
Object.defineProperty($dump.Ka9, "$djs", { value: { f: main, segs: [39,43,16,56,62,13,70,78,21,92,102,17,110,122,31,142,156,19,162,178,45,208,226,29], scaf: [["site",26,22]] } });
$dump.a9 = $dump.Ka9.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
