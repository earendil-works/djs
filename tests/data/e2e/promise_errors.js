// Promise TypeErrors with QuickJS's messages.
const t = (f) => { try { f(); console.log("no error"); } catch (e) { console.log(e.name + ": " + e.message); } };
t(() => Promise.prototype.then.call({}, () => {}));
t(() => new Promise(1));
t(() => Promise(() => {}));
t(() => Promise.resolve.call(1));
t(() => Promise.reject.call(1));
t(() => Promise.all.call(1));
t(() => Promise.prototype.finally.call(1));
t(() => Promise.prototype.catch.call(1));
t(() => Promise.withResolvers.call(1));
t(() => Promise.try.call(1));
t(() => Promise.resolve.call({}));
t(() => Promise.resolve.call(function (e) { e(1, 2); e(3, 4); }));
t(() => Promise.resolve.call(function (e) {}));
const p = Promise.resolve(1);
p.constructor = 1;
t(() => p.then());
p.constructor = { [Symbol.species]: 1 };
t(() => p.then());
t(() => new Promise.prototype.then());
// Errors inside executors and handlers reject.
new Promise(() => { throw new SyntaxError("in executor"); }).catch(e => console.log("executor", e.name, e.message));
Promise.resolve().then(() => { null.x; }).catch(e => console.log("handler", e.name));
Promise.all.call(Promise, 1).catch(e => console.log("all", e.name + ": " + e.message));
// A custom capability constructor.
function Fake(executor) {
  executor(v => console.log("fake resolve", v), r => console.log("fake reject", r));
}
Fake.resolve = v => v;
Promise.all.call(Fake, [1, 2]);
Promise.race.call(Fake, [3]);
Promise.reject.call(Fake, 4);
Promise.resolve.call(Fake, 5);
