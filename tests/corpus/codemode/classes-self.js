// Classes that name themselves, held across awaits (plain dumps, 0207): a helper class
// declared in the body whose constructor counts through its own name, a class expression
// on a global whose method returns its inner binding, a class with a static private field
// still at its initial value.
class Counter {
  static made = 0;
  constructor(label) { this.label = label; Counter.made++; }
  again() { return new Counter(this.label + "+"); }
}
globalThis.Tag = class Tag { static n = 0; self() { return Tag; } };
class Ids { static #next = 1; static peek() { return Ids.#next; } }
const a = new Counter("a");
const r1 = await tools.echo(a.label);
const b = a.again();
Tag.n++;
const r2 = await tools.echo(new Tag().self() === Tag);
console.log(r1, b.label, Counter.made, r2, Tag.n, Ids.peek());
return Counter.made;
