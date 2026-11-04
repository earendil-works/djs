"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(9, "later", ["y"]);
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
  let name = "y";
  let fail = true;
  return { $f3: $dump.fn({ async [Symbol.asyncDispose]() {
      const v = await later(name);
      print("async dispose", v);
      if (fail) throw new Error("dispose " + name + " failed");
    } }) };
})();
$dump.n3 = { [Symbol.asyncDispose]: $dump.s2.$f3 };
$dump.a4 = $dump.resume($dump.s2.$f3, { at: "await#1", awaiting: $dump.p1, this: $dump.n3, slots: { v: undefined } });
$dump.s7 = (() => {
  let name = "x";
  let fail = true;
  return { $f14: $dump.fn({ async [Symbol.asyncDispose]() {
      const v = await later(name);
      print("async dispose", v);
      if (fail) throw new Error("dispose " + name + " failed");
    } }) };
})();
$dump.n6 = { [Symbol.asyncDispose]: $dump.s7.$f14 };
$dump.n5 = [$dump.n6, $dump.s7.$f14, 4, $dump.n3, $dump.s2.$f3, 4];
$dump.u8 = $dump.using($dump.n5, { hasAwaited: true, next: 0, value: $dump.a4 });
$dump.a9 = $dump.resume(failing, { at: "await-using#1", awaiting: $dump.a4, slots: { "using.record@51": $dump.u8, x: $dump.n6, y: $dump.n3, "finally.value@157": undefined, "finally.return@157": 0 } });
$dump.a10 = $dump.resume(main, { at: "await#2", awaiting: $dump.a9, slots: { it: undefined, seen: undefined } });
