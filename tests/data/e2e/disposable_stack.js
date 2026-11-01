// DisposableStack, AsyncDisposableStack, SuppressedError and the iterator dispose
// methods (0051; the `using` syntax is not there yet).
function t(f) {
  try {
    console.log(f());
  } catch (e) {
    console.log(e);
  }
}
const log = [];
const res = (name, fail) => ({
  [Symbol.dispose]() {
    log.push("dispose " + name);
    if (fail) throw new Error(name);
  },
});
console.log(typeof Symbol.dispose, String(Symbol.dispose), String(Symbol.asyncDispose));

const s = new DisposableStack();
console.log(s.use(null), s.adopt(5, (v) => log.push("adopt " + v)), s.defer(() => log.push("defer")));
s.use(res("a"));
console.log(s.disposed, s.dispose(), s.disposed, s.dispose(), log.splice(0));
t(() => s.use(res("late")));
t(() => new DisposableStack().use(1));
t(() => new DisposableStack().use({}));
t(() => DisposableStack());
t(() => DisposableStack.prototype.dispose.call({}));
const f = new DisposableStack();
f.use(res("x", true));
f.use(res("y"));
f.use(res("z", true));
try {
  f.dispose();
} catch (e) {
  console.log(e.name, e instanceof SuppressedError, e.error.message, e.suppressed.message, log.splice(0));
}
const m = new DisposableStack();
m.use(res("moved"));
const moved = m.move();
m.dispose();
console.log(log.splice(0), m.disposed, moved.disposed);
moved[Symbol.dispose]();
console.log(log.splice(0), Object.prototype.toString.call(moved));

const se = new SuppressedError(new Error("e"), "s", "msg");
console.log(se.message, se.error.message, se.suppressed, se instanceof Error, SuppressedError.prototype.name, SuppressedError.length);
console.log(Object.getPrototypeOf(SuppressedError) === Error, new SuppressedError().hasOwnProperty("message"));

// Iterator.prototype[Symbol.dispose] calls return.
const it = [1, 2].values().map((x) => x);
it.next();
it[Symbol.dispose]();
console.log(it.next());

(async () => {
  const a = new AsyncDisposableStack();
  a.use({
    async [Symbol.asyncDispose]() {
      log.push("async dispose");
    },
  });
  a.use(res("sync in async"));
  a.use(null);
  a.defer(async () => {
    await null;
    log.push("deferred async");
  });
  const order = [];
  const done = a.disposeAsync().then(() => order.push("disposed"));
  Promise.resolve().then(() => order.push(1)).then(() => order.push(2)).then(() => order.push(3)).then(() => order.push(4)).then(() => order.push(5));
  await done;
  await null;
  await null;
  await null;
  console.log(log.splice(0), order.join(), a.disposed);
  console.log(await a.disposeAsync());
  const b = new AsyncDisposableStack();
  b.use(res("sync fail", true));
  b.defer(() => Promise.reject(new Error("async fail")));
  b.defer(() => {
    throw new Error("sync throw");
  });
  await b.disposeAsync().catch((e) => console.log(e.name, e.error.message, e.suppressed.name, e.suppressed.error.message, e.suppressed.suppressed.message));
  console.log(log.splice(0));
  t(() => b.use({}));
  t(() => new AsyncDisposableStack().use({}));
  t(() => new AsyncDisposableStack().use(5));
  await AsyncDisposableStack.prototype.disposeAsync.call({}).catch((e) => console.log(String(e)));
  async function* gen() {
    try {
      yield 1;
    } finally {
      log.push("gen closed");
    }
  }
  const g = gen();
  await g.next();
  console.log(await g[Symbol.asyncDispose](), log.splice(0));
})();
