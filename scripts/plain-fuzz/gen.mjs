// gen.mjs: a seeded generator of small codemode-like async programs for the plain-dump
// fuzzer (0208, D2940). `generate(seed)` returns the body of pi's `async (tools,
// console) => {...}`: tool calls (`tools.echo`, `tools.fail`, each one idle point) awaited
// in every expression and statement position, around closures, classes, accessors,
// generators, async generators, `Promise.all` over mapped async functions and `using`.
//
// The same seed gives the same program. Values stay small and typed (numbers, strings,
// arrays of numbers, objects with the number fields `a` and `b`), so programs run without
// TypeErrors and print what they compute; loops run at most three times, and the awaits
// are counted against a budget so that a program has a few dozen idle points at most.

// mulberry32: small, deterministic, good enough to pick shapes.
function rng(seed) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const TYPES = ["num", "str", "arr", "obj"];

class Gen {
  constructor(seed, opts) {
    this.r = rng(seed);
    this.opts = opts;
    this.out = [];
    this.indent = 0;
    this.n = 0;
    // The awaits written so far, weighted by the iterations of the loops around them.
    this.awaits = 0;
    this.scopes = [[]];
    // The function being written: async (await allowed), generator (yield allowed), the
    // loops and labels around the current statement, the loop multiplier.
    this.fn = { async: true, gen: false, loops: [], mult: 1, top: true };
  }

  // --- choices
  rand(n) {
    return Math.floor(this.r() * n);
  }
  chance(p) {
    return this.r() < p;
  }
  pick(xs) {
    return xs[this.rand(xs.length)];
  }
  // Picks one of [weight, fn] whose weight is positive and calls it.
  oneOf(cases) {
    const live = cases.filter((c) => c[0] > 0);
    let w = live.reduce((s, c) => s + c[0], 0) * this.r();
    for (const [p, f] of live) {
      if ((w -= p) < 0) return f();
    }
    return live[live.length - 1][1]();
  }
  fresh(p) {
    return `${p}${++this.n}`;
  }

  // --- scopes
  vars(pred) {
    return this.scopes.flat().filter(pred);
  }
  declare(name, type, extra = {}) {
    this.scopes[this.scopes.length - 1].push({ name, type, ...extra });
  }
  block(f) {
    this.scopes.push([]);
    this.indent++;
    try {
      f();
    } finally {
      this.indent--;
      this.scopes.pop();
    }
  }
  inFunction(fn, f) {
    const saved = [this.fn, this.scopes];
    this.fn = fn;
    // A function sees the variables around it, but not the loops (no break across it).
    this.scopes = [...this.scopes, []];
    try {
      return f();
    } finally {
      [this.fn, this.scopes] = saved;
    }
  }
  line(s) {
    this.out.push("  ".repeat(this.indent) + s);
  }

  // --- awaits
  // An expression in a callback that is not async (no await in it).
  sync(f) {
    const saved = this.fn;
    this.fn = { ...saved, async: false };
    try {
      return f();
    } finally {
      this.fn = saved;
    }
  }
  canAwait() {
    return this.fn.async && this.awaits + this.fn.mult <= this.opts.budget;
  }
  // A tool call; the caller wraps it in `await`.
  tool(arg) {
    this.awaits += this.fn.mult;
    return `tools.echo(${arg})`;
  }

