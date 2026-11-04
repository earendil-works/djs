"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 10);
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
$dump.p1 = $djs.resume(9, "later", ["y"]);
var gen = function* gen() {
  using g = resource("g");
  yield 1;
  yield 2;
};
$dump.s2 = (() => {
  let name = "y";
  let fail = true;
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
  let name = "x";
  let fail = true;
  return { $f14: $djs.fn({ async [Symbol.asyncDispose]() {
      const v = await later(name);
      print("async dispose", v);
      if (fail) throw new Error("dispose " + name + " failed");
    } }) };
})();
$dump.n5 = { [Symbol.asyncDispose]: $dump.s6.$f14 };
$dump.Ka7 = async function () {
const $djs_U = $djs.U;

try {

const $djs_t0 = $dump.a4;
{
await using $djs_t1 = $djs_U($dump.n5, $dump.s6.$f14, 4);
await using $djs_t2 = $djs_U(undefined, () => $djs_t0, 4);

}
} catch (e) {
    print("caught", e.constructor.name, e.message, e.suppressed && e.suppressed.message);
  }
};
Object.defineProperty($dump.Ka7, "$djs", { value: { f: failing, at: "await-using#1", segs: [208,165,1,211,168,95], scaf: [["try",29,44],["using",51,91],["site",157,198]] } });
$dump.a7 = $dump.Ka7.call(undefined);
$dump.Ka8 = async function () {

(await $dump.a7);
await loop();
print(await early());
const it = gen();
const seen = [it.next().value];
await later("gen");
seen.push(JSON.stringify(it.return("stop")));
print("gen", seen.join(" "));
};
Object.defineProperty($dump.Ka8, "$djs", { value: { f: main, at: "await#2", segs: [39,62,13,53,78,21,75,102,17,93,122,31,125,156,19,145,178,45,191,226,29], scaf: [["site",43,22]] } });
$dump.a8 = $dump.Ka8.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
