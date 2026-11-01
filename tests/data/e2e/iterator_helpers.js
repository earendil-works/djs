// Iterator helpers (0051): Iterator, Iterator.from, Iterator.concat and the
// Iterator.prototype methods, with their closing and reentrancy rules.
function t(f) {
  try {
    console.log(f());
  } catch (e) {
    console.log(e);
  }
}

// A logging iterator over 1..n.
function counter(n, log) {
  let i = 0;
  return Iterator.from({
    next() {
      log.push("next");
      i++;
      return i <= n ? { value: i, done: false } : { value: undefined, done: true };
    },
    return() {
      log.push("return");
      return {};
    },
  });
}

console.log(
  [1, 2, 3, 4, 5]
    .values()
    .map((x) => x * 10)
    .filter((x) => x !== 20)
    .drop(1)
    .take(2)
    .toArray()
);
console.log([1, 2].values().flatMap((x) => [x, x + 0.5]).toArray());
console.log(["ab", "c"].values().flatMap((s) => new String(s)).toArray());
console.log([1, 2, 3].values().reduce((a, b) => a + b));
console.log([1, 2, 3].values().reduce((a, b) => a + b, 10));
console.log([5, 6].values().map((x, i) => [x, i]).toArray());
console.log([1, 2, 3].values().some((x) => x > 1), [1, 2].values().every((x) => x > 1));
console.log([1, 2, 3].values().find((x) => x > 1));
let seen = [];
[3, 4].values().forEach((x, i) => seen.push(x + ":" + i));
console.log(seen);

// Closing: take(n) closes after n values; some/find close when they stop early.
let log = [];
console.log(counter(5, log).take(2).toArray(), log);
log = [];
console.log(counter(5, log).some((x) => x === 2), log);
log = [];
console.log(counter(5, log).find((x) => x === 9), log);
log = [];
const m = counter(5, log).map((x) => x);
m.next();
console.log(m.return(), log, m.next());
log = [];
const fresh = counter(5, log).filter((x) => true);
console.log(fresh.return(), log);
log = [];
t(() => counter(5, log).map(() => { throw new Error("boom"); }).next());
console.log(log);
log = [];
t(() => counter(5, log).take(-1));
console.log(log);
t(() => Iterator.prototype.map.call({ next() {} }, 1));
t(() => [].values().reduce((a, b) => a));

// flatMap closes the inner iterator, then the outer one.
log = [];
const inner = counter(3, log);
const fm = counter(3, log).flatMap(() => inner);
fm.next();
fm.return();
console.log(log);
t(() => [1].values().flatMap((x) => 1).next());

// Reentrancy and brands.
let h;
h = Iterator.from({ next() { return h.next(); } }).map((x) => x);
t(() => h.next());
const helperProto = Object.getPrototypeOf([].values().map((x) => x));
t(() => helperProto.next.call({}));
t(() => Object.prototype.toString.call(helperProto));

// Iterator.from: strings, wrapping, and iterators that already inherit from Iterator.
console.log(Iterator.from("ab").toArray());
const plain = { i: 0, next() { return { value: this.i++, done: this.i > 2 }; } };
const wrapped = Iterator.from(plain);
console.log(wrapped instanceof Iterator, wrapped.toArray());
const it = [1].values();
console.log(Iterator.from(it) === it);
t(() => Iterator.from(1));

// The constructor is abstract; subclasses work.
t(() => new Iterator());
t(() => Iterator());
class Countdown extends Iterator {
  constructor(n) {
    super();
    this.n = n;
  }
  next() {
    return this.n > 0 ? { value: this.n--, done: false } : { value: undefined, done: true };
  }
}
console.log(new Countdown(3).map((x) => x * 2).toArray());
console.log(Iterator.prototype.constructor === Iterator, Iterator.prototype[Symbol.toStringTag]);
const sub = Object.create(Iterator.prototype);
sub[Symbol.toStringTag] = "Mine";
console.log(Object.prototype.toString.call(sub), Iterator.prototype[Symbol.toStringTag]);
t(() => { Iterator.prototype.constructor = 1; });

// Iterator.concat.
console.log(Iterator.concat([1, 2], new Set([3]), "xy".split("")).toArray());
t(() => Iterator.concat(1));
log = [];
const cat = Iterator.concat({ [Symbol.iterator]: () => counter(3, log) });
cat.next();
cat.return();
console.log(log);
