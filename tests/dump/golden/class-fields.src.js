// A class with fields, private fields and methods, a static block, an accessor.
class Counter {
  #n = 0;
  static made = 0;
  static { Counter.kind = "counter"; }
  label = "c";
  get n() { return this.#n; }
  #bump(by) { this.#n += by; }
  async add(by) { const v = await later(by); this.#bump(v); return this.n; }
}
const c = new Counter();
Counter.made++;
c.add(3).then((n) => print(c.label, n, Counter.made, Counter.kind));
