"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(4, "later", ["b"]);
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
$dump.s2 = (() => {
  let name = "b";
  let fail = undefined;
  return { $f3: $dump.fn({ async [Symbol.asyncDispose]() {
      const v = await later(name);
      print("async dispose", v);
      if (fail) throw new Error("dispose " + name + " failed");
    } }) };
})();
$dump.n3 = { [Symbol.asyncDispose]: $dump.s2.$f3 };
$dump.a4 = $dump.resume($dump.s2.$f3, { at: "await#1", awaiting: $dump.p1, this: $dump.n3, slots: { v: undefined } });
$dump.s7 = (() => {
  let name = "a";
  return { $f15: $dump.fn({ [Symbol.dispose]() {
      print("dispose", name);
    } }) };
})();
$dump.n6 = { [Symbol.dispose]: $dump.s7.$f15 };
$dump.s9 = (() => {
  let name = "c";
  return { $f16: $dump.fn({ [Symbol.dispose]() {
      print("dispose", name);
    } }) };
})();
$dump.n8 = { [Symbol.dispose]: $dump.s9.$f16 };
$dump.n5 = [$dump.n6, $dump.s7.$f15, 0, $dump.n3, $dump.s2.$f3, 4, undefined, undefined, 3, $dump.n8, $dump.s9.$f16, 0];
$dump.u10 = $dump.using($dump.n5, { needsAwait: true, hasAwaited: true, next: 0, value: $dump.a4 });
$dump.a11 = $dump.resume(block, { at: "await-using#1", awaiting: $dump.a4, slots: { "using.record@39": $dump.u10, a: $dump.n6, b: $dump.n3, n: null, c: $dump.n8, "finally.value@200": undefined, "finally.return@200": 0 } });
$dump.a12 = $dump.resume(main, { at: "await#1", awaiting: $dump.a11, slots: { it: undefined, seen: undefined } });
