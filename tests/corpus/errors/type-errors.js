// Messages of common TypeErrors, printed as "Name: message".
function attempt(label, fn) {
  try {
    fn();
    console.log(label, "-> no error");
  } catch (e) {
    console.log(label, "->", e);
  }
}
const u = undefined;
const n = null;
attempt("property of undefined", () => u.x);
attempt("property of null", () => n.x);
attempt("nested property", () => ({}).a.b);
attempt("set property of undefined", () => { u.x = 1; });
attempt("call undefined", () => u());
attempt("call non-function", () => (1)());
attempt("call missing method", () => ({}).missing());
attempt("call property of string", () => "s".nope());
attempt("new non-constructor", () => new (() => {})());
attempt("new number", () => new (5)());
attempt("assign to const", () => { const c = 1; c = 2; });
attempt("in on primitive", () => "a" in "abc");
attempt("instanceof non-callable", () => ({}) instanceof {});
attempt("frozen object", () => { "use strict"; Object.freeze({ a: 1 }).a = 2; });
attempt("non-writable", () => { const o = {}; Object.defineProperty(o, "p", { value: 1 }); o.p = 2; });
attempt("getter only", () => { const o = { get g() { return 1; } }; o.g = 2; });
attempt("non-extensible", () => { const o = Object.preventExtensions({}); o.x = 1; });
attempt("delete non-configurable", () => { delete Object.prototype; });
attempt("define on non-extensible", () => Object.defineProperty(Object.preventExtensions({}), "x", { value: 1 }));
attempt("symbol to string", () => "" + Symbol("s"));
attempt("symbol to number", () => +Symbol("s"));
attempt("bigint mix", () => 1n + 1);
attempt("bigint to number", () => +1n);
attempt("class without new", () => { class K {} K(); });
attempt("super property before super()", () => { class A {} class B extends A { constructor() { this.x = 1; super(); } } new B(); });
attempt("not iterable", () => { const [a] = {}; return a; });
attempt("spread non-iterable", () => [...123]);
attempt("for-of non-iterable", () => { for (const x of 5) {} });
attempt("destructure undefined", () => { const { a } = undefined; return a; });
attempt("destructure null", () => { const { a } = null; return a; });
attempt("Object.defineProperty on primitive", () => Object.defineProperty(1, "x", {}));
attempt("Object.create bad proto", () => Object.create(5));
attempt("setPrototypeOf cycle", () => { const a = {}; const b = Object.create(a); Object.setPrototypeOf(a, b); });
attempt("Array.prototype.map bad callback", () => [1].map(5));
attempt("reduce empty", () => [].reduce((a, b) => a + b));
attempt("JSON.stringify cycle", () => { const o = {}; o.o = o; JSON.stringify(o); });
attempt("JSON.stringify bigint", () => JSON.stringify({ n: 1n }));
attempt("private field on wrong object", () => { class P { #x = 1; static get(o) { return o.#x; } } P.get({}); });
attempt("String.prototype.repeat on null", () => String.prototype.repeat.call(null, 1));
attempt("Function.prototype.call on non-function", () => Function.prototype.call.call({}));
