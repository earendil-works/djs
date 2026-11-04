"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(1, "later", ["body"]);
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
var gen = function* gen() {
  using g = resource("g");
  yield 1;
  yield 2;
};
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
$dump.s3 = (() => {
  let name = "a";
  return { $f10: $dump.fn({ [Symbol.dispose]() {
      print("dispose", name);
    } }) };
})();
$dump.n2 = { [Symbol.dispose]: $dump.s3.$f10 };
$dump.s5 = (() => {
  let name = "b";
  let fail = undefined;
  return { $f11: $dump.fn({ async [Symbol.asyncDispose]() {
      const v = await later(name);
      print("async dispose", v);
      if (fail) throw new Error("dispose " + name + " failed");
    } }) };
})();
$dump.n4 = { [Symbol.asyncDispose]: $dump.s5.$f11 };
$dump.s7 = (() => {
  let name = "c";
  return { $f12: $dump.fn({ [Symbol.dispose]() {
      print("dispose", name);
    } }) };
})();
$dump.n6 = { [Symbol.dispose]: $dump.s7.$f12 };
$dump.u8 = $dump.using([$dump.n2, $dump.s3.$f10, 0, $dump.n4, $dump.s5.$f11, 4, undefined, undefined, 3, $dump.n6, $dump.s7.$f12, 0]);
$dump.a9 = $dump.resume(block, { at: "await#1", awaiting: $dump.p1, slots: { a: $dump.n2, b: $dump.n4, n: null, c: $dump.n6, "using.record@39": $dump.u8, "call.callee@157": print, "call.this@157": undefined, "call.arg@163": "in block" } });
$dump.a10 = $dump.resume(main, { at: "await#1", awaiting: $dump.a9, slots: { it: undefined, seen: undefined } });
