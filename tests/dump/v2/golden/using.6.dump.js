"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(16, "later", ["r1"]);
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
  let name = "r1";
  let fail = undefined;
  return { $f3: $dump.fn({ async [Symbol.asyncDispose]() {
      const v = await later(name);
      print("async dispose", v);
      if (fail) throw new Error("dispose " + name + " failed");
    } }) };
})();
$dump.n3 = { [Symbol.asyncDispose]: $dump.s2.$f3 };
$dump.a4 = $dump.resume($dump.s2.$f3, { at: "await#1", awaiting: $dump.p1, this: $dump.n3, slots: { v: undefined } });
$dump.s5 = (() => {
  let name = "r2";
  let fail = undefined;
  return { $f20: $dump.fn({ async [Symbol.asyncDispose]() {
      const v = await later(name);
      print("async dispose", v);
      if (fail) throw new Error("dispose " + name + " failed");
    } }) };
})();
$dump.i6 = $dump.advance(Array.prototype.values.call([$dump.n3, { [Symbol.asyncDispose]: $dump.s5.$f20 }]), 1);
$dump.u7 = $dump.using([$dump.n3, $dump.s2.$f3, 4], { hasAwaited: true, next: -3, value: $dump.a4 });
$dump.a8 = $dump.resume(loop, { at: "await-using#1", awaiting: $dump.a4, slots: { r: $dump.n3, "for-of.iterator@26": $dump.i6, "for-of.next@26": $dump.builtin("%ArrayIteratorPrototype%.next"), "using.record@43": $dump.u7, "completion.kind@43": 0, "completion.value@43": undefined } });
$dump.a9 = $dump.resume(main, { at: "await#3", awaiting: $dump.a8, slots: { it: undefined, seen: undefined } });
