// Proxy (0045): every trap with a logging handler, the invariant checks, revocation,
// proxies as prototypes and handlers, callable and constructible proxies, and the
// builtins that reach internal methods.
// difftest: no-stress 3 s suspended, 8 s under GC stress (debug, 0053)
function t(label, f) { try { console.log(label, f()); } catch (e) { console.log(label, String(e)); } }
const TRAPS = ["get", "set", "has", "deleteProperty", "ownKeys", "getOwnPropertyDescriptor", "defineProperty", "getPrototypeOf", "setPrototypeOf", "isExtensible", "preventExtensions", "apply", "construct"];
let log = [];
const key = (k) => (typeof k === "symbol" ? k.toString() : String(k));
function logger(name, override = {}) {
  const h = {};
  for (const trap of TRAPS) {
    h[trap] = function (target, ...rest) {
      log.push(name + "." + trap + (typeof rest[0] === "string" || typeof rest[0] === "symbol" ? ":" + key(rest[0]) : ""));
      return trap in override ? override[trap](target, ...rest) : Reflect[trap](target, ...rest);
    };
  }
  return h;
}
function L(label, f) {
  log = [];
  let r;
  try { r = f(); } catch (e) { r = String(e); }
  console.log(label, JSON.stringify(r) ?? String(r), "|", log.join(" "));
}

const target = { a: 1, b: [1, 2], [Symbol.toStringTag]: "T" };
const p = new Proxy(target, logger("p"));
L("get", () => p.a);
L("get elem", () => p.b[1]);
L("set", () => { p.c = 3; return target.c; });
L("set index", () => { p[5] = 1; return target[5]; });
L("has", () => ["a" in p, "zz" in p, 0 in p]);
L("delete", () => [delete p.c, "c" in target]);
L("keys", () => Object.keys(p));
L("values", () => Object.values(p));
L("entries", () => Object.entries(p));
L("ownKeys", () => Reflect.ownKeys(p).map(key));
L("names", () => Object.getOwnPropertyNames(p));
L("gopd", () => Object.getOwnPropertyDescriptor(p, "a"));
L("define", () => Object.defineProperty(p, "d", { value: 4, enumerable: true }) === p);
L("proto", () => Object.getPrototypeOf(p) === Object.prototype);
L("setproto", () => Reflect.setPrototypeOf(p, Array.prototype));
L("isext", () => Object.isExtensible(p));
L("toString", () => Object.prototype.toString.call(p));
L("json", () => JSON.stringify(p));
L("spread", () => ({ ...p }));
L("assign", () => Object.assign({}, p));
L("for-in", () => { const ks = []; for (const k in p) ks.push(k); return ks; });
L("for-in delete", () => { const ks = []; for (const k in p) { ks.push(k); if (k === "a") delete target.d; } target.d = 4; return ks; });
L("hasOwn", () => [Object.hasOwn(p, "a"), p.hasOwnProperty("zz")]);
// (Object.isFrozen/isSealed: qjs asks isExtensible after the keys, the spec first.)
const fq = new Proxy({ x: 1 }, logger("q"));
L("freeze", () => Object.freeze(fq) === fq);
t("isFrozen", () => [Object.isFrozen(fq), Object.isSealed(fq), Object.isFrozen(new Proxy({}, {}))]);
const sq = new Proxy({ x: 1 }, logger("q"));
L("seal", () => Object.seal(sq) === sq);
t("isSealed", () => [Object.isFrozen(sq), Object.isSealed(sq)]);
L("prevent", () => { const q = new Proxy({}, logger("q")); return [Object.preventExtensions(q) === q, Object.isExtensible(q)]; });

// Callable and constructible proxies.
function F(x) { this.x = x; }
F.prototype.hi = function () { return "hi " + this.x; };
const pf = new Proxy(F, logger("f"));
L("call", () => typeof pf);
L("construct", () => new pf(7).hi());
L("apply trap", () => new Proxy(function () { return 1; }, { apply: (t, self, args) => [self === undefined, args] })(1, 2));
L("construct trap", () => new (new Proxy(function () {}, { construct: (t, args, nt) => ({ args, same: nt !== t }) }))(5));
L("method call", () => { const o = { m: new Proxy(function () { return this === o; }, {}) }; return o.m(); });
L("bind", () => { const b = Function.prototype.bind.call(pf, null, 1); return [b.name, b.length, new b().x]; });
L("instanceof", () => [new F(1) instanceof pf, {} instanceof pf]);
L("instanceof chain", () => Object.create(new Proxy(F.prototype, logger("c"))) instanceof F);
L("isArray", () => [Array.isArray(new Proxy([], {})), Array.isArray(new Proxy({}, {})), Array.isArray(new Proxy(new Proxy([], {}), {}))]);
L("array ops", () => { const a = new Proxy([3, 1, 2], logger("a")); return [a.length, a.slice(1), Array.prototype.concat.call([], a).length]; });
L("array push", () => { const arr = [1]; const a = new Proxy(arr, logger("a")); a.push(2); return arr; });
L("not ctor", () => new (new Proxy(() => 1, {}))());
L("not callable", () => new Proxy({}, {})());
L("new.target", () => { let seen; function G() { seen = new.target; } const pg = new Proxy(G, {}); new pg(); return seen === pg; });
L("ctor proto via get", () => { function H() {} const ph = new Proxy(H, logger("h")); return Object.getPrototypeOf(Reflect.construct(function () {}, [], ph)) === H.prototype; });

