// difftest: experimental-decorators
// TypeScript legacy decorators without metadata (0065): tsc's __decorate semantics.
// (qjs ignores a parenthesized class decorator `@(...) class`, so none are used here.)
const log: string[] = [];
const R = Reflect as any;
// Reflect.metadata is not used without emitDecoratorMetadata.
R.metadata = () => { log.push("metadata called"); return () => {}; };
const d = (name: string) => (...a: any[]) => {
  const desc = a[2] && typeof a[2] === "object" ? Object.keys(a[2]).join("/") : String(a[2]);
  log.push(name + ":" + a.length + ":" + String(a[1]) + ":" + desc);
};
{
  const k = (t: any, key: string) => { log.push("k " + key); };
  class C { @k x: string; @k m(a: number) {} }
  log.push(Object.getOwnPropertyNames(C.prototype).join());
}
// A class decorator that runs before the class is declared hits the TDZ of a later class.
try {
  @d("x") class A { @d("f") x: number; static y = Later; }
} catch (e) { log.push("err " + (e as Error).constructor.name); }
class Later {}
// Class expressions.
const E1 = @d("ce") class { @d("m") m() {} };
log.push("E1 name " + E1.name);
const sub = (c: any) => class extends c { static tag = 1 };
const E2 = @sub class Named { static who() { return Named; } };
log.push("E2 " + E2.name + " " + (E2 as any).tag + " " + ((E2 as any).who() === E2));
// A class decorator replaces the class; the declaration's binding holds the
// undecorated class while the members are decorated.
let seen: any;
const repl = (c: any) => { log.push("outer during: " + (seen === Box2)); return class X extends c {}; };
const mem = (t: any, k: string) => { seen = Box2; };
@repl class Box2 { @mem m() {} static self() { return Box2; } }
log.push("Box2 " + Box2.name + " " + (Box2.self() === Box2) + " " + (seen !== Box2) + " " + seen.name);
// `this` in decorator expressions is the enclosing function's.
const self = { tag: "outer" };
function f(this: any) {
  @d(this.tag) class Q { @d(this.tag + "m") m() {} }
}
f.call(self);
// A field decorator's descriptor is defined on the prototype.
class F { @((t: any, k: string) => ({ value: 42, writable: true, enumerable: true, configurable: true })) z: number; }
log.push("F.z " + (F.prototype as any).z + " own " + Object.hasOwn(new F(), "z"));
// Reflect.decorate replaces the application.
R.decorate = (decs: any[], t: any, k?: any, desc?: any) => {
  log.push("decorate " + decs.length + " " + String(k) + " " + typeof desc);
  return k === undefined ? t : desc;
};
@d("c") class G { @d("m") m(@d("p") a: string) {} @d("f") f = 1; constructor(@d("cp") x: number) {} }
delete R.decorate;
// Falsy decorators are skipped; truthy results replace the descriptor.
const ro = (t: any, k: string, desc: PropertyDescriptor) => ({ ...desc, writable: false });
class H { @(null as any) @d("ok") @ro m() {} }
log.push("H.m writable " + Object.getOwnPropertyDescriptor(H.prototype, "m")!.writable);
// Static members, auto-accessors, getters and setters, computed and symbol keys.
const sym = Symbol("s");
const key = "dyn";
class L {
  @d("sacc") static accessor sa = 1;
  @d("acc") accessor a = 2;
  @d("sg") static get sg() { return 1; }
  @d("set") set s(v: number) {}
  @d("sym") [sym]() {}
  @d("dyn") [key + 1](@d("dynp") x: number) {}
  @d("cf") [key + 2] = 1;
  #p(@d("priv") x: number) {}
}
log.push("L " + L.sa + " " + new L().a);
// Decorators of TypeScript members without code are not evaluated.
abstract class M {
  @d("decl") declare x: number;
  @d("abs") abstract m(): void;
  @d("ov") o(): void;
  o() {}
}
// Parameter decorators, in the order tsc applies them.
class P {
  constructor(@d("c0") a: number, @d("c1") @d("c1b") b: number) {}
  m(@d("m0") a: number, b: number, @d("m2") c: number) {}
  static s(@d("s0") a: number) {}
}
log.forEach((l) => console.log(l));
