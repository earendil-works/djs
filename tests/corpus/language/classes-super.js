// The e2e case tests/data/e2e/classes_super.js, here for the stress modes.
// Classes (0040): `extends`, derived constructors, `super` calls and properties, home
// objects (also in object literals), subclassing builtins.
function t(f) {
  try {
    console.log(f());
  } catch (e) {
    console.log(e);
  }
}
class Animal {
  constructor(name) {
    this.name = name;
  }
  speak() {
    return `${this.name} makes a sound`;
  }
  get kind() {
    return "animal";
  }
  static create(n) {
    return new this(n);
  }
}
class Dog extends Animal {
  constructor(name) {
    super(name);
    this.tricks = [];
  }
  speak() {
    return super.speak() + " (woof)";
  }
  get kind() {
    return "dog/" + super.kind;
  }
  static create(n) {
    const d = super.create(n);
    d.tricks.push("sit");
    return d;
  }
}
const d = Dog.create("Rex");
console.log(d.speak(), d.kind, d.tricks, d instanceof Dog, d instanceof Animal);
console.log(Object.getPrototypeOf(Dog) === Animal, Object.getPrototypeOf(Dog.prototype) === Animal.prototype);
class Puppy extends Dog {}
const pup = new Puppy("Bit");
console.log(pup.speak(), pup.constructor.name, Puppy.length);

// new.target, the default derived constructor passes everything on.
class Base {
  constructor(...args) {
    this.args = args;
    this.nt = new.target.name;
  }
}
class Derived extends Base {}
const der = new Derived(1, 2, 3);
console.log(der.args, der.nt);
console.log(Reflect.construct(Base, [], Derived).nt);

// `this` before super(), super() twice, return overrides.
t(() => {
  class A extends Base {
    constructor() {
      this.x = 1;
      super();
    }
  }
  return new A();
});
t(() => {
  class A extends Base {
    constructor() {
      super();
      super();
    }
  }
  return new A();
});
t(() => {
  class A extends Base {
    constructor() {}
  }
  return new A();
});
t(() => {
  class A extends Base {
    constructor() {
      return { custom: true };
    }
  }
  return JSON.stringify(new A());
});
t(() => {
  class A extends Base {
    constructor() {
      super();
      return 1;
    }
  }
  return new A();
});
t(() => {
  class A extends Base {
    constructor() {
      super();
      try {
        return 0;
      } catch (e) {
        return "caught";
      }
    }
  }
  return new A();
});
t(() => {
  class A {
    constructor() {
      return 1;
    }
  }
  return typeof new A();
});
t(() => {
  class A extends Base {
    constructor() {
      const f = () => super(7);
      f();
      return this.args;
    }
  }
  return new A();
});
t(() => {
  class A extends Base {
    constructor() {
      try {
        return;
      } finally {
        super("late");
      }
    }
  }
  return new A().args;
});

// Bad heritage.
t(() => {
  class A extends 1 {}
});
t(() => {
  function F() {}
  F.prototype = 3;
  class A extends F {}
});
t(() => {
  class A extends null {}
  return [Object.getPrototypeOf(A) === Function.prototype, Object.getPrototypeOf(A.prototype)];
});
t(() => {
  class A extends null {
    constructor() {
      return Object.create(A.prototype);
    }
  }
  return new A() instanceof A;
});
t(() => {
  class A extends Base {}
  Object.setPrototypeOf(A, {});
  return new A();
});

// super property assignment uses `this` as the receiver.
class P {
  set v(x) {
    this.seen = x;
  }
}
class Q extends P {
  m() {
    super.v = 5;
    super.w = 6;
    super["x" + 1] = 7;
    return [this.seen, this.w, this.x1, Object.keys(this)];
  }
  inc() {
    this.n = 1;
    super.n += 10;
    super.n++;
    return this.n;
  }
  del() {
    delete super.n;
  }
}
console.log(new Q().m(), new Q().inc());
t(() => new Q().del());
t(() => {
  class A {
    static m() {
      super.x = 1;
    }
  }
  Object.freeze(A);
  A.m();
});
t(() => {
  class A {
    static m() {
      return super.x;
    }
  }
  Object.setPrototypeOf(A, null);
  return A.m();
});

// Object literal methods have home objects too.
const proto = {
  greet() {
    return "proto hi " + this.who;
  },
};
const o = {
  __proto__: proto,
  who: "o",
  greet() {
    return super.greet() + "!";
  },
  get g() {
    return super.greet.name;
  },
  arrow() {
    return (() => super.greet())();
  },
};
console.log(o.greet(), o.g, o.arrow());
const moved = { greet: o.greet, who: "moved" };
console.log(moved.greet());

// Subclassing builtins.
class MyArray extends Array {
  sum() {
    return this.reduce((a, b) => a + b, 0);
  }
}
const ma = MyArray.from([1, 2, 3]);
ma.push(4);
console.log(ma.sum(), ma.length, Array.isArray(ma), ma instanceof MyArray, ma.map((x) => x * 2) instanceof MyArray);
class MyError extends Error {
  constructor(m) {
    super(m);
    this.name = "MyError";
  }
}
const me = new MyError("bad");
console.log(String(me), me instanceof Error, me.message, Object.prototype.toString.call(me));
class MyMap extends Map {
  getOr(k, v) {
    return this.has(k) ? this.get(k) : v;
  }
}
const mm = new MyMap([[1, "a"]]);
console.log(mm.getOr(1, "z"), mm.getOr(2, "z"), mm.size);
class MyPromise extends Promise {}
const mp = MyPromise.resolve(1);
console.log(mp instanceof MyPromise, mp.then((x) => x) instanceof MyPromise);
mp.then((v) => console.log("my promise", v));
