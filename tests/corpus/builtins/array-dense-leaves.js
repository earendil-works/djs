// Dense-array leaves of indexOf/lastIndexOf/includes/slice/join/pop and the iterable
// consumers' fast steps (0080, D1801). Each case also runs where the leaf must decline.
const out = [];
const log = (...a) => out.push(a.map(String).join(" "));

// Search: dense, holes (through the prototype), NaN, -0, fromIndex with side effects.
{
  const a = [1, "2", NaN, -0, undefined, 1];
  log("idx", a.indexOf(1), a.indexOf(1, 1), a.indexOf("2"), a.indexOf(NaN), a.indexOf(0), a.indexOf(undefined));
  log("inc", a.includes(NaN), a.includes(0), a.includes(2), a.includes(undefined), a.includes(1, 6));
  log("last", a.lastIndexOf(1), a.lastIndexOf(1, 4), a.lastIndexOf(1, -2), a.lastIndexOf(9), a.lastIndexOf(1, -100));
  const h = [1, , 3];
  log("holes", h.indexOf(undefined), h.includes(undefined), h.lastIndexOf(undefined));
  Array.prototype[1] = "p";
  log("proto", h.indexOf("p"), h.includes("p"), h.lastIndexOf("p"), h.join("-"), h.slice().join("-"), h.slice().hasOwnProperty(1));
  delete Array.prototype[1];
  const b = [1, 2, 3, 4];
  const from = { valueOf() { b.length = 1; return 0; } };
  log("shrink", b.indexOf(3, from), [1, 2, 3, 4].includes(4, { valueOf() { return 1; } }));
  const c = [1, 2, 3];
  log("grow", c.lastIndexOf(3, { valueOf() { c.push(3); return 5; } }));
  log("strings", ["a", "b" + "c", "bc"].indexOf("b" + "c"), [1n, 2n].includes(2n), [{}].indexOf({}));
  log("arraylike", Array.prototype.indexOf.call({ length: 2, 0: "x", 1: "y" }, "y"), Array.prototype.includes.call("abc", "c"));
}

// slice: plain, negative, species, frozen, subclass, holes.
{
  const a = [1, 2, 3, 4, 5];
  log("slice", a.slice(1, 3), a.slice(-2), a.slice(3, 1).length, a.slice(), a.slice(0, 100).length);
  class MyArray extends Array {}
  const m = MyArray.from([1, 2, 3]);
  const s = m.slice(1);
  log("species", s instanceof MyArray, s.length, s.join());
  const f = Object.freeze([1, 2]);
  log("frozen", f.slice().join(), Object.isFrozen(f.slice()));
  const weird = [1, 2, 3];
  weird.constructor = { [Symbol.species]: function (n) { const r = Object.preventExtensions([]); return r; } };
  try { weird.slice(); } catch (e) { log("nonext", e.constructor.name); }
  const sp = [1, 2, 3];
  sp.constructor = { [Symbol.species]: function (n) { return Object.defineProperty([], 0, { value: 9, writable: false, configurable: false }); } };
  try { log("nonconf", sp.slice().join()); } catch (e) { log("nonconf", e.constructor.name); }
  const big = [];
  for (let i = 0; i < 1000; i++) big.push(i);
  log("big", big.slice(10, 20).join(), big.slice(-3));
}

// join: numbers, strings, null/undefined, bools, floats, bigint, objects, symbols.
{
  log("join", [1, -2, 3.5, 1e21, -0, NaN, "x", null, undefined, true, 10n].join("|"));
  log("join2", ["é", "😀", 1].join(""), [].join(), [1].join(), [, 1].join(), ["a", "b"].join(undefined));
  log("join3", [{ toString() { return "T"; } }, 1].join(), [[1, 2], 3].join(";"));
  try { [Symbol("s")].join(); } catch (e) { log("sym", e.constructor.name); }
}

