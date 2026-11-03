"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(14, "later", [1]);
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
  let name = "r1";
  let fail = undefined;
  return { $f9: $dump.fn({ async [Symbol.asyncDispose]() {
      const v = await later(name);
      print("async dispose", v);
      if (fail) throw new Error("dispose " + name + " failed");
    } }) };
})();
$dump.n2 = { [Symbol.asyncDispose]: $dump.s3.$f9 };
$dump.s4 = (() => {
  let name = "r2";
  let fail = undefined;
  return { $f18: $dump.fn({ async [Symbol.asyncDispose]() {
      const v = await later(name);
      print("async dispose", v);
      if (fail) throw new Error("dispose " + name + " failed");
    } }) };
})();
$dump.i5 = $dump.advance(Array.prototype.values.call([$dump.n2, { [Symbol.asyncDispose]: $dump.s4.$f18 }]), 1);
$dump.u6 = $dump.using([$dump.n2, $dump.s3.$f9, 4]);
$dump.a7 = $dump.resume(loop, { at: "await#1", awaiting: $dump.p1, bindings: { r: $dump.n2 }, operands: [["using.record", $dump.u6], ["call.callee", print], ["call.this", undefined], ["call.arg[0]", "iteration"]], loops: [{ kind: "for-of", iterator: $dump.i5, next: $dump.builtin("%ArrayIteratorPrototype%.next") }] });
$dump.a8 = $dump.resume(main, { at: "await#3", awaiting: $dump.a7, bindings: { it: undefined, seen: undefined } });
