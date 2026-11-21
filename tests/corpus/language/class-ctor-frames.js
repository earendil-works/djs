// Class constructors without `check_ctor` and with their own results, super() from the
// frame, `this` checks left out after a first unconditional super(), class fields
// through `define_field` (0270): the cases where each shortcut must not apply.
class A { constructor(x) { this.x = x; } }
class R { constructor(x) { if (x) return { r: x }; return 5; } }
class R2 { constructor() { try { return; } finally { this.f = 1; } } }
class D extends A {}
class E extends A { constructor(a) { super(a); this.y = a; } }
class F extends A { constructor() { return {z: 1}; } }
class G extends A { constructor() { super(1); return undefined; } }
class H extends A { constructor() { super(1); return 3; } }
console.log(JSON.stringify([new A(1), new R(0), new R(7), new R2(), new D(2), new E(3), new F(), new G()]));
try { A.call({}); } catch (e) { console.log(e.message); }
try { [1].map(A); } catch (e) { console.log(e.message); }
try { new H(); } catch (e) { console.log(e.message); }
console.log(Reflect.construct(A, [4]).x, new (class extends E {})(9).y);
class P { constructor(...a) { this.a = a; } }
class Q extends P {}
console.log(JSON.stringify([new Q(), new Q(1, 2, 3), new (class extends Q { constructor(x) { super(x, { cause: x }, [x, 1], "s", null); } })(5)]));
let evaluated = false, caught;
class S extends Object { constructor() { try { super(evaluated = true); } catch (e) { caught = e; } } }
Object.setPrototypeOf(S, parseInt);
try { new S(); } catch (_) {}
console.log(typeof caught, caught && caught.constructor === TypeError, evaluated);
class T extends A { constructor() { super(1); super(2); } }
try { new T(); } catch (e) { console.log(e.message); }
class U extends A { constructor() { const f = () => super(3); f(); this.q = 1; } }
console.log(JSON.stringify(new U()));
class V extends A { constructor(c) { if (c) super(4); this.q = 1; } }
try { new V(false); } catch (e) { console.log(e.message); }
console.log(JSON.stringify(new V(true)));
class W extends A { constructor() { return; super(1); } }
try { new W(); } catch (e) { console.log(e.message); }
class X extends A { constructor() { super(1); const g = () => this.x; this.g = g(); } }
console.log(JSON.stringify(new X()));
class Y extends A { constructor() { let o = 1; super(o); this.o = o; } }
console.log(JSON.stringify(new Y()));
class Z extends A { constructor() { for (let i = 0; i < 2; i++) super(i); } }
try { new Z(); } catch (e) { console.log(e.message); }
class N extends null { constructor() { super(); } }
try { new N(); } catch (e) { console.log(e.message); }
class M extends A { constructor() { super(Z0); } }
try { new M(); } catch (e) { console.log(e.message); }
function Pf(x) { this.p = x; } class Pg extends Pf { constructor() { super(7); this.k = new.target === Pg; } }
console.log(JSON.stringify(new Pg()), JSON.stringify(Reflect.construct(Pg, [], Object)));
class Arr extends Array {} const arr = new Arr(1, 2, 3); console.log(arr.length, arr instanceof Arr, Array.isArray(arr));
class Er extends Error { constructor(m) { super(m); this.name = "Er"; } } const er = new Er("boom"); console.log(String(er), er instanceof Er, er instanceof Error);
class Mp extends Map {} console.log(new Mp([[1,2]]).get(1));
class Pr extends Promise {} console.log(new Pr(r => r(1)) instanceof Pr);
{
class A2 { x = 1; y; "s t" = 2; 3 = 4; [Symbol.iterator] = 5; __proto__ = 6; constructor(z) { this.z = z; } }
const a = new A2(7); console.log(JSON.stringify(a), Object.keys(a).join(), Object.getPrototypeOf(a) === A2.prototype, a.__proto__);
let log = [];
const P = new Proxy({}, { defineProperty(t, k, d) { log.push(String(k)); return Reflect.defineProperty(t, k, d); } });
class Base { constructor() { return P; } }
class D extends Base { f = 1; g = 2; }
new D(); console.log(log.join());
class Fz { constructor() { return Object.freeze({}); } }
class E extends Fz { h = 1; }
try { new E(); } catch (e) { console.log(e.constructor.name, e.message); }
class Pe { constructor() { return Object.preventExtensions({}); } }
class G extends Pe { h = 1; }
try { new G(); } catch (e) { console.log(e.constructor.name, e.message); }
const deny = new Proxy({}, { defineProperty() { return false; } });
class Bd { constructor() { return deny; } }
class H extends Bd { k = 1; }
try { new H(); } catch (e) { console.log(e.constructor.name); }
class S { static s = 1; static t = S.s + 1; }
console.log(S.s, S.t);
for (let i = 0; i < 3; i++) { const o = new A2(i); if (o.x !== 1) console.log("bad"); }
}
{
try { A(); } catch (e) { console.log(e.constructor.name, e.message); }
class Cond extends A { constructor(c) { const v = c ? 1 : 2; super(v); this.v = v; } }
console.log(JSON.stringify(new Cond(true)), JSON.stringify(new Cond(false)));
class Late extends A { constructor() { const f = () => this; try { f(); } catch (e) { console.log("arrow", e.constructor.name); } super(9); console.log(f().x); } }
new Late();
class Twice extends A { constructor() { super(1); try { super(2); } catch (e) { console.log("twice", e.message); } this.t = 1; } }
console.log(JSON.stringify(new Twice()));
class Getter extends A { constructor(o) { super(o.v); } }
console.log(new Getter({ get v() { return 11; } }).x);
class Swap extends A { constructor(o) { super(o.v); } }
const other = class { constructor(x) { this.other = x; } };
console.log(JSON.stringify(new Swap({ get v() { Object.setPrototypeOf(Swap, other); return 12; } })));
console.log(JSON.stringify(new Swap({ v: 13 })));
class TP { x = this.constructor.name; constructor() {} }
class TQ extends TP { y = this.x + "!"; }
console.log(JSON.stringify(new TQ()));
}
