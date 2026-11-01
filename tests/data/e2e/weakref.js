// WeakRef and FinalizationRegistry (0051): the parts that do not depend on when the
// collector runs (every target stays reachable, so plain and gc-stress runs agree).
function t(f) {
  try {
    console.log(f());
  } catch (e) {
    console.log(e);
  }
}
const target = { name: "target" };
const sym = Symbol("local");
const w = new WeakRef(target);
console.log(w.deref() === target, new WeakRef(sym).deref() === sym, Object.prototype.toString.call(w));
t(() => new WeakRef(1));
t(() => new WeakRef(Symbol.for("registered")));
t(() => WeakRef(target));
t(() => WeakRef.prototype.deref.call({}));
const log = [];
const r = new FinalizationRegistry((held) => log.push(held));
const token = {};
console.log(r.register(target, "held", token), r.register(sym, 1), r.register(target, target.name, sym));
console.log(r.unregister(token), r.unregister(token), r.unregister(sym), r.unregister({}));
t(() => r.register(1, 2));
t(() => r.register(target, target));
t(() => r.register(target, 1, 2));
t(() => r.unregister(1));
t(() => new FinalizationRegistry(1));
t(() => FinalizationRegistry(() => 1));
t(() => FinalizationRegistry.prototype.register.call(w, target, 1));
class MyRef extends WeakRef {
  constructor(t) {
    super(t);
    this.extra = 1;
  }
}
const mine = new MyRef(target);
console.log(mine.deref() === target, mine instanceof WeakRef, mine.extra, WeakRef.prototype.deref.length);
console.log(FinalizationRegistry.prototype.register.length, Object.prototype.toString.call(r), log);
