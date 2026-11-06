// The construct path without `step` and the constructors' `new` entries (0236): plain
// functions, base and derived classes, super(...args), new.target, the class checks,
// and Array/Map/Set/Date with arguments the entries answer and ones they decline.
const out = [];
const log = (...a) => out.push(a.map(String).join(" "));
{
  function F(a, b) { this.s = a + b; this.t = new.target === F; }
  log("F", new F(1, 2).s, new F(1, 2).t, new F().s, F.prototype.isPrototypeOf(new F()));
  function R() { return { r: 1 }; }
  function P() { return 5; }
  log("ret", new R().r, new R() instanceof R, new P() instanceof P);
  const g = function () {};
  g.prototype = 7;
  log("proto", Object.getPrototypeOf(new g()) === Object.prototype);
  Object.defineProperty(g, "prototype", { value: Array.prototype });
  log("proto2", Array.isArray(new g()), new g() instanceof Array);
  class A { constructor(x = 3) { this.x = x; } m() { return this.x * 2; } }
  class B extends A { constructor(...xs) { super(...xs); this.y = new.target.name; } }
  class C extends B {}
  class D extends A { constructor() { super(4); return undefined; } }
  class E extends A { constructor() { return { e: 1 }; } }
  class G extends A { constructor(flag) { if (flag) super(1); } }
  log("cls", new A().x, new A(5).m(), new B(6).y, new C(7).x, new C(7).y, new C().x, new D().x, new E().e);
  for (const [name, f] of [["G", () => new G(false)], ["call", () => A()], ["twice", () => new (class extends A { constructor() { super(); super(); } })()], ["ret", () => new (class extends A { constructor() { super(); return 1; } })()]]) {
    try { f(); log(name, "no error"); } catch (e) { log(name, e.constructor.name, e.message); }
  }
  log("G1", new G(true).x);
  const bound = F.bind(null, "a");
  log("bound", new bound("b").s, new bound("b") instanceof F);
  log("reflect", Reflect.construct(A, [8], B).x, Reflect.construct(A, [8], B) instanceof B, Reflect.construct(F, [1, 1], A).t);
  const px = new Proxy(A, {});
  log("proxy", new px(9).x, new px(9) instanceof A);
}
{
  const arr = (a) => `${a.length}:${a.join("|")}:${Object.keys(a).length}`;
  log("Array", arr(new Array()), arr(new Array(3)), arr(new Array(0)), arr(new Array("3")), arr(new Array(1, 2)), arr(new Array(undefined)), arr(Array(2)), arr(Array(1, 2, 3, 4, 5, 6, 7, 8)));
  for (const n of [-1, 1.5, 2 ** 32, NaN]) {
    try { new Array(n); log("len", n, "no error"); } catch (e) { log("len", n, e.constructor.name); }
  }
  class MyArray extends Array {}
  log("MyArray", new MyArray(2).length, new MyArray(2) instanceof MyArray, new MyArray(1, 2).length);
  const m = new Map(), s = new Set(), mn = new Map(null), su = new Set(undefined);
  log("Map", m.size, s.size, mn.size, su.size, new Map([[1, 2]]).get(1), new Set("abca").size, Object.getPrototypeOf(m) === Map.prototype);
  for (const [name, f] of [["Map()", () => Map()], ["Set()", () => Set()], ["Map(1)", () => new Map(1)]]) {
    try { f(); log(name, "no error"); } catch (e) { log(name, e.constructor.name); }
  }
  class MyMap extends Map { constructor() { super(); this.k = 1; } }
  log("MyMap", new MyMap().k, new MyMap() instanceof MyMap, new MyMap().size);
  const d = new Date(0), d2 = new Date(d), d3 = new Date("2020-01-02T03:04:05Z");
  log("Date", d.getTime(), d2.getTime(), d3.getTime(), new Date(1.7).getTime(), new Date(-0).getTime(), new Date(8.64e15 + 1).getTime(), new Date(null).getTime(), new Date(true).getTime(), new Date(undefined).getTime());
  log("Date2", new Date("x").getTime(), new Date({ valueOf() { return 5; } }).getTime(), new Date(2020, 1).getUTCMonth() >= 0, typeof Date(), typeof new Date().getTime());
  for (const v of [Symbol(), 1n]) {
    try { new Date(v); log("Date", typeof v, "no error"); } catch (e) { log("Date", typeof v, e.constructor.name); }
  }
}
console.log(out.join("\n"));
