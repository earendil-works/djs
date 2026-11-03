"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(22, "later", ["ret"]);
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
  let name = "e";
  let fail = undefined;
  return { $f8: $dump.fn({ async [Symbol.asyncDispose]() {
      const v = await later(name);
      print("async dispose", v);
      if (fail) throw new Error("dispose " + name + " failed");
    } }) };
})();
$dump.n2 = { [Symbol.asyncDispose]: $dump.s3.$f8 };
$dump.u4 = $dump.using([$dump.n2, $dump.s3.$f8, 4]);
$dump.a5 = $dump.resume(early, { at: "await#1", awaiting: $dump.p1, bindings: { e: $dump.n2 }, operands: [["using.record", $dump.u4], ["binary.left", "early "]] });
$dump.a6 = $dump.resume(main, { at: "await#4", awaiting: $dump.a5, bindings: { it: undefined, seen: undefined }, operands: [["call.callee", print], ["call.this", undefined]] });
