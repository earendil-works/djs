"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 5);
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
$dump.p1 = $djs.resume(4, "later", ["b"]);
var gen = function* gen() {
  using g = resource("g");
  yield 1;
  yield 2;
};
$dump.s2 = (() => {
  let name = "b";
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
$dump.s6 = (() => {
  let name = "a";
  return { $f15: $djs.fn({ [Symbol.dispose]() {
      print("dispose", name);
    } }) };
})();
$dump.n5 = { [Symbol.dispose]: $dump.s6.$f15 };
$dump.s8 = (() => {
  let name = "c";
  return { $f16: $djs.fn({ [Symbol.dispose]() {
      print("dispose", name);
    } }) };
})();
$dump.n7 = { [Symbol.dispose]: $dump.s8.$f16 };
$dump.Ka9 = async function () {
const $djs_U = $djs.U;

{

const $djs_t0 = $dump.a4;
{
using $djs_t1 = $djs_U($dump.n5, $dump.s6.$f15, 0);
await using $djs_t2 = $djs_U(undefined, () => $djs_t0, 4);

}
}
print("after block");
};
Object.defineProperty($dump.Ka9, "$djs", { value: { f: block, at: "await-using#1", segs: [191,203,21], scaf: [["using",39,81],["site",200,188]] } });
$dump.a9 = $dump.Ka9.call(undefined);
$dump.Ka10 = async function () {

(await $dump.a9);
await failing();
await loop();
print(await early());
const it = gen();
const seen = [it.next().value];
await later("gen");
seen.push(JSON.stringify(it.return("stop")));
print("gen", seen.join(" "));
};
Object.defineProperty($dump.Ka10, "$djs", { value: { f: main, segs: [39,43,16,56,62,13,70,78,21,92,102,17,110,122,31,142,156,19,162,178,45,208,226,29], scaf: [["site",26,22]] } });
$dump.a10 = $dump.Ka10.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
