"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 27);
globalThis.$dump = {};
$dump.b0 = Object.getPrototypeOf(function* () {}).prototype;
function resource(name) {
  return {
    [Symbol.dispose]() {
      print("dispose", name);
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
function asyncResource(name, fail) {
  return {
    async [Symbol.asyncDispose]() {
      const v = await later(name);
      print("async dispose", v);
      if (fail) throw new Error("dispose " + name + " failed");
    },
  };
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
$dump.p1 = $djs.resume(26, "later", ["gen"]);
var gen = function* gen() {
  using g = resource("g");
  yield 1;
  yield 2;
};
$dump.s3 = (() => {
  let name = "g";
  return { $f16: $djs.fn({ [Symbol.dispose]() {
      print("dispose", name);
    } }) };
})();
$dump.n2 = { [Symbol.dispose]: $dump.s3.$f16 };
$dump.n5 = { __proto__: $dump.b0 };
$dump.Ka4 = function* () {
const $djs_U = $djs.U;
const g = $dump.n2;
using $djs_t0 = $djs_U($dump.n2, $dump.s3.$f16, 0);
(yield);
yield 2;
};
Object.defineProperty($dump.Ka4, "$djs", { value: { f: gen, at: "yield#1", segs: [119,58,8], scaf: [["using",26,64],["site",47,111],["site",68,129]] } });
$dump.Ka4.prototype = $dump.n5;
$dump.a4 = $dump.Ka4.call(undefined);
$dump.a4.next();
$dump.Ka6 = async function () {
const it = $dump.a4;
const seen = [1];
(await $dump.p1);
seen.push(JSON.stringify(it.return("stop")));
print("gen", seen.join(" "));
};
Object.defineProperty($dump.Ka6, "$djs", { value: { f: main, at: "await#5", segs: [77,178,45,123,226,29], scaf: [["site",156,60]] } });
$dump.a6 = $dump.Ka6.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
