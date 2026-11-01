// The e2e case tests/data/e2e/classes_private.js, here for the stress modes.
// Classes (0040): private fields, methods and accessors, static privates, brand checks,
// `#x in obj`, and their errors.
function t(f) {
  try {
    console.log(f());
  } catch (e) {
    console.log(e);
  }
}
class Account {
  #balance = 0;
  static #count = 0;
  #log = [];
  constructor(start) {
    this.#balance = start;
    Account.#count++;
  }
  #record(what) {
    this.#log.push(what);
  }
  get #summary() {
    return `${this.#balance} after ${this.#log.length}`;
  }
  set #amount(v) {
    this.#balance += v;
    this.#record(v);
  }
  deposit(v) {
    this.#amount = v;
    return this.#summary;
  }
  static count() {
    return Account.#count;
  }
  static has(o) {
    return #balance in o && #record in o;
  }
  static peek(o) {
    return o.#balance;
  }
  static #secret() {
    return "s" + this.#count;
  }
  static reveal() {
    return this.#secret();
  }
}
const a = new Account(10);
new Account(0);
console.log(a.deposit(5), a.deposit(-3), Account.count(), Account.reveal());
console.log(Account.has(a), Account.has({}), Object.keys(a), JSON.stringify(a));
console.log(Reflect.ownKeys(a), Object.getOwnPropertySymbols(a), Reflect.ownKeys(Account));
t(() => Account.peek({}));
t(() => Account.peek(1));
t(() => Account.has(1));

// Brand checks on methods and accessors.
class B {
  #m() {
    return 1;
  }
  get #g() {
    return 2;
  }
  set #s(v) {}
  static call(o) {
    return o.#m();
  }
  static write(o) {
    o.#m = 1;
  }
  static writeGetter(o) {
    o.#g = 1;
  }
  static readSetter(o) {
    return o.#s;
  }
  static same(o) {
    return o.#m === new B().#m;
  }
}
t(() => B.call(new B()));
t(() => B.call({}));
t(() => B.write(new B()));
t(() => B.writeGetter(new B()));
t(() => B.readSetter(new B()));
t(() => B.same(new B()));

// Return override: private names on foreign objects, once.
class Ret {
  constructor(o) {
    return o;
  }
}
class Stamp extends Ret {
  #id = 7;
  static id(o) {
    return o.#id;
  }
}
const plain = {};
new Stamp(plain);
console.log(Stamp.id(plain), Object.keys(plain));
t(() => new Stamp(plain));
class Meth extends Ret {
  #m() {}
  static has(o) {
    return #m in o;
  }
}
const plain2 = {};
new Meth(plain2);
console.log(Meth.has(plain2));
t(() => new Meth(plain2));

// Private names are per class evaluation; frozen objects keep their private fields
// writable. (Adding one to a non-extensible object is a TypeError since
// nonextensible-applies-to-private, which qjs does not implement yet.)
function make() {
  return class {
    #v = 1;
    static get(o) {
      return o.#v;
    }
  };
}
const K1 = make();
const K2 = make();
t(() => K1.get(new K2()));
class Fr {
  #v = 1;
  bump() {
    Object.freeze(this);
    this.#v++;
    return this.#v;
  }
}
console.log(new Fr().bump());

// Compound and logical assignment, update, destructuring targets.
class Ops {
  #n = 1;
  #u;
  run() {
    this.#n += 2;
    this.#n **= 2;
    this.#n++;
    ++this.#n;
    this.#u ??= "set";
    this.#u ||= "no";
    [this.#n, ...this.#u] = [100, "a", "b"];
    ({ x: this.#n } = { x: this.#n + 1 });
    return [this.#n, this.#u];
  }
}
console.log(new Ops().run());

// Optional chains, nested classes, private names in arrows and computed keys.
class Outer {
  #x = "outer";
  static read(o) {
    return o?.#x;
  }
  inner() {
    const self = this;
    class Inner {
      #y = "inner";
      both() {
        return [this.#y, self.#x, #x in self, #x in this];
      }
    }
    return new Inner().both();
  }
  arrow() {
    return (() => this.#x)();
  }
}
console.log(Outer.read(new Outer()), Outer.read(null), new Outer().inner(), new Outer().arrow());

// Proxies: private names never trap and live on the proxy object itself; public fields
// go through its defineProperty trap.
class PA {
  #x = 1;
  static g(o) {
    return o.#x;
  }
}
t(() => PA.g(new Proxy(new PA(), {})));
const traps = [];
class PB {
  constructor() {
    return new Proxy(
      {},
      {
        defineProperty(t, k, d) {
          traps.push(String(k));
          return Reflect.defineProperty(t, k, d);
        },
      },
    );
  }
}
class PC extends PB {
  #y = 5;
  pub = 6;
  static g(o) {
    return [o.#y, #y in o, o.pub];
  }
}
const pc = new PC();
console.log(PC.g(pc), traps, Reflect.ownKeys(pc));
