// Prototype property caches (0062, D1621): a site keeps seeing every change.
function t(f) {
  try {
    console.log(f());
  } catch (e) {
    console.log(e);
  }
}
function getM(o) { return o.m; }
function callF(o) { return o.f(); }
const A = { m: "A.m", f() { return "A.f"; } };
const B = Object.create(A);
const C = Object.create(B);
const x = Object.create(C);
const y = Object.create(B);
for (let i = 0; i < 3; i++) console.log(getM(x), getM(y), callF(x), callF(y));
B.m = "B.m";
console.log(getM(x), getM(y));
x.m = "own x.m";
console.log(getM(x), getM(y));
delete x.m;
delete B.m;
console.log(getM(x), getM(y));
A.m = "A.m changed";
console.log(getM(x), callF(x));
Object.defineProperty(A, "m", { get() { return "A getter"; }, configurable: true });
console.log(getM(x));
Object.defineProperty(A, "m", { value: "A.m data again", writable: true, configurable: true });
console.log(getM(x));
Object.setPrototypeOf(C, { m: "new C proto" });
console.log(getM(x), getM(y));
Object.setPrototypeOf(x, A);
console.log(getM(x));
delete A.m;
console.log(getM(x), getM(y));
// Classes.
class Base { hello() { return "base"; } }
class Derived extends Base {}
const ds = [new Derived(), new Derived(), new Base()];
function hello(o) { return o.hello(); }
console.log(ds.map(hello).join());
Derived.prototype.hello = function () { return "derived"; };
console.log(ds.map(hello).join());
ds[0].hello = () => "instance";
console.log(ds.map(hello).join());
// Primitives.
function upper(s) { return s.toUpperCase(); }
function len(s) { return s.length; }
function fixed(n) { return n.toFixed(1); }
console.log(upper("ab"), upper("cd"), len("xyz"), fixed(1.25), fixed(2));
String.prototype.toUpperCase = function () { return "patched " + this; };
console.log(upper("ef"));
Object.prototype.extra = "from Object.prototype";
function getExtra(o) { return o.extra; }
console.log(getExtra({}), getExtra("s"), getExtra(1), getExtra([]));
Number.prototype.extra = "from Number.prototype";
console.log(getExtra(5), getExtra({}));
delete Object.prototype.extra;
delete Number.prototype.extra;
console.log(getExtra(5), getExtra({}));
// Typed arrays answer canonical numeric keys themselves.
function get15(o) { return o["1.5"]; }
const P = { "1.5": "from P" };
const plain = Object.create(P);
const ta = new Uint8Array(2);
Object.setPrototypeOf(ta, P);
console.log(get15(plain), get15(plain), get15(ta));
// Proxies on the chain.
const prox = Object.create(new Proxy({ m: "proxy target" }, { get: (tt, k) => "trap " + String(k) }));
console.log(getM(prox), getM(prox));
t(() => getM(null));
