// Reflect (0038).
function t(label, f) { try { console.log(label, f()); } catch (e) { console.log(label, String(e)); } }
function F(a, b) { this.a = a; this.nt = new.target === G; }
function G() {}
G.prototype.tag = "g";
t("construct", () => { const o = Reflect.construct(F, [1, 2], G); return [o.a, o.nt, o.tag, Object.getPrototypeOf(o) === G.prototype]; });
t("construct plain", () => Reflect.construct(F, [3]).a);
t("construct Error", () => { const e = Reflect.construct(Error, ["m"], G); return [e.message, Object.getPrototypeOf(e) === G.prototype]; });
t("construct Array", () => { const a = Reflect.construct(Array, [3], G); return [a.length, Array.isArray(a)]; });

t("construct bad nt", () => Reflect.construct(F, [], Math.max));
t("apply", () => Reflect.apply(Math.max, null, [1, 5, 3]));
t("apply bad", () => Reflect.apply(1, null, []));
const o = { x: 1, get g() { return this.y; }, set s(v) { this.seen = v; } };
t("get", () => [Reflect.get(o, "x"), Reflect.get(o, "g", { y: 9 }), Reflect.get([5], 0)]);
t("set", () => { const r = {}; return [Reflect.set(o, "s", 7, r), r.seen, Reflect.set(o, "z", 1), o.z, Reflect.set(Object.freeze({ q: 1 }), "q", 2), Reflect.set(o, "x", 3, r), r.x]; });
t("set receiver nonwritable", () => { const r = Object.defineProperty({}, "x", { value: 1 }); return Reflect.set({}, "x", 2, r); });
t("has", () => [Reflect.has(o, "x"), Reflect.has(o, "toString"), Reflect.has(o, "nope")]);
t("define", () => [Reflect.defineProperty(o, "d", { value: 1 }), Reflect.defineProperty(o, "d", { value: 2 }), Reflect.defineProperty([], "length", { value: 2 })]);
t("delete", () => [Reflect.deleteProperty(o, "x"), Reflect.deleteProperty(o, "d"), "x" in o]);
t("ownKeys", () => Reflect.ownKeys({ b: 1, a: 2, 1: 3, [Symbol.iterator]: 4 }).length);
t("proto", () => [Reflect.getPrototypeOf([]) === Array.prototype, Reflect.setPrototypeOf({}, null), Reflect.setPrototypeOf(Object.preventExtensions({}), {})]);
t("ext", () => { const x = {}; return [Reflect.isExtensible(x), Reflect.preventExtensions(x), Reflect.isExtensible(x)]; });
t("gopd", () => Reflect.getOwnPropertyDescriptor({ k: 1 }, "k").value);
t("non-object", () => Reflect.get(1, "x"));
t("tag", () => [String(Reflect), Object.keys(Reflect).length, Reflect.apply.length, Reflect.construct.length, Reflect.get.length, Reflect.set.length]);
