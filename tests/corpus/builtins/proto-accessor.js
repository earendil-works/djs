// Object.prototype.__proto__ (Annex B.2.2.1) and Function.prototype.toString of proxies.
function t(label, f) { try { console.log(label, f()); } catch (e) { console.log(label, String(e)); } }
const d = Object.getOwnPropertyDescriptor(Object.prototype, "__proto__");
t("desc", () => [typeof d.get, typeof d.set, d.enumerable, d.configurable, d.get.name, d.set.name, d.get.length, d.set.length]);
t("get undefined", () => d.get.call(undefined));
t("get primitive", () => d.get.call(1) === Number.prototype);
t("set undefined", () => d.set.call(undefined, {}));
t("set primitive", () => d.set.call(1, {}));
t("set non-object", () => d.set.call({}, 1));
t("assign", () => { const o = {}; o.__proto__ = Array.prototype; return [Array.isArray(o), o instanceof Array, Object.keys(o)]; });
t("assign null", () => { const o = { a: 1 }; o.__proto__ = null; return [Object.getPrototypeOf(o), o.__proto__]; });
t("immutable", () => { Object.prototype.__proto__ = {}; });
t("not extensible", () => { const o = Object.preventExtensions({}); o.__proto__ = {}; });
t("cycle", () => { const o = {}; const p = Object.create(o); o.__proto__ = p; });
t("destructure", () => { const { __proto__: p } = []; return p === Array.prototype; });
t("literal", () => { const o = { __proto__: [] }; return [Array.isArray(o), o.__proto__ === Array.prototype, Object.keys(o)]; });
t("computed own", () => { const o = { ["__proto__"]: 1 }; return [Object.keys(o), o.__proto__, Object.getPrototypeOf(o) === Object.prototype]; });
t("proxy", () => { const log = []; const p = new Proxy({}, { getPrototypeOf(tg) { log.push("gpo"); return Array.prototype; }, setPrototypeOf(tg, v) { log.push("spo"); return true; } }); const r = p.__proto__ === Array.prototype; p.__proto__ = null; return [r, log]; });
t("null proto", () => Object.create(null).__proto__);
t("names", () => Object.getOwnPropertyNames(Object.prototype).includes("__proto__"));
t("fn proxy", () => { const log = []; const p = new Proxy(function named() {}, { get(tg, k) { log.push(String(k)); return tg[k]; } }); return [Function.prototype.toString.call(p), log]; });
t("arrow proxy", () => Function.prototype.toString.call(new Proxy(() => 1, {})));
t("name getter", () => Function.prototype.toString.call(new Proxy(function () {}, { get: () => 42 })));
t("not callable", () => Function.prototype.toString.call(new Proxy({}, {})));
