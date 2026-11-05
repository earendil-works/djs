// await, async return and Promise.all skip their builtin steps when no user code could run
// (D3130, D3131). Every case here makes one of those steps observable, so it must take
// the full path, in the same job order as the spec's.
const log = [];
const tick = (n) => {
  let p = Promise.resolve();
  for (let i = 0; i < n; i++) p = p.then(() => {});
  return p;
};

// Job order: await of a value, of a promise and of a thenable against then callbacks.
async function order() {
  const seq = [];
  const a = (async () => { seq.push("a0"); await 1; seq.push("a1"); await null; seq.push("a2"); })();
  const b = (async () => { seq.push("b0"); await Promise.resolve(); seq.push("b1"); })();
  const c = (async () => { seq.push("c0"); await { then(r) { seq.push("then"); r(); } }; seq.push("c1"); })();
  Promise.resolve().then(() => seq.push("p1")).then(() => seq.push("p2")).then(() => seq.push("p3"));
  await Promise.all([a, b, c]);
  return seq.join(" ");
}
log.push("order " + (await order()));

// An async function returning a promise takes two extra ticks; a plain value none.
{
  const seq = [];
  const f = async () => Promise.resolve("p");
  const g = async () => "v";
  f().then((v) => seq.push(v));
  g().then((v) => seq.push(v));
  await tick(4);
  log.push("return order " + seq.join(" "));
}

// A getter for `then` on an awaited object runs, once per await.
{
  let gets = 0;
  const o = { get then() { gets++; return undefined; } };
  const v = await o;
  log.push("then getter " + gets + " " + (v === o));
}

// A non-callable `then` fulfills with the object itself, also from an async return.
{
  const o = { then: 42 };
  const r = await (async () => o)();
  log.push("then not callable " + (r === o));
}

// A Proxy is asked for `then`.
{
  const keys = [];
  const p = new Proxy({}, { get(t, k) { keys.push(String(k)); return undefined; } });
  await p;
  await (async () => p)();
  log.push("proxy " + keys.join(","));
}

// A promise with its own `constructor`, or a patched %Promise.prototype%.constructor,
// is wrapped in a new promise: its `then` runs.
{
  const p = Promise.resolve(5);
  let thens = 0;
  p.then = function (f, r) { thens++; return Promise.prototype.then.call(this, f, r); };
  log.push("own then " + (await p) + " " + thens);
  const q = Promise.resolve(6);
  Object.defineProperty(q, "constructor", { get() { log.push("constructor read"); return Promise; } });
  log.push("own constructor " + (await q));
}

// Promise.all: a patched then, resolve, species or constructor is called as the spec says.
{
  const then = Promise.prototype.then;
  let calls = 0;
  Promise.prototype.then = function (f, r) { calls++; return then.call(this, f, r); };
  const v = await Promise.all([1, Promise.resolve(2)]);
  Promise.prototype.then = then;
  log.push("patched then " + v + " calls " + calls);

  const resolve = Promise.resolve;
  let resolves = 0;
  Promise.resolve = function (x) { resolves++; return resolve.call(this, x); };
  log.push("patched resolve " + (await Promise.all([3, 4])) + " " + resolves);
  Promise.resolve = resolve;

  const species = Object.getOwnPropertyDescriptor(Promise, Symbol.species);
  let speciesGets = 0;
  Object.defineProperty(Promise, Symbol.species, { configurable: true, get() { speciesGets++; return Promise; } });
  log.push("patched species " + (await Promise.all([5, Promise.resolve(6)])) + " " + speciesGets);
  Object.defineProperty(Promise, Symbol.species, species);
}

// The values array is resolved like any value: a `then` on Array.prototype is called.
{
  let seen = 0;
  Object.defineProperty(Array.prototype, "then", {
    configurable: true,
    get() { seen++; return undefined; },
  });
  const v = await Promise.all([Promise.resolve(7), 8]);
  delete Array.prototype.then;
  log.push("values then " + v + " " + (seen > 0));
}

// A resolve element function called twice stores once; Promise.all of a subclass.
{
  let keep;
  const t = { then(r) { keep = r; r("first"); } };
  const v = await Promise.all([t, 9]);
  keep("second");
  log.push("twice " + v);
  class Sub extends Promise {}
  const s = Sub.all([Sub.resolve(10), 11]);
  log.push("subclass " + (s instanceof Sub) + " " + (await s));
}

// allSettled, any and race share the loop.
{
  const s = await Promise.allSettled([1, Promise.reject(2), Promise.resolve(3)]);
  log.push("allSettled " + JSON.stringify(s));
  log.push("any " + (await Promise.any([Promise.reject(1), Promise.resolve(2), 3])));
  log.push("race " + (await Promise.race([new Promise(() => {}), Promise.resolve(4), 5])));
  try {
    await Promise.all([Promise.resolve(1), Promise.reject(new Error("no")), 3]);
  } catch (e) {
    log.push("all rejects " + e.message);
  }
}

// An async function that throws rejects; an unhandled rejection of an awaited promise is
// handled by the await.
{
  const f = async () => { throw new TypeError("bad"); };
  try { await f(); } catch (e) { log.push("thrown " + e.name); }
  const r = Promise.reject(new Error("late"));
  try { await r; } catch (e) { log.push("awaited rejection " + e.message); }
}
return log.join("\n");
