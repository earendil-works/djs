// Decorators and auto-accessors (0065): context objects, application and initializer
// order, replacement, metadata, private elements, errors.
function t(f) {
  try {
    console.log(f());
  } catch (e) {
    console.log(e);
  }
}
const log = [];
const tag = (name) => (value, ctx) => {
  log.push(`${name}:${ctx.kind}:${String(ctx.name)}:${ctx.static}:${ctx.private}`);
  ctx.addInitializer(function () {
    log.push(`init ${name}:${typeof this}`);
  });
};

// Context objects.
let ctxs = [];
const save = (v, ctx) => {
  ctxs.push(ctx);
};
class K {
  @save m() {}
  @save static accessor s = 1;
  @save #f = 2;
  @save set #w(v) {}
}
for (const c of ctxs) {
  const access = Object.keys(c.access).map((k) => `${k}/${c.access[k].length}/${c.access[k].name}`);
  console.log(Object.keys(c).join(), c.kind, String(c.name), access.join(), c.addInitializer.length);
}
console.log(Object.getPrototypeOf(ctxs[0].metadata), ctxs[0].metadata === ctxs[1].metadata);
console.log(K[Symbol.metadata] === ctxs[0].metadata);

// Order of decoration and of the initializers, with inheritance.
class Base {
  @tag("bf") bf = log.push("bf value");
}
@tag("cls")
class Derived extends Base {
  @tag("sm") static sm() {}
  @tag("m") m() {}
  @tag("g") get g() { return 1; }
  @tag("df") df = log.push("df value");
  @tag("sa") static accessor sa = log.push("sa value");
  constructor() {
    log.push("before super");
    super();
    log.push("after super");
  }
}
log.push("defined");
new Derived();
console.log(log.join("\n"));

// Replacement of methods, getters, setters and accessors, public and private, static.
const wrap = (m, ctx) => function (...a) {
  return `${String(ctx.name)}(${m.call(this, ...a)})`;
};
const twice = (s) => function (v) {
  s.call(this, v * 2);
};
const tenfold = ({ get, set }) => ({
  get() {
    return get.call(this) * 10;
  },
  set(v) {
    set.call(this, v + 1);
  },
  init(v) {
    return v + 100;
  },
});
class R {
  @wrap static #sm() { return "sm"; }
  @wrap static get #sg() { return "sg"; }
  @tenfold static accessor #sa = 1;
  @wrap get g() { return "g"; }
  @twice set g(v) { this.v = v; }
  @tenfold accessor [`a${1}`] = 2;
  static run() {
    R.#sa = 5;
    return [R.#sm(), R.#sg, R.#sa].join();
  }
}
t(() => R.run());
t(() => {
  const r = new R();
  r.g = 4;
  r.a1 = 3;
  return [r.g, r.v, r.a1, Object.getOwnPropertyDescriptor(R.prototype, "a1").get.name].join();
});

// Class replacement: the inner binding, `this` of the static elements, metadata.
let original;
const sub = (cls, ctx) => {
  original = cls;
  ctx.metadata.replaced = ctx.name;
  return class extends cls {
    static tag = "sub";
  };
};
@sub
class E {
  static self = this;
  static #p = 1;
  static inner() {
    return E;
  }
  static get p() {
    return this.#p;
  }
}
t(() => [E.tag, E.self === E, E.inner() === E, Object.getPrototypeOf(E) === original].join());
t(() => E[Symbol.metadata].replaced);
const Anon = @sub class {};
t(() => [Anon.name, original.name].join());

// Metadata inheritance.
const meta = (k) => (_, ctx) => {
  ctx.metadata[k] = true;
};
@meta("a") class MA {}
class MB extends MA {
  @meta("b") static x;
}
class MC extends MB {}
t(() => [Object.keys(MB[Symbol.metadata]).join(), "a" in MB[Symbol.metadata],
  MC[Symbol.metadata] === MB[Symbol.metadata]].join());
t(() => {
  const P = function () {};
  P[Symbol.metadata] = 1;
  return class extends P { @meta("c") m() {} };
});

// Receivers, call expressions, private names in decorators.
const host = {
  name: "host",
  dec() {
    log.length = 0;
    log.push(this.name);
  },
  make(n) {
    return () => {
      log.push(n);
    };
  },
};
class Q {
  static #d = (v, ctx) => {
    log.push("private " + ctx.name);
  };
  static make() {
    return class { @host.dec @(host.make)("x") @host.make("y") @Q.#d m() {} };
  }
}
Q.make();
console.log(log.join());

// Errors.
const ret = (v) => () => v;
t(() => class { @(ret(1)) m() {} });
t(() => class { @(ret({})) f; });
t(() => class { @(ret(1)) accessor a; });
t(() => class { @(ret({ init: 2 })) accessor a; });
t(() => @(ret(null)) class {});
t(() => class { @((v, ctx) => ctx.addInitializer("x")) static m() {} });
let late;
class L { @((v, ctx) => { late = ctx; }) #m() {} }
t(() => late.addInitializer(() => {}));
t(() => late.access.get({}));
t(() => late.access.get(1));
t(() => class { @((v, ctx) => { throw new RangeError("from " + ctx.name); }) static x; });
t(() => class C { @(C) m() {} });

// `accessor` as a name, and auto-accessors without decorators.
class A {
  accessor;
  accessor
  x = 1;
  static accessor() {
    return "method";
  }
  accessor #y = 2;
  static accessor z = 3;
  get y() {
    return this.#y;
  }
}
t(() => [Object.keys(new A()).join(), A.accessor(), new A().y, A.z].join());
t(() => Object.getOwnPropertyNames(A.prototype).join());
t(() => Object.getOwnPropertyDescriptor(A, "z").get.call({}));
