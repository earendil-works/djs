// Microtask order: reactions run in registration order, one tick per step; adopting a
// thenable or a promise costs extra ticks.
const log = [];
const tick = (name, n) => {
  let p = Promise.resolve();
  for (let i = 0; i < n; i++) p = p.then(() => log.push(name + i));
  return p;
};
tick("a", 4);
tick("b", 4);
Promise.resolve().then(() => Promise.resolve("adopted")).then(v => log.push(v));
Promise.resolve().then(() => ({ then(r) { log.push("thenable called"); r("thenable"); } })).then(v => log.push(v));
new Promise(r => r(Promise.resolve("ctor adopt"))).then(v => log.push(v));
const p = Promise.resolve("shared");
p.then(v => log.push(v + "1"));
p.then(v => log.push(v + "2"));
tick("c", 6).then(() => console.log(log.join(" ")));
// A getter for `then` runs synchronously during resolution.
let getterRan = false;
new Promise(r => r({ get then() { getterRan = true; return undefined; } })).then(v => console.log("getter", getterRan, typeof v));
console.log("sync getter", getterRan);
// then on a subclass-like constructor: species decides the result's constructor.
const p2 = Promise.resolve(1);
p2.constructor = undefined;
console.log("undefined constructor", p2.then() instanceof Promise);
// A throwing then getter rejects.
Promise.resolve({ get then() { throw new RangeError("then getter"); } }).catch(e => console.log("rejected by", e.name));
// Self resolution.
let res;
const self = new Promise(r => { res = r; });
res(self);
self.catch(e => console.log("self", e.name, e.message));
