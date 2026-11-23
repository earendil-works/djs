"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 17);
globalThis.$dump = {};
$dump.b0 = Object.getPrototypeOf([][Symbol.iterator]())["next"];
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
$dump.p1 = $djs.resume(16, "later", ["r1"]);
var gen = function* gen() {
  using g = resource("g");
  yield 1;
  yield 2;
};
$dump.s2 = (() => {
  let name = "r1";
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
$dump.s5 = (() => {
  let name = "r2";
  let fail = undefined;
  return { $f20: $djs.fn({ async [Symbol.asyncDispose]() {
      const v = await later(name);
      print("async dispose", v);
      if (fail) throw new Error("dispose " + name + " failed");
    } }) };
})();
$dump.i6 = $djs.advance(Array.prototype.values.call([$dump.n3, { [Symbol.asyncDispose]: $dump.s5.$f20 }]), 1);
$dump.Ka7 = async function () {
const $djs_L = $djs.L, $djs_U = $djs.U;

{ let $djs_f1 = true; for (const $djs_v1 of $djs_L($dump.i6, $dump.b0)) { if ($djs_f1) { $djs_f1 = false; {

{

const $djs_t0 = $dump.a4;
{
await using $djs_t1 = $djs_U(undefined, () => $djs_t0, 4);

}
}
} } else { await using r = $djs_v1; {
    print("iteration", await later(1));
  } } } }
};
Object.defineProperty($dump.Ka7, "$djs", { value: { f: loop, at: "await-using#1", segs: [276,31,13,301,92,45], scaf: [["for-of",26,83],["using",43,213],["site",137,262],["site",137,266]] } });
$dump.a7 = $dump.Ka7.call(undefined);
$dump.Ka8 = async function () {

(await $dump.a7);
print(await early());
const it = gen();
const seen = [it.next().value];
await later("gen");
seen.push(JSON.stringify(it.return("stop")));
print("gen", seen.join(" "));
};
Object.defineProperty($dump.Ka8, "$djs", { value: { f: main, at: "await#3", segs: [39,78,21,61,102,17,79,122,31,111,156,19,131,178,45,177,226,29], scaf: [["site",62,22]] } });
$dump.a8 = $dump.Ka8.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