// Proxies as prototypes, handlers and targets.
const chained = Object.create(new Proxy({ inherited: 1 }, logger("proto")));
L("proto get", () => [chained.inherited, chained.missing]);
L("proto set", () => { chained.z = 1; return Object.keys(chained); });
L("proto has", () => "inherited" in chained);
// (The keys only: for a proxy further up the chain qjs reads its keys twice.)
t("proto for-in", () => { const ks = []; for (const k in chained) ks.push(k); return ks; });
L("handler proxy", () => { const hp = new Proxy({}, logger("handler")); const q = new Proxy({ v: 1 }, hp); return [q.v, "v" in q]; });
L("nested", () => { const inner = new Proxy({ n: 1 }, logger("inner")); const outer = new Proxy(inner, logger("outer")); return [outer.n, Object.keys(outer)]; });
L("primitive proto", () => { const saved = Object.getPrototypeOf(Number.prototype); Object.setPrototypeOf(Number.prototype, new Proxy({}, logger("num"))); try { const r = (5).foo; (5).bar = 1; return r; } finally { Object.setPrototypeOf(Number.prototype, saved); } });

// Revocation.
const r = Proxy.revocable({ a: 1 }, {});
L("revocable", () => [r.proxy.a, Object.keys(r), typeof r.revoke, r.revoke.length, r.revoke.name]);
r.revoke();
r.revoke();
for (const [label, f] of [["get", () => r.proxy.a], ["set", () => { r.proxy.a = 1; }], ["has", () => "a" in r.proxy], ["keys", () => Object.keys(r.proxy)], ["typeof", () => typeof r.proxy], ["toString", () => Object.prototype.toString.call(r.proxy)], ["isArray", () => Array.isArray(r.proxy)], ["proto", () => Object.getPrototypeOf(r.proxy)], ["as proxy target", () => new Proxy(r.proxy, {})]]) {
  L("revoked " + label, f);
}

// Invariant checks.
const frozen = Object.freeze({ k: 1 });
const sealedFn = Object.preventExtensions({ k: 1 });
t("inv get", () => new Proxy(frozen, { get: () => 2 }).k);
t("inv get ok", () => new Proxy(frozen, { get: () => 1 }).k);
t("inv set", () => { new Proxy(frozen, { set: () => true }).k = 2; });
t("inv set false", () => { new Proxy({}, { set: () => false }).k = 2; });
t("inv has", () => "k" in new Proxy(frozen, { has: () => false }));
t("inv has ext", () => "k" in new Proxy(sealedFn, { has: () => false }));
t("inv delete", () => delete new Proxy(frozen, { deleteProperty: () => true }).k);
t("inv delete false", () => delete new Proxy({}, { deleteProperty: () => false }).k);
t("inv ownKeys dup", () => Object.keys(new Proxy({}, { ownKeys: () => ["a", "a"] })));
t("inv ownKeys type", () => Object.keys(new Proxy({}, { ownKeys: () => [1] })));
t("inv ownKeys obj", () => Object.keys(new Proxy({}, { ownKeys: () => 1 })));
t("inv ownKeys missing", () => Object.keys(new Proxy(frozen, { ownKeys: () => [] })));
t("inv ownKeys extra", () => Object.keys(new Proxy(sealedFn, { ownKeys: () => ["k", "x"] })));
t("inv gopd", () => Object.getOwnPropertyDescriptor(new Proxy({}, { getOwnPropertyDescriptor: () => ({ value: 1, configurable: false }) }), "x"));
t("inv gopd undefined", () => Object.getOwnPropertyDescriptor(new Proxy(frozen, { getOwnPropertyDescriptor: () => undefined }), "k"));
t("inv define", () => Object.defineProperty(new Proxy(sealedFn, { defineProperty: () => true }), "x", { value: 1 }));
t("inv define false", () => Object.defineProperty(new Proxy({}, { defineProperty: () => false }), "x", { value: 1 }));
t("inv define reflect", () => Reflect.defineProperty(new Proxy({}, { defineProperty: () => false }), "x", { value: 1 }));
t("inv proto", () => Object.getPrototypeOf(new Proxy(frozen, { getPrototypeOf: () => Array.prototype })));
t("inv proto type", () => Object.getPrototypeOf(new Proxy({}, { getPrototypeOf: () => 1 })));
t("inv isExtensible", () => Object.isExtensible(new Proxy({}, { isExtensible: () => false })));
t("inv preventExtensions", () => Object.preventExtensions(new Proxy({}, { preventExtensions: () => true })));
t("inv preventExtensions false", () => Object.preventExtensions(new Proxy({}, { preventExtensions: () => false })));
t("inv setPrototypeOf false", () => Object.setPrototypeOf(new Proxy({}, { setPrototypeOf: () => false }), null));
t("inv construct", () => new (new Proxy(function () {}, { construct: () => 1 }))());
t("trap not callable", () => new Proxy({}, { get: 1 }).x);
t("trap null", () => new Proxy({ x: 5 }, { get: null }).x);

// The constructor.
t("call", () => Proxy({}, {}));
t("bad target", () => new Proxy(1, {}));
t("bad handler", () => new Proxy({}, null));
t("statics", () => [Proxy.length, Proxy.name, "prototype" in Proxy, Object.getOwnPropertyNames(Proxy).sort()]);
