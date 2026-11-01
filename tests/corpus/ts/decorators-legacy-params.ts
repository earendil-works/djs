// difftest: emit-decorator-metadata
// emitDecoratorMetadata (0065): the parameter and return types of constructors and
// methods, recorded by a Reflect.metadata stand-in.
const R = Reflect as any;
const store = new Map<any, any[]>();
R.metadata = (k: string, v: unknown) => (t: any, key: any) => {
  let l = store.get(t); if (!l) store.set(t, l = []);
  l.push(String(key) + " " + k + "=" + (Array.isArray(v) ? "[" + v.map(n).join() + "]" : n(v)));
};
function n(v: any) { return typeof v === "function" ? v.name : String(v); }
const log: string[] = [];
const d = (...a: any[]) => { log.push("d " + a.length + " " + String(a[1]) + " " + typeof a[2]); };
enum E { A, B }
enum S { A = "a", B = "b" }
enum M { A = 1, B = "b" }
const sym = Symbol("s");
class Dep {}
namespace N { export namespace In { export class X {} } }
type T1 = string;
interface I1 {}
@d
class C {
  constructor(a: number, b?: string, c = 1, { x }: { x: number } = { x: 1 }, [y]: [number] = [1], ...rest: Dep[]) {}
  @d m1(this: C, a: Dep, ...r: string[]): void {}
  @d m2(...r: Array<string>) {}
  @d async m3() {}
  @d *m4() {}
  @d async *m5() {}
  @d m6(): Promise<number> { return null as any; }
  @d get g1(): number { return 1; }
  @d set g1(v) {}
  @d get g2() { return 1; }
  @d set g3(v: Dep) {}
  @d static sm(a: string) {}
  @d [sym](a: string) {}
  @d m7(x: asserts): x is string { return true; }
}
class Dep2 {}
for (const [t, l] of store) console.log(t === C ? "C" : t === C.prototype ? "C.prototype" : "?", l.join("\n  "));
console.log(log.join("\n"));
