// Iterator proposals (0051): chunks/windows, includes, join, zip/zipKeyed.
function t(f) {
  try {
    console.log(f());
  } catch (e) {
    console.log(e);
  }
}
function counter(n, log, name = "it") {
  let i = 0;
  return Iterator.from({
    next() {
      log.push(name + " next");
      i++;
      return i <= n ? { value: i, done: false } : { value: undefined, done: true };
    },
    return() {
      log.push(name + " return");
      return {};
    },
  });
}
const r = (n) => Array.from({ length: n }, (_, i) => i).values();

console.log(r(7).chunks(3).toArray(), r(6).chunks(2).toArray(), r(0).chunks(5).toArray());
console.log(r(5).windows(2).toArray(), r(2).windows(3).toArray(), r(2).windows(3, "allow-partial").toArray());
console.log(r(3).windows(2, "allow-partial").toArray());
const ch = r(3).chunks(2);
const first = ch.next().value;
first.push("mutated");
console.log(first, ch.next(), ch.next());
t(() => r(1).chunks(0));
t(() => r(1).chunks(1.5));
t(() => r(1).windows(Infinity));
t(() => r(1).windows(1, "bad"));
let log = [];
t(() => counter(3, log).chunks("2"));
console.log(log);
log = [];
const w = counter(5, log).windows(2);
w.next();
console.log(w.return(), log);
log = [];
const partial = counter(1, log).chunks(4);
partial.next();
console.log(partial.return(), log);

console.log(r(5).includes(3), r(5).includes(9), [NaN].values().includes(NaN), [-0].values().includes(0));
console.log(r(5).includes(1, 1), r(5).includes(2, 1), r(5).includes(0, Infinity));
t(() => r(5).includes(1, "1"));
t(() => r(5).includes(1, 0.5));
t(() => r(5).includes(1, -1));
log = [];
console.log(counter(5, log).includes(2), log);

console.log(r(4).join(), r(4).join(" - "), r(0).join(), ["a", null, undefined, "b"].values().join());
console.log(r(3).join({ toString: () => "|" }));
log = [];
t(() => counter(3, log).join({ toString() { throw new Error("sep"); } }));
console.log(log);

console.log(Iterator.zip([[1, 2, 3], "xy".split("")]).toArray());
console.log(Iterator.zip([[1, 2, 3], [4]], { mode: "longest" }).toArray());
console.log(Iterator.zip([[1, 2, 3], [4]], { mode: "longest", padding: ["p", "q"] }).toArray());
t(() => Iterator.zip([[1], [2, 3]], { mode: "strict" }).toArray());
t(() => Iterator.zip([[1, 2], [3]], { mode: "strict" }).toArray());
console.log(Iterator.zip([[1], [2]], { mode: "strict" }).toArray(), Iterator.zip([]).toArray());
t(() => Iterator.zip(["ab"]));
t(() => Iterator.zip([], { mode: "nope" }));
t(() => Iterator.zip([], 5));
const keyed = Iterator.zipKeyed({ a: [1, 2], b: [3], [Symbol.for("s")]: [4, 5] }, { mode: "longest", padding: { b: "pad" } }).toArray();
console.log(keyed.map((o) => [Object.getPrototypeOf(o), o.a, o.b, o[Symbol.for("s")]]));
log = [];
const z = Iterator.zip([counter(3, log, "a"), counter(3, log, "b")]);
z.next();
z.return();
console.log(log);
log = [];
const z0 = Iterator.zip([counter(3, log, "a"), counter(3, log, "b")]);
z0.return();
console.log(log);
log = [];
console.log(Iterator.zip([counter(1, log, "a"), counter(3, log, "b")]).toArray(), log);
log = [];
t(() => Iterator.zip([counter(1, log, "a"), 5]));
console.log(log);
