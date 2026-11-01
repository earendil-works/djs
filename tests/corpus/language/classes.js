// The e2e case tests/data/e2e/classes.js, here for the stress modes.
// Classes (0040): declarations and expressions, methods, accessors, static members,
// computed keys, fields, static blocks, the class binding and its TDZ.
function t(f) {
  try {
    console.log(f());
  } catch (e) {
    console.log(e);
  }
}
class Point {
  constructor(x, y) {
    this.x = x;
    this.y = y;
  }
  get len() {
    return Math.hypot(this.x, this.y);
  }
  set len(v) {
    const k = v / this.len;
    this.x *= k;
    this.y *= k;
  }
  add(o) {
    return new Point(this.x + o.x, this.y + o.y);
  }
  static origin() {
    return new Point(0, 0);
  }
  toString() {
    return `(${this.x}, ${this.y})`;
  }
}
const p = new Point(3, 4);
console.log(p.len, String(p.add(Point.origin())), p instanceof Point);
p.len = 10;
console.log(p.x, p.y);
console.log(typeof Point, Point.name, Point.length, Object.getOwnPropertyNames(Point));
console.log(Object.getOwnPropertyNames(Point.prototype));
const d = Object.getOwnPropertyDescriptor(Point, "prototype");
console.log(d.writable, d.enumerable, d.configurable);
const m = Object.getOwnPropertyDescriptor(Point.prototype, "add");
console.log(m.writable, m.enumerable, m.configurable, Point.prototype.constructor === Point);
const g = Object.getOwnPropertyDescriptor(Point.prototype, "len");
console.log(typeof g.get, typeof g.set, g.enumerable, g.get.name, g.set.name);
console.log(Object.keys(p), JSON.stringify(p));
t(() => Point(1, 2));
t(() => new Point.origin());
t(() => new Point.prototype.add());

// Class expressions, names, computed keys.
const k = "dyn";
const s = Symbol("sym");
const E = class {
  [k + "amic"]() {
    return 1;
  }
  get [s]() {
    return 2;
  }
  static [`st${1 + 1}`] = 3;
  42() {
    return "num";
  }
  "quoted key"() {
    return "q";
  }
};
const e = new E();
console.log(E.name, e.dynamic(), e[s], E.st2, e[42](), e["quoted key"]());
console.log(e.dynamic.name, Object.getOwnPropertyDescriptor(E.prototype, s).get.name);
const Named = class Inner {
  who() {
    return Inner.name;
  }
};
console.log(Named.name, new Named().who(), typeof Inner);
const obj = { C: class {} };
console.log(obj.C.name, (class {}).name, String(class A { m() {} }));
t(() => {
  class B {
    m() {
      B = 1;
    }
  }
  new B().m();
});
t(() => {
  new C0();
  class C0 {}
});
t(() => {
  class C1 extends C1 {}
});
t(() => {
  class C2 {
    static x = C2.name;
  }
  return C2.x;
});

// Fields, initializer order, `this` in initializers, static blocks.
const order = [];
const key = (n) => {
  order.push("key " + n);
  return n;
};
class F {
  a = (order.push("a"), 1);
  [key("b")] = (order.push("b"), this.a + 1);
  static s = (order.push("static s"), "S");
  c;
  static {
    order.push("block " + this.s);
    this.t = this.s + "T";
  }
  constructor() {
    order.push("ctor " + this.a);
  }
  arrow = () => this.a;
  static [key("st")] = 5;
}
order.push("defined");
const f = new F();
console.log(order.join(", "));
console.log(f.a, f.b, f.c, "c" in f, F.t, F.st, f.arrow.call(null), Object.keys(f));
const fd = Object.getOwnPropertyDescriptor(f, "a");
console.log(fd.writable, fd.enumerable, fd.configurable);
class Named2 {
  fn = function () {};
  arrow = () => {};
  static sfn = function () {};
  ["com" + "puted"] = () => {};
}
const n2 = new Named2();
console.log(n2.fn.name, n2.arrow.name, Named2.sfn.name, n2.computed.name);
t(() => {
  class X {
    x = 1;
  }
  Object.preventExtensions(X.prototype);
  class Y extends X {
    y = 2;
  }
  return JSON.stringify(new Y());
});
class Counter {
  static count = 0;
  static {
    for (let i = 0; i < 3; i++) this.count += i;
  }
}
console.log(Counter.count);

// Generators and async methods.
class G {
  *gen() {
    yield 1;
    yield 2;
  }
  static *[Symbol.iterator]() {
    yield* [3, 4];
  }
  async am() {
    return 5;
  }
  static async *ag() {
    yield 6;
  }
}
console.log([...new G().gen()], [...G]);
console.log("async method", await new G().am());
console.log("async gen", (await G.ag().next()).value);
console.log(typeof G.prototype.gen.prototype, G.prototype.am.prototype);

// Accessor pairs and statics merge; static and instance names may repeat.
class Acc {
  static get v() {
    return "sg";
  }
  static set v(x) {
    Acc.last = x;
  }
  get v() {
    return "ig";
  }
  static v2 = 1;
  v2 = 2;
}
Acc.v = 9;
console.log(Acc.v, Acc.last, new Acc().v, Acc.v2, new Acc().v2);

// Classes in loops get fresh bindings.
const made = [];
for (let i = 0; i < 2; i++) {
  made.push(
    class {
      static id = i;
    },
  );
}
console.log(made[0].id, made[1].id, made[0] === made[1]);
