"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(26, "later", ["gen"]);
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
var gen = function* gen() {
  using g = resource("g");
  yield 1;
  yield 2;
};
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
$dump.s3 = (() => {
  let name = "g";
  return { $f15: $dump.fn({ [Symbol.dispose]() {
      print("dispose", name);
    } }) };
})();
$dump.n2 = { [Symbol.dispose]: $dump.s3.$f15 };
$dump.u4 = $dump.using([$dump.n2, $dump.s3.$f15, 0]);
$dump.a5 = $dump.resume(gen, { at: "yield#1", slots: { g: $dump.n2, "using.record@26": $dump.u4 } });
$dump.a6 = $dump.resume(main, { at: "await#5", awaiting: $dump.p1, slots: { it: $dump.a5, seen: [1] } });
