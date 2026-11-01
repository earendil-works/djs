// Array.fromAsync (0051): async iterables, sync iterables of promises, array-likes,
// mapping, closing on errors, and tick order against other jobs.
const log = [];
async function* agen() {
  try {
    yield 1;
    yield Promise.resolve(2);
    yield 3;
  } finally {
    log.push("agen finally");
  }
}
function syncIter(values) {
  let i = 0;
  return {
    [Symbol.iterator]() {
      return {
        next() {
          log.push("next " + i);
          return i < values.length ? { value: values[i++], done: false } : { value: undefined, done: true };
        },
        return() {
          log.push("return");
          return {};
        },
      };
    },
  };
}
const show = (p) => p.then((v) => console.log("ok", v), (e) => console.log("err", String(e)));
(async () => {
  await show(Array.fromAsync(agen()));
  await show(Array.fromAsync([Promise.resolve(1), 2, Promise.resolve(3)], (x, i) => x * 10 + i));
  await show(Array.fromAsync({ length: 3, 0: "a", 1: Promise.resolve("b"), 2: "c" }));
  await show(Array.fromAsync(syncIter([1, 2]), async (x) => x + 1));
  console.log(log.splice(0).join());
  await show(Array.fromAsync(syncIter([1, 2]), (x) => { throw new Error("map"); }));
  console.log(log.splice(0).join());
  await show(Array.fromAsync(agen(), async (x) => { if (x === 2) throw new RangeError("async map"); return x; }));
  console.log(log.splice(0).join());
  await show(Array.fromAsync([Promise.reject(new Error("rejected"))]));
  await show(Array.fromAsync([1], 5));
  await show(Array.fromAsync(null));
  await show(Array.fromAsync(5));
  class C {
    constructor(...args) {
      log.push("C " + args.length);
    }
  }
  const c = await Array.fromAsync.call(C, [1, 2]);
  const d = await Array.fromAsync.call(C, { length: 1, 0: "x" });
  console.log(c instanceof C, c.length, c[1], d instanceof C, d.length, log.splice(0).join());
  console.log(Array.fromAsync.length, Array.fromAsync([]) instanceof Promise);
  // Interleaving with other jobs.
  const order = [];
  const p = Array.fromAsync([1, 2, 3]).then(() => order.push("fromAsync"));
  Promise.resolve().then(() => order.push("a")).then(() => order.push("b")).then(() => order.push("c")).then(() => order.push("d"));
  await p;
  await null;
  await null;
  console.log(order.join());
})();