  // --- expressions of a type, `d` levels deep at most
  expr(type, d = 2) {
    switch (type) {
      case "num":
        return this.num(d);
      case "str":
        return this.str(d);
      case "arr":
        return this.arr(d);
      case "obj":
        return this.obj(d);
      case "bool":
        return this.bool(d);
    }
    throw new Error(type);
  }
  varOf(type) {
    const vs = this.vars((v) => v.type === type);
    return vs.length ? this.pick(vs).name : null;
  }
  num(d) {
    const v = this.varOf("num");
    if (d <= 0) return v && this.chance(0.6) ? v : String(this.rand(10));
    const a = this.canAwait();
    const sub = () => this.num(d - 1);
    return this.oneOf([
      [3, () => String(this.rand(10))],
      [v ? 4 : 0, () => v],
      [a ? 6 : 0, () => `(await ${this.tool(sub())})`],
      [a ? 2 : 0, () => `(await ${this.tool(`{ v: ${sub()} }`)}).v`],
      [a ? 2 : 0, () => `(await ${this.tool(`[${sub()}, ${sub()}]`)})[1]`],
      [3, () => `(${sub()} + ${sub()})`],
      [1, () => `(${sub()} * 2 - ${sub()})`],
      [2, () => `(${this.bool(d - 1)} ? ${sub()} : ${sub()})`],
      [1, () => `(${sub()}, ${sub()})`],
      [1, () => `(${sub()} || ${sub()})`],
      [1, () => `(${sub()} && ${sub()})`],
      [1, () => `(${sub()} ?? ${sub()})`],
      [2, () => `${this.arr(d - 1)}.length`],
      [1, () => `${this.str(d - 1)}.length`],
      [2, () => `${this.obj(d - 1)}.${this.pick(["a", "b"])}`],
      [1, () => `${this.arr(d - 1)}.reduce((s, x) => s + x, ${sub()})`],
      [1, () => `Math.max(${sub()}, ${sub()})`],
      [1, () => `[${sub()}, ${sub()}][${this.rand(2)}]`],
      [2, () => this.callNum(d)],
      [a ? 1 : 0, () => `(await (async () => (${sub()}))())`],
      [a ? 1 : 0, () => `(await new Promise((res) => res(${sub()})))`],
    ]);
  }
  // A call of something in scope that gives a number.
  callNum(d) {
    const fs = this.vars((v) => v.type === "fn" || v.type === "afn" || v.type === "inst" || v.type === "gen" || v.type === "cls");
    const usable = fs.filter((f) => f.type !== "afn" || this.canAwait());
    if (!usable.length) return String(this.rand(10));
    const f = this.pick(usable);
    const arg = () => this.num(d - 1);
    switch (f.type) {
      case "fn":
        return `${f.name}(${arg()})`;
      case "afn":
        this.awaits += this.fn.mult * f.awaits;
        return `(await ${f.name}(${arg()}))`;
      case "inst":
        return this.oneOf([
          [2, () => `${f.name}.get(${arg()})`],
          [2, () => `${f.name}.twice`],
          [1, () => `(${f.name}.value = ${arg()})`],
          [
            this.canAwait() ? 2 : 0,
            () => {
              this.awaits += this.fn.mult;
              return `(await ${f.name}.fetch(${arg()}))`;
            },
          ],
        ]);
      case "cls":
        return this.chance(0.5) ? `new ${f.name}(${arg()}).twice` : `${f.name}.make(${arg()}).get(${arg()})`;
      case "gen":
        return this.oneOf([
          [1, () => `[...${f.name}(${this.rand(4)})].reduce((s, x) => s + x, 0)`],
          [1, () => `(${f.name}(${this.rand(3) + 1}).next().value)`],
        ]);
    }
  }
  str(d) {
    const v = this.varOf("str");
    const lit = () => JSON.stringify(this.pick(["a", "b", "x", "ok", "s1", ""]));
    if (d <= 0) return v && this.chance(0.6) ? v : lit();
    const a = this.canAwait();
    return this.oneOf([
      [3, lit],
      [v ? 4 : 0, () => v],
      [a ? 5 : 0, () => `(await ${this.tool(this.str(d - 1))})`],
      [3, () => "`${" + this.str(d - 1) + "}-${" + this.num(d - 1) + "}:${" + this.str(d - 1) + "}`"],
      [2, () => `(${this.str(d - 1)} + ${this.num(d - 1)})`],
      [1, () => `String(${this.num(d - 1)})`],
      [2, () => `${this.arr(d - 1)}.join("-")`],
      [1, () => `(${this.bool(d - 1)} ? ${this.str(d - 1)} : ${this.str(d - 1)})`],
      [1, () => `typeof ${this.num(d - 1)}`],
      [1, () => `Object.keys(${this.obj(d - 1)}).join("")`],
      [1, () => `JSON.stringify(${this.obj(d - 1)})`],
    ]);
  }
  arr(d) {
    const v = this.varOf("arr");
    if (d <= 0) return v && this.chance(0.6) ? v : `[${this.num(0)}, ${this.num(0)}]`;
    const a = this.canAwait();
    const sub = () => this.num(d - 1);
    return this.oneOf([
      [3, () => `[${sub()}, ${sub()}${this.chance(0.4) ? `, ${sub()}` : ""}]`],
      [v ? 4 : 0, () => v],
      [a ? 3 : 0, () => `(await ${this.tool(this.arr(d - 1))})`],
      [2, () => `[${sub()}, ...${this.arr(d - 1)}]`],
      [2, () => `${this.arr(d - 1)}.map((x) => x + ${this.sync(sub)})`],
      [1, () => `${this.arr(d - 1)}.filter((x) => x > ${this.sync(sub)})`],
      [1, () => `Object.values(${this.obj(d - 1)}).filter((x) => typeof x === "number")`],
      [
        a ? 3 : 0,
        () => {
          // Promise.all over a mapped async function: several calls open at once.
          const x = this.fresh("x");
          const src = this.arr(d - 1);
          const inner = this.inFunction({ async: true, gen: false, loops: [], mult: this.fn.mult * 3 }, () => {
            this.declare(x, "num");
            return this.oneOf([
              [2, () => `(await ${this.tool(`${x} + ${this.num(1)}`)})`],
              [1, () => `${x} * (await ${this.tool(this.num(1))})`],
            ]);
          });
          return `(await Promise.all(${src}.map(async (${x}) => ${inner})))`;
        },
      ],
      [this.vars((v) => v.type === "gen").length ? 1 : 0, () => `[...${this.pick(this.vars((v) => v.type === "gen")).name}(${this.rand(3) + 1})]`],
    ]);
  }
  obj(d) {
    const v = this.varOf("obj");
    if (d <= 0) return v && this.chance(0.6) ? v : `{ a: ${this.num(0)}, b: ${this.num(0)} }`;
    const a = this.canAwait();
    const sub = () => this.num(d - 1);
    return this.oneOf([
      [3, () => `{ a: ${sub()}, b: ${sub()} }`],
      [v ? 4 : 0, () => v],
      [a ? 3 : 0, () => `(await ${this.tool(this.obj(d - 1))})`],
      [2, () => `{ ...${this.obj(d - 1)}, b: ${sub()} }`],
      [1, () => `{ [${this.pick(['"a"', '"b"'])}]: ${sub()}, ${this.pick(["a", "b"])}: ${sub()}, a: ${sub()}, b: ${sub()} }`],
      [3, () => this.accessorObj(d)],
      [1, () => `{ a: ${sub()}, b: ${sub()}, m() { return this.a; } }`],
    ]);
  }
  // An object with accessors that capture a variable of the frame (the `get m() { n++ }`
  // shape: the closure lives in the frame's scope).
  accessorObj(d) {
    const muts = this.vars((v) => v.type === "num" && v.mut);
    const n = muts.length ? this.pick(muts).name : null;
    const sub = () => this.num(d - 1);
    if (!n) return `{ a: ${sub()}, get b() { return this.a + 1; } }`;
    return this.oneOf([
      [2, () => `{ a: ${sub()}, get b() { ${n}++; return ${n}; } }`],
      [1, () => `{ get a() { return ${n} + 1; }, set a(v) { ${n} = v; }, b: ${sub()} }`],
      [1, () => `{ a: ${sub()}, b: ${sub()}, get g() { return ${n} * 2; } }`],
    ]);
  }
  bool(d) {
    if (d <= 0) return this.pick(["true", "false", `${this.num(0)} > ${this.num(0)}`]);
    return this.oneOf([
      [2, () => `${this.num(d - 1)} > ${this.num(d - 1)}`],
      [1, () => `${this.num(d - 1)} === ${this.num(d - 1)}`],
      [1, () => `${this.str(d - 1)} !== ${this.str(d - 1)}`],
      [1, () => `!${this.bool(d - 1)}`],
      [1, () => `(${this.bool(d - 1)} && ${this.bool(d - 1)})`],
      [1, () => `(${this.bool(d - 1)} || ${this.bool(d - 1)})`],
      [1, () => `${this.arr(d - 1)}.includes(${this.num(d - 1)})`],
      [1, () => `"a" in ${this.obj(d - 1)}`],
    ]);
  }

