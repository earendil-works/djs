"djs-dump 2";
$dump.keep = (o, keys) => {
  for (const k of Reflect.ownKeys(o)) {
    if (!["length", "name", "prototype", "constructor"].includes(k) && !keys.includes(k)) delete o[k];
  }
  for (const k of keys) if (!Object.prototype.hasOwnProperty.call(o, k)) throw new TypeError("the class does not make " + String(k));
};
$dump.p1 = $dump.pending(1, "later", [3]);
class Counter {
  #n = 0;
  static made = 0;
  static { Counter.kind = "counter"; }
  label = "c";
  get n() { return this.#n; }
  #bump(by) { this.#n += by; }
  async add(by) { const v = await later(by); this.#bump(v); return this.n; }
}
const c = { __proto__: Counter.prototype, label: "c" };
$dump.a2 = $dump.resume(Counter.prototype.add, { at: "await#1", awaiting: $dump.p1, this: c, args: [3], bindings: { by: 3, v: undefined } });
$dump.p3 = Promise.prototype.then.call($dump.a2, (n) => print(c.label, n, Counter.made, Counter.kind), undefined);
$dump.keep(Counter, []);
$dump.keep(Counter.prototype, ["add", "n"]);
Object.defineProperty(Counter, "made", { value: 1, writable: true, enumerable: true, configurable: true });
Object.defineProperty(Counter, "kind", { value: "counter", writable: true, enumerable: true, configurable: true });
$dump.private(c, Counter, "#bump");
$dump.private(c, Counter, "#n", 0);
