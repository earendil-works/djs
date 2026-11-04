"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 15);
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
$dump.p1 = $djs.resume(14, "later", [1]);
var gen = function* gen() {
  using g = resource("g");
  yield 1;
  yield 2;
};
$dump.s3 = (() => {
  let name = "r1";
  let fail = undefined;
  return { $f9: $djs.fn({ async [Symbol.asyncDispose]() {
      const v = await later(name);
      print("async dispose", v);
      if (fail) throw new Error("dispose " + name + " failed");
    } }) };
})();
$dump.n2 = { [Symbol.asyncDispose]: $dump.s3.$f9 };
$dump.s4 = (() => {
  let name = "r2";
  let fail = undefined;
  return { $f18: $djs.fn({ async [Symbol.asyncDispose]() {
      const v = await later(name);
      print("async dispose", v);
      if (fail) throw new Error("dispose " + name + " failed");
    } }) };
})();
$dump.i5 = $djs.advance(Array.prototype.values.call([$dump.n2, { [Symbol.asyncDispose]: $dump.s4.$f18 }]), 1);
$dump.a6_0 = print;
$dump.Ka6 = async function () {
const $djs_R = $djs.R, $djs_L = $djs.L, $djs_U = $djs.U;

{ let $djs_f1 = true; for (const $djs_v1 of $djs_L($dump.i5, $dump.b0)) { if ($djs_f1) { $djs_f1 = false; {
const r = $dump.n2;
await using $djs_t0 = $djs_U($dump.n2, $dump.s3.$f9, 4);

{

const $djs_t1 = $dump.a6_0;
$djs_R($djs_t1, undefined, ["iteration", (await $dump.p1)]);
}
} } else { await using r = $djs_v1; {
    print("iteration", await later(1));
  } } } }
};
Object.defineProperty($dump.Ka6, "$djs", { value: { f: loop, segs: [369,31,13,394,92,45], scaf: [["for-of",26,100],["using",43,218],["k",0,295],["k",0,302],["k",0,311],["tmp",0,323,"call.arg@104","array.element"],["site",117,337],["tmp",0,336,"call.arg@117","array.element"],["k",0,322],["site",137,357],["site",137,359]], cmap: [["call.callee@98",0,"$djs_t1"],["call.this@98",1,undefined],["call.arg@104",1,"iteration"]] } });
$dump.a6 = $dump.Ka6.call(undefined);
$dump.Ka7 = async function () {

(await $dump.a6);
print(await early());
const it = gen();
const seen = [it.next().value];
await later("gen");
seen.push(JSON.stringify(it.return("stop")));
print("gen", seen.join(" "));
};
Object.defineProperty($dump.Ka7, "$djs", { value: { f: main, at: "await#3", segs: [39,78,21,61,102,17,79,122,31,111,156,19,131,178,45,177,226,29], scaf: [["site",62,22]] } });
$dump.a7 = $dump.Ka7.call(undefined);
try { (undefined); } finally { delete globalThis.$dump; delete globalThis.$djs; }