  // --- statements
  print(e) {
    this.line(`console.log(${JSON.stringify(this.fresh("p"))}, JSON.stringify(${e}));`);
  }
  stmts(k, d) {
    for (let i = 0; i < k; i++) this.stmt(d);
  }
  stmt(d) {
    const a = this.canAwait();
    const loop = d > 0 && this.fn.loops.length < 2;
    this.oneOf([
      [4, () => this.declStmt(d)],
      [4, () => this.print(this.expr(this.pick(TYPES), 2))],
      [this.vars((v) => v.mut && v.type === "num").length ? 2 : 0, () => this.assignStmt()],
      [2, () => this.mutateStmt()],
      [a ? 2 : 0, () => this.destructStmt()],
      [d > 0 ? 2 : 0, () => this.ifStmt(d)],
      [loop ? 2 : 0, () => this.forStmt(d)],
      [loop ? 2 : 0, () => this.forOfStmt(d)],
      [loop ? 1 : 0, () => this.forInStmt(d)],
      [loop ? 1 : 0, () => this.whileStmt(d)],
      [loop && this.fn.top ? 1 : 0, () => this.labeledStmt(d)],
      [d > 0 ? 2 : 0, () => this.tryStmt(d)],
      [d > 0 ? 1 : 0, () => this.switchStmt(d)],
      [this.fn.top && d > 0 ? 2 : 0, () => this.asyncFnDecl()],
      [this.fn.top && d > 0 ? 1 : 0, () => this.closureDecl()],
      [this.fn.top && d > 0 ? 1 : 0, () => this.classDecl()],
      [this.fn.top && d > 0 ? 1 : 0, () => this.genDecl()],
      [this.fn.top && d > 0 && a ? 1 : 0, () => this.asyncGenStmt(d)],
      [this.opts.using && d > 0 ? 1 : 0, () => this.usingStmt(d)],
      [this.fn.loops.length ? 1 : 0, () => this.jumpStmt()],
    ]);
  }
  declStmt() {
    const type = this.pick(TYPES);
    const mut = this.chance(0.5);
    const name = this.fresh("v");
    const e = this.expr(type, 2);
    this.line(`${mut ? "let" : "const"} ${name} = ${e};`);
    this.declare(name, type, { mut });
    if (this.chance(0.3)) this.print(name);
  }
  assignStmt() {
    const v = this.pick(this.vars((v) => v.mut && v.type === "num")).name;
    this.line(`${v} ${this.pick(["=", "+=", "-="])} ${this.num(2)};`);
  }
  mutateStmt() {
    this.oneOf([
      [this.varOf("obj") ? 2 : 0, () => this.line(`${this.varOf("obj")}.${this.pick(["a", "b"])} = ${this.num(2)};`)],
      [this.varOf("arr") ? 2 : 0, () => this.line(`${this.varOf("arr")}.push(${this.num(2)});`)],
      [this.varOf("arr") ? 1 : 0, () => this.line(`${this.varOf("arr")}[${this.num(0)} % 2] = ${this.num(2)};`)],
      [this.varOf("obj") ? 1 : 0, () => this.line(`${this.varOf("obj")}[${this.str(1)} || "a"] = ${this.num(2)};`)],
      [1, () => this.line(`${this.str(1)};`)],
    ]);
  }
  destructStmt() {
    const [x, y, z] = [this.fresh("v"), this.fresh("v"), this.fresh("v")];
    this.oneOf([
      [
        2,
        () => {
          this.line(`const { a: ${x}, b: ${y} = ${this.num(1)} } = ${this.obj(2)};`);
          this.declare(x, "num");
          this.declare(y, "num");
        },
      ],
      [
        2,
        () => {
          this.line(`let [${x}, , ${y} = ${this.num(1)}] = ${this.arr(2)};`);
          this.declare(x, "num", { mut: true });
          this.declare(y, "num", { mut: true });
        },
      ],
      [
        1,
        () => {
          this.line(`const { a: ${x}, ...${z} } = await ${this.tool(`{ a: ${this.num(1)}, b: ${this.num(1)} }`)};`);
          this.declare(x, "num");
        },
      ],
      [
        1,
        () => {
          this.line(`const [${x} = ${this.num(1)}, ...${z}] = ${this.arr(2)};`);
          this.declare(x, "num");
          this.declare(z, "arr");
        },
      ],
      [
        1,
        () => {
          const o = this.varOf("obj");
          if (!o) return this.line(`;`);
          this.line(`({ a: ${o}.b = ${this.num(1)} } = ${this.obj(1)});`);
        },
      ],
    ]);
  }
  body(d, k = 1 + this.rand(3)) {
    this.block(() => this.stmts(k, d - 1));
  }
  ifStmt(d) {
    this.line(`if (${this.bool(2)}) {`);
    this.body(d);
    if (this.chance(0.5)) {
      this.line(`} else {`);
      this.body(d);
    }
    this.line(`}`);
  }
  // Loops push themselves (a label, whether `continue` is allowed) and multiply the await
  // weight by their iterations.
  loop(info, iters, f) {
    const saved = this.fn.mult;
    this.fn.loops.push(info);
    this.fn.mult *= iters;
    try {
      f();
    } finally {
      this.fn.loops.pop();
      this.fn.mult = saved;
    }
  }
  forStmt(d) {
    const i = this.fresh("i");
    const k = 1 + this.rand(3);
    this.line(`for (let ${i} = 0; ${i} < ${k}; ${i}++) {`);
    this.loop({ cont: true }, k, () =>
      this.block(() => {
        this.declare(i, "num");
        this.stmts(1 + this.rand(3), d - 1);
      }),
    );
    this.line(`}`);
  }
  forOfStmt(d) {
    const x = this.fresh("x");
    const src = this.oneOf([
      [3, () => `[${this.num(1)}, ${this.num(1)}]`],
      [this.canAwait() ? 2 : 0, () => `(await ${this.tool(`[${this.num(0)}, ${this.num(0)}]`)})`],
      [this.varOf("gen") ? 2 : 0, () => `${this.varOf("gen")}(2)`],
      [1, () => `[${this.num(1)}, ${this.num(1)}].entries().map(([k, x]) => k + x)`],
    ]);
    this.line(`for (const ${x} of ${src}) {`);
    this.loop({ cont: true }, 3, () =>
      this.block(() => {
        this.declare(x, "num");
        this.stmts(1 + this.rand(3), d - 1);
      }),
    );
    this.line(`}`);
  }
  forInStmt(d) {
    const k = this.fresh("k");
    this.line(`for (const ${k} in ${this.obj(1)}) {`);
    this.loop({ cont: true }, 2, () =>
      this.block(() => {
        this.declare(k, "str");
        this.stmts(1 + this.rand(2), d - 1);
      }),
    );
    this.line(`}`);
  }
  whileStmt(d) {
    const w = this.fresh("w");
    const k = 1 + this.rand(3);
    this.line(`let ${w} = 0;`);
    const doWhile = this.chance(0.4);
    this.line(doWhile ? `do {` : `while (${w} < ${k}) {`);
    this.loop({ cont: false }, k, () =>
      this.block(() => {
        this.line(`${w}++;`);
        this.stmts(1 + this.rand(3), d - 1);
      }),
    );
    this.line(doWhile ? `} while (${w} < ${k});` : `}`);
  }
  labeledStmt(d) {
    const l = this.fresh("L");
    const i = this.fresh("i");
    this.line(`${l}: for (const ${i} of [0, 1, 2]) {`);
    this.loop({ cont: true, label: l }, 3, () =>
      this.block(() => {
        this.declare(i, "num");
        this.stmts(1 + this.rand(2), d - 1);
        this.line(`if (${i} === ${this.rand(3)}) ${this.pick(["continue", "break"])} ${l};`);
        this.stmts(this.rand(2), d - 1);
      }),
    );
    this.line(`}`);
  }
  jumpStmt() {
    const loops = this.fn.loops;
    const inner = loops[loops.length - 1];
    const labeled = loops.filter((l) => l.label);
    const target = labeled.length && this.chance(0.5) ? this.pick(labeled) : null;
    const kind = (target ?? inner).cont && this.chance(0.5) ? "continue" : "break";
    this.line(`if (${this.bool(1)}) ${kind}${target ? " " + target.label : ""};`);
  }
  tryStmt(d) {
    // A try without catch may throw out of the program: one around it catches.
    const hasCatch = this.chance(0.8);
    if (!hasCatch) {
      this.line(`try {`);
      this.indent++;
    }
    this.line(`try {`);
    this.block(() => {
      this.stmts(this.rand(2), d - 1);
      if (this.canAwait() && this.chance(0.6)) {
        this.awaits += this.fn.mult;
        this.line(`await tools.fail({ message: ${this.str(1)} });`);
      } else if (this.chance(0.3)) {
        this.line(`throw new Error(${this.str(1)});`);
      }
      this.stmts(this.rand(2), d - 1);
    });
    if (hasCatch) {
      const e = this.fresh("e");
      this.line(`} catch (${e}) {`);
      this.block(() => {
        this.declare(e, "err");
        this.line(`console.log(${JSON.stringify(this.fresh("c"))}, ${e}.message);`);
        this.stmts(this.rand(2), d - 1);
      });
    }
    if (!hasCatch || this.chance(0.4)) {
      this.line(`} finally {`);
      this.body(d, 1 + this.rand(2));
    }
    this.line(`}`);
    if (!hasCatch) {
      this.indent--;
      this.line(`} catch (e) {`);
      this.line(`  console.log("outer", e.message);`);
      this.line(`}`);
    }
  }
  switchStmt(d) {
    this.line(`switch (${this.num(2)} % 3) {`);
    this.block(() => {
      const cases = this.rand(3) + 1;
      for (let c = 0; c < cases; c++) {
        this.line(`case ${this.chance(0.5) ? c : this.num(1)}: {`);
        this.loop({ cont: false, sw: true }, 1, () => this.body(d, 1 + this.rand(2)));
        this.line(this.chance(0.7) ? `  break;` : `  // falls through`);
        this.line(`}`);
      }
      if (this.chance(0.5)) {
        this.line(`default: {`);
        this.body(d, 1);
        this.line(`}`);
      }
    });
    this.line(`}`);
  }
  // An async helper: awaits inside, and maybe a variable of the frame it captures.
  asyncFnDecl() {
    const name = this.fresh("f");
    const x = this.fresh("a");
    const arrow = this.chance(0.4);
    // Its awaits count where it is called.
    const start = this.awaits;
    this.line(arrow ? `const ${name} = async (${x}) => {` : `async function ${name}(${x}) {`);
    this.inFunction({ async: true, gen: false, loops: [], mult: 1, top: false }, () =>
      this.block(() => {
        this.declare(x, "num");
        this.stmts(1 + this.rand(3), 2);
        this.line(`return ${this.num(2)};`);
      }),
    );
    this.line(arrow ? `};` : `}`);
    const own = this.awaits - start;
    this.awaits = start;
    this.declare(name, "afn", { awaits: Math.max(1, own) });
    if (this.canAwait() && this.chance(0.7)) this.print(this.callNum(2));
  }
  closureDecl() {
    const n = this.fresh("n");
    const f = this.fresh("f");
    this.line(`let ${n} = ${this.num(1)};`);
    this.declare(n, "num", { mut: true });
    this.line(
      this.pick([
        `const ${f} = (x) => (${n} += x);`,
        `const ${f} = (x) => { ${n}++; return ${n} + x; };`,
        `function ${f}(x) { return ${n} * 2 + x; }`,
      ]),
    );
    this.declare(f, "fn");
    if (this.chance(0.5)) this.print(`${f}(${this.num(1)})`);
  }
  classDecl() {
    const c = this.fresh("C");
    const caps = this.vars((v) => v.type === "num" && v.mut);
    const cap = caps.length ? this.pick(caps).name : "0";
    this.line(`class ${c} {`);
    this.line(`  constructor(v) { this.value = v; }`);
    this.line(`  get twice() { return this.value * 2; }`);
    this.line(`  get(x) { return this.value + x + ${cap}; }`);
    this.line(`  async fetch(x) { return this.value + (await tools.echo(x)); }`);
    this.line(`  static make(v) { return new ${c}(v + 1); }`);
    this.line(`}`);
    this.declare(c, "cls");
    const i = this.fresh("o");
    this.line(`const ${i} = new ${c}(${this.num(2)});`);
    this.declare(i, "inst");
  }
  genDecl() {
    const g = this.fresh("g");
    const caps = this.vars((v) => v.type === "num");
    const cap = caps.length ? this.pick(caps).name : "1";
    this.line(`function* ${g}(k) {`);
    this.line(`  for (let i = 0; i < k; i++) yield i + ${cap};`);
    this.line(`}`);
    this.declare(g, "gen");
  }
  // An async generator, consumed by `for await` with an await in the loop body: the
  // generator and the loop both wait across idle points.
  asyncGenStmt(d) {
    const g = this.fresh("ag");
    const x = this.fresh("x");
    this.line(`async function* ${g}(k) {`);
    this.line(`  for (let i = 0; i < k; i++) {`);
    this.line(`    yield await tools.echo(i * ${this.rand(5) + 1});`);
    this.line(`  }`);
    this.line(`}`);
    const k = 1 + this.rand(2);
    this.awaits += this.fn.mult * k;
    this.line(`for await (const ${x} of ${g}(${k})) {`);
    this.loop({ cont: true }, k, () =>
      this.block(() => {
        this.declare(x, "num");
        this.stmts(1 + this.rand(2), d - 1);
      }),
    );
    this.line(`}`);
  }
  usingStmt(d) {
    const r = this.fresh("r");
    const tag = JSON.stringify(this.fresh("u"));
    const asyncDispose = this.canAwait() && this.chance(0.5);
    this.line(`{`);
    this.block(() => {
      if (asyncDispose) {
        this.awaits += this.fn.mult;
        this.line(`await using ${r} = { async [Symbol.asyncDispose]() { console.log(${tag}, await tools.echo("bye")); } };`);
      } else {
        this.line(`using ${r} = { [Symbol.dispose]() { console.log(${tag}); } };`);
      }
      this.stmts(1 + this.rand(2), d - 1);
    });
    this.line(`}`);
  }

  program() {
    // A mutable counter first, so accessors and closures have something to capture.
    const n = this.fresh("n");
    this.line(`let ${n} = 0;`);
    this.declare(n, "num", { mut: true });
    this.stmts(this.opts.statements, this.opts.depth);
    const a = this.canAwait();
    // Everything the body declared that is a value, so that its state is compared.
    const seen = this.scopes[0].filter((v) => TYPES.includes(v.type)).map((v) => v.name);
    const fields = [
      `n: ${n}`,
      `vars: [${seen.join(", ")}]`,
      `calls: [${[1, 2, 3].map(() => this.num(1)).join(", ")}]`,
      `o: ${this.obj(2)}`,
    ];
    if (a) fields.push(`last: await ${this.tool(this.str(1))}`);
    this.line(`return { ${fields.join(", ")} };`);
    return this.out.join("\n") + "\n";
  }
}

export const DEFAULTS = { statements: 8, depth: 3, budget: 30, using: true };

export function generate(seed, opts = {}) {
  return new Gen(seed, { ...DEFAULTS, ...opts }).program();
}

// `node gen.mjs SEED`: prints the program.
if (import.meta.url === `file://${process.argv[1]}`) {
  process.stdout.write(generate(Number(process.argv[2] ?? 1)));
}
