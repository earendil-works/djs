// Generators: yield, yield*, return/throw resumption, finally, iterator closing (0041).
function S(v) {
  if (Array.isArray(v)) return "[" + v.map(S).join(",") + "]";
  if (v && typeof v === "object") return "{" + Object.keys(v).map((k) => k + ":" + S(v[k])).join(",") + "}";
  return typeof v === "string" ? '"' + v + '"' : String(v);
}
function* g(a) {
  const x = yield a;
  console.log("got", x);
  try {
    yield 2;
    yield 3;
  } finally {
    console.log("fin");
  }
  return 4;
}
let it = g(1);
console.log(S([it.next(), it.next("X"), it.next(), it.return(9), it.next()]));
it = g(1);
it.next();
it.next();
try {
  it.throw(new Error("boom"));
} catch (e) {
  console.log("caught", e.message);
}
console.log([...g(5)].join());
function* inner() {
  const r = yield 1;
  console.log("inner got", r);
  yield 2;
  return "iv";
}
function* outer() {
  const v = yield* inner();
  console.log("v", v);
  yield* [7, 8];
  return "done";
}
console.log(S([...outer()]));
it = outer();
it.next();
console.log(S(it.next("R")));
console.log(S(it.return("early")));
function* fin() {
  try {
    yield* inner();
  } finally {
    console.log("outer fin");
  }
}
it = fin();
it.next();
console.log(S(it.return(5)));
// yield inside expressions with operands on the stack
function* expr() {
  const r = 1 + (yield 10) * 2 + [yield 20, yield 30].length;
  return r;
}
it = expr();
console.log(S([it.next(), it.next(3), it.next(), it.next(), it.next()]));
it = expr();
it.next();
console.log(S(it.return(42)));
const o = { *m() { yield this.v; }, v: 7 };
console.log([...o.m()][0], typeof g.prototype, Object.getPrototypeOf(g) === Object.getPrototypeOf(function* () {}), g.name);
// yield* passes the inner result objects through
const resObj = { done: false, value: 1, extra: true };
const raw = { [Symbol.iterator]() { return this; }, next() { return resObj; } };
function* pass() { yield* raw; }
console.log(pass().next() === resObj);
function* catcher() {
  try {
    yield 1;
  } catch (e) {
    console.log("inner caught", e);
    yield 99;
  }
}
function* fw() { yield* catcher(); }
it = fw();
it.next();
console.log(S(it.throw("E")));
const nothrow = {
  [Symbol.iterator]() { return this; },
  next() { return { done: false, value: 1 }; },
  return() { console.log("closed"); return {}; },
};
function* nt() { yield* nothrow; }
it = nt();
it.next();
try {
  it.throw(1);
} catch (e) {
  console.log(e.message);
}
function* cnt() {
  try {
    for (let i = 0; ; i++) yield i;
  } finally {
    console.log("cnt closed");
  }
}
for (const v of cnt()) {
  if (v == 2) break;
  console.log(v);
}
function* fo() { for (const x of [1, 2, 3]) yield x; }
it = fo();
it.next();
console.log(S(it.return(0)));
// A return while destructuring closes the pattern's iterator.
const closing = {
  [Symbol.iterator]() { return { next() { return { done: false }; }, return() { console.log("pattern closed"); return {}; } }; },
};
function* dstr() { const [a = yield "in default"] = closing; return a; }
it = dstr();
console.log(S(it.next()), S(it.return("r")));
// Errors in parameters throw at the call, before the generator exists.
function* bad(a = null.x) {}
try {
  bad();
} catch (e) {
  console.log("param error", e.constructor.name);
}
