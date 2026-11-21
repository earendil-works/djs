// `{...src}` as the first part of a literal copies a plain source's values at once (0262):
// every other source takes the per-key path, and the results must be the same objects.
const keys = (o) => Reflect.ownKeys(o).map(String).join();
const show = (o) => JSON.stringify(Reflect.ownKeys(o).map((k) => [String(k), Object.getOwnPropertyDescriptor(o, k)]));
const s1 = Symbol("s1"), s2 = Symbol("s2");

// Plain sources: literals, JSON.parse, class instances, symbols among strings.
const src = { b: 1, [s1]: "x", a: 2, __proto__: { inherited: 1 } };
const c = { ...src };
console.log(keys(c), c.b, c.a, c[s1], "inherited" in c, Object.getPrototypeOf(c) === Object.prototype);
console.log(show({ ...JSON.parse('{"z":1,"y":[1,2],"x":{"w":null}}') }));
class P { constructor() { this.q = 1; this.p = 2; } m() {} }
console.log(keys({ ...new P() }), "m" in { ...new P() });
console.log(keys({ ...{ [s2]: 1, k: 2, [s1]: 3, j: 4 } }));
const own = JSON.parse('{"__proto__": 7, "v": 1}');
const withProto = { ...own };
console.log(keys(withProto), withProto.__proto__ === 7 ? "own __proto__" : "proto", Object.getPrototypeOf(withProto) === Object.prototype);

// The copy is independent of the source; later parts add to it, overwrite and delete.
const d = { ...src, a: 9, extra: true };
d.b = 5;
delete d.a;
console.log(keys(d), d.b, src.b, src.a, keys(src));
const e = { ...src, ...{ b: "second" }, b2: 0 };
console.log(keys(e), e.b);
const f = { __proto__: null, ...src };
console.log(Object.getPrototypeOf(f), keys(f));

// Sources that are not plain: non-enumerable, read-only or frozen properties, getters,
// index keys, arrays, strings, dictionaries, null-prototype objects, functions, primitives.
const ne = Object.defineProperty({ a: 1 }, "hidden", { value: 2, enumerable: false });
console.log(show({ ...ne }));
const ro = Object.defineProperty({ a: 1 }, "ro", { value: 2, enumerable: true, writable: false });
console.log(show({ ...ro }));
console.log(show({ ...Object.freeze({ fz: 1, fy: 2 }) }), show({ ...Object.seal({ se: 1 }) }));
const log = [];
console.log(show({ ...{ a: 1, get g() { log.push("get"); return 2; } } }), log.join());
console.log(keys({ ...{ b: 1, 2: "two", 1: "one", a: 0 } }), keys({ ...["x", "y"] }));
const dict = {};
for (let i = 0; i < 40; i++) dict["p" + i] = i;
delete dict.p3;
console.log(Object.keys({ ...dict }).length, { ...dict }.p39, "p3" in { ...dict });
const bare = Object.create(null);
bare.n = 1;
console.log(keys({ ...bare }), keys({ ...function fn() {} }), keys({ ...null, ...undefined, ...5, ...true }));
const nonext = Object.preventExtensions({ pe: 1 });
const g = { ...nonext };
g.more = 1;
console.log(keys(g), Object.isExtensible(g));
const empty = { ...{} };
empty.x = 1;
console.log(keys(empty), keys({ ...new Map([[1, 2]]) }), keys({ ...new Error("m") }));
