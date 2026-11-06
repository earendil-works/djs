// The builtin closures' lazy `length` and `name`, Promise.prototype.then and the
// resolving functions without builtin frames, async functions that never await, and
// allSettled/any elements settled in Rust (D3155-D3158): each observable step must be the
// spec's.
const log = [];
const show = (label, v) => log.push(label + " " + JSON.stringify(v));

// The resolving functions: own keys, attributes, and changes to them.
{
  let res, rej;
  new Promise((a, b) => { res = a; rej = b; });
  show("keys", Object.getOwnPropertyNames(res));
  show("length name", [res.length, res.name, rej.length, rej.name]);
  show("desc", Object.getOwnPropertyDescriptor(res, "length"));
  show("name desc", Object.getOwnPropertyDescriptor(rej, "name"));
  res.extra = 1;
  show("keys+extra", Reflect.ownKeys(res));
  delete res.length;
  show("deleted length", [Reflect.ownKeys(res), res.length, "length" in res]);
  Object.defineProperty(rej, "name", { value: "r" });
  show("renamed", [Reflect.ownKeys(rej), rej.name, Object.getOwnPropertyDescriptor(rej, "name")]);
  Object.freeze(rej);
  show("frozen", [Object.isFrozen(rej), Object.getOwnPropertyDescriptor(rej, "length").writable]);
  const { resolve, reject } = Promise.withResolvers();
  show("withResolvers", [resolve.length, reject.name, Object.keys(resolve)]);
  let exec;
  class P extends Promise { constructor(e) { super(e); } }
  P.resolve(1);
  new P((a, b) => { exec = [a.length, b.length, a.name]; });
  show("subclass resolving", exec);
}

// A resolve function called twice, with itself, with a thenable; reject after resolve.
{
  let res, rej;
  const p = new Promise((a, b) => { res = a; rej = b; });
  res(1); res(2); rej(3);
  log.push("twice " + (await p));
  let self;
  const q = new Promise((a) => { self = a; });
  self(q);
  try { await q; } catch (e) { log.push("self resolution " + e.constructor.name); }
  const t = new Promise((a) => a({ then(f) { log.push("thenable then"); f("adopted"); } }));
  log.push("thenable " + (await t));
}

// then: the species is read and called as the spec says.
{
  const p = Promise.resolve(1);
  let reads = 0;
  Object.defineProperty(p, "constructor", { get() { reads++; return Promise; } });
  log.push("then species " + (await p.then((v) => v + 1)) + " reads " + reads);
  class Q extends Promise {}
  const q = Q.resolve(2).then((v) => v);
  log.push("subclass then " + (q instanceof Q) + " " + (await q));
  try {
    Promise.prototype.then.call({}, () => {});
  } catch (e) {
    log.push("then on non-promise " + e.constructor.name);
  }
  const r = Promise.resolve(3).then(undefined, 5);
  log.push("non-callable handlers " + (await r));
}

// GetPrototypeFromConstructor: a Proxy new target is asked for `prototype`; a plain one
// gives its own.
{
  let gets = 0;
  function F() {}
  F.prototype = Object.create(Promise.prototype);
  const P = new Proxy(F, { get(t, k) { gets++; return Reflect.get(t, k); } });
  const p = Reflect.construct(Promise, [() => {}], P);
  log.push("newTarget proxy " + gets + " " + (p instanceof F));
  const q = Reflect.construct(Promise, [() => {}], F);
  log.push("newTarget plain " + gets + " " + (Object.getPrototypeOf(q) === F.prototype));
  function G() {}
  G.prototype = 5;
  const r = Reflect.construct(Promise, [() => {}], G);
  log.push("newTarget primitive prototype " + (Object.getPrototypeOf(r) === Promise.prototype));
}

// Async functions that never await: their promise, a throw, a thenable, `arguments`.
{
  const f = async function (a) { return arguments.length + a; };
  const p = f(1, 2);
  log.push("no await " + (p instanceof Promise) + " " + (await p));
  const g = async () => { throw new RangeError("x"); };
  try { await g(); } catch (e) { log.push("no await throw " + e.name); }
  const h = async () => ({ then(r) { r("from thenable"); } });
  log.push("no await thenable " + (await h()));
  const order = [];
  const k = async () => { order.push("body"); };
  k().then(() => order.push("then"));
  order.push("after call");
  await null; await null;
  log.push("no await order " + order.join(" "));
  async function twice(x) { if (x) return 1; await null; return 2; }
  log.push("conditional await " + (await twice(true)) + " " + (await twice(false)));
}

// allSettled and any: a setter on Object.prototype is not called, keys in order, the
// last element of `any` makes the AggregateError.
{
  let setterCalls = 0;
  Object.defineProperty(Object.prototype, "status", { configurable: true, set() { setterCalls++; } });
  const s = await Promise.allSettled([Promise.resolve(1), Promise.reject(2), 3, new Promise((r) => setTimeout(() => r(4), 0))]);
  delete Object.prototype.status;
  show("allSettled", s);
  show("allSettled keys", s.map((o) => Object.keys(o)));
  log.push("allSettled setter " + setterCalls);
  try {
    await Promise.any([Promise.reject(1), Promise.reject(2), new Promise((_, r) => setTimeout(() => r(3), 0))]);
  } catch (e) {
    show("any errors", [e.constructor.name, e.errors]);
  }
  log.push("any first " + (await Promise.any([Promise.reject(1), Promise.resolve(2), Promise.resolve(3)])));
  const settled = Promise.allSettled([]);
  show("allSettled empty", await settled);
}
return log.join("\n");
