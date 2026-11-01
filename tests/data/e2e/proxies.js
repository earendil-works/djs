// 0045: proxies across suspension and GC: traps on every property path, in loops (so
// the trap frames are suspended and collected mid-flight), revocation, callable and
// constructible proxies, and proxies as prototypes.
const log = [];
const handler = {
  get(t, k, r) { log.push("get " + String(k)); return Reflect.get(t, k, r); },
  set(t, k, v, r) { log.push("set " + String(k)); return Reflect.set(t, k, v, r); },
  has(t, k) { log.push("has " + String(k)); return k in t; },
  deleteProperty(t, k) { log.push("delete " + String(k)); return delete t[k]; },
  ownKeys(t) { log.push("ownKeys"); return Reflect.ownKeys(t); },
};
const target = { a: 1 };
const p = new Proxy(target, handler);
let sum = 0;
for (let i = 0; i < 50; i++) {
  p["k" + (i % 5)] = i;
  sum += p["k" + (i % 5)] + p.a;
}
console.log(sum, Object.keys(p).length, "k3" in p, delete p.k3, "k3" in p);
console.log(log.length, log.slice(0, 4).join(","), log.slice(-3).join(","));
const child = Object.create(p);
child.own = 1;
console.log(child.a, child.own, "a" in child, Object.keys(child));
const counter = new Proxy(function (x) { return x * 2; }, {
  apply(t, self, args) { return t(...args) + 1; },
  construct(t, args) { return { made: args[0] }; },
});
let calls = 0;
for (let i = 0; i < 30; i++) calls += counter(i);
console.log(calls, new counter(5).made, typeof counter);
const r = Proxy.revocable({ v: 7 }, {});
console.log(r.proxy.v);
r.revoke();
try {
  r.proxy.v;
} catch (e) {
  console.log(e.name, e.message);
}
const json = JSON.stringify(new Proxy({ x: [1, 2], y: "s" }, {}));
console.log(json, Array.isArray(new Proxy([], {})), Object.prototype.toString.call(new Proxy([], {})));
const garbage = [];
for (let i = 0; i < 40; i++) garbage.push(new Proxy({ i }, { get: (t, k) => (k === "double" ? t.i * 2 : t[k]) }));
console.log(garbage.reduce((s, q) => s + q.double, 0), { ...garbage[3] }.i);
