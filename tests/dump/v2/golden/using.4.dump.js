"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(11, "later", ["x"]);
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
  let name = "x";
  let fail = true;
  return { $f3: $dump.fn({ async [Symbol.asyncDispose]() {
      const v = await later(name);
      print("async dispose", v);
      if (fail) throw new Error("dispose " + name + " failed");
    } }) };
})();
$dump.n3 = { [Symbol.asyncDispose]: $dump.s2.$f3 };
$dump.a4 = $dump.resume($dump.s2.$f3, { at: "await#1", awaiting: $dump.p1, this: $dump.n3, bindings: { v: undefined } });
$dump.s6 = (() => {
  let name = "y";
  let fail = true;
  return { $f14: $dump.fn({ async [Symbol.asyncDispose]() {
      const v = await later(name);
      print("async dispose", v);
      if (fail) throw new Error("dispose " + name + " failed");
    } }) };
})();
$dump.n5 = { [Symbol.asyncDispose]: $dump.s6.$f14 };
$dump.n7 = new Error("dispose y failed");
$dump.u8 = $dump.using([$dump.n3, $dump.s2.$f3, 4, $dump.n5, $dump.s6.$f14, 4], { error: $dump.n7, hasError: true, hasAwaited: true, next: -3, value: $dump.a4 });
$dump.a9 = $dump.resume(failing, { at: "await-using#1", awaiting: $dump.a4, bindings: { x: $dump.n3, y: $dump.n5 }, operands: [["using.record", $dump.u8], ["finally.value", undefined], ["finally.return", 0], ["using.record", $dump.u8]] });
$dump.a10 = $dump.resume(main, { at: "await#2", awaiting: $dump.a9, bindings: { it: undefined, seen: undefined } });
Object.defineProperty($dump.n7, "stack", { value: "    at <anonymous> (src.js:17:32)\n", writable: true, configurable: true });