// pop: dense, holes, frozen, non-writable length, proto index, array-like, getters.
{
  const a = [1, 2, 3];
  log("pop", a.pop(), a.length, a.join());
  const h = [1, , ];
  log("pophole", h.length, h.pop(), h.length);
  Array.prototype[1] = "P";
  const h2 = [1, , 3];
  h2.pop();
  log("pophole2", h2.pop(), h2.length);
  delete Array.prototype[1];
  try { Object.freeze([1, 2]).pop(); } catch (e) { log("popfrozen", e.constructor.name); }
  const ro = [1, 2];
  Object.defineProperty(ro, "length", { writable: false });
  try { ro.pop(); } catch (e) { log("popro", e.constructor.name, ro.length, ro[1]); }
  const g = [1, 2];
  Object.defineProperty(g, 1, { get() { return "g"; }, configurable: true });
  log("popget", g.pop(), g.length);
  const al = { length: 2, 0: "a", 1: "b" };
  log("poplike", Array.prototype.pop.call(al), al.length, 1 in al);
  log("popempty", [].pop(), [undefined].pop());
  const stack = [];
  for (let i = 0; i < 100; i++) stack.push(i);
  let t = 0;
  while (stack.length) t += stack.pop();
  log("popsum", t);
}

// Iterable consumers: arrays (dense, holes), strings, typed arrays, entries.
{
  const h = [1, , 3];
  log("from", Array.from(h), Array.from([1, 2], (x) => x * 2), Array.from("a😀"), Array.from(new Uint8Array([7, 8])));
  log("ta", new Float64Array([1, 2.5]).join(), new Int8Array(h).join(), new Uint8Array("12").join(), Uint8Array.from([1, 2]).join());
  log("set", [...new Set([1, 2, 2, 3])], [...new Set("abca")], new Set(h).has(undefined));
  log("map", [...new Map([[1, "a"], [2, "b"]]).entries()].join(";"), new Map([["k", 1]].values()).get("k"));
  log("fromEntries", JSON.stringify(Object.fromEntries([["a", 1], ["b", 2]])));
  log("groupBy", JSON.stringify(Object.groupBy([1, 2, 3], (x) => (x % 2 ? "o" : "e"))), Map.groupBy([1, 2], (x) => x).size);
  const [first, ...rest] = [1, 2, 3];
  log("rest", first, rest, [...h], [..."ab"]);
  const ws = new WeakSet([{}, {}]);
  log("weak", ws instanceof WeakSet);
  // next replaced: the consumers must call it.
  const proto = Object.getPrototypeOf([][Symbol.iterator]());
  const next = proto.next;
  let calls = 0;
  proto.next = function () { calls++; return next.call(this); };
  Array.from([1, 2]); new Set([1]); new Map([[1, 1]]); new Int8Array([1]); Object.fromEntries([]);
  const [x, ...y] = [1, 2];
  proto.next = next;
  log("calls", calls);
  // An adder that mutates the source while it is iterated.
  const src = [1, 2, 3];
  const S = class extends Set { add(v) { if (v === 1) src.push(4); if (v === 2) src.length = 3; return super.add(v); } };
  log("mutate", [...new S(src)]);
}

// Object.values / entries: plain data in one step; accessors and exotics in order.
{
  const s = Symbol("s");
  const o = { b: 1, a: 2, 1: "one", 0: "zero", [s]: 3 };
  Object.defineProperty(o, "hidden", { value: 4, enumerable: false });
  Object.defineProperty(o, "hget", { get() { throw new Error("no"); }, enumerable: false });
  log("values", Object.values(o), JSON.stringify(Object.entries(o)));
  const g = { x: 1, get y() { delete this.z; return 2; }, z: 3 };
  log("getter", JSON.stringify(Object.entries(g)), Object.values({ get q() { return "Q"; } }));
  log("arrays", Object.values([1, , 3]), JSON.stringify(Object.entries(["a"])));
  log("ta", Object.values(new Int8Array([5, 6])), JSON.stringify(Object.entries(new Float32Array([0.5]))));
  function f() {}
  f.k = "v";
  log("fn", Object.values(f), Object.values("ab"), Object.values(new String("xy")), Object.entries(3).length);
  const p = new Proxy({ a: 1 }, { get(t, k) { return "trap-" + String(k); } });
  log("proxy", Object.values(p));
  const e = new Error("m");
  e.extra = 1;
  log("error", Object.values(e));
  (function () { log("arguments", Object.values(arguments)); })(1, 2);
}

// Promise.all over an array.
Promise.all([1, Promise.resolve(2), 3]).then((v) => {
  log("all", v);
  console.log(out.join("\n"));
});
