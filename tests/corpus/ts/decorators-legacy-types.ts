// difftest: emit-decorator-metadata
// emitDecoratorMetadata (0065): the serialized types of annotations (tsc's
// serializeTypeNode), recorded by a Reflect.metadata stand-in.
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
class C {
  @d a: string | undefined;
  @d b: 1 | 2;
  @d c: "x";
  @d e: `t${string}`;
  @d f: true | false;
  @d g: bigint;
  @d h: symbol;
  @d i: E;
  @d j: S;
  @d k: M;
  @d l: [number, string];
  @d m: Array<string>;
  @d n: Map<string, number>;
  @d o: N.In.X;
  @d p: T1;
  @d q: I1;
  @d r: never;
  @d s: unknown;
  @d t: any;
  @d u: object;
  @d v: { a: number };
  @d w: keyof Dep;
  @d x: typeof sym;
  @d y: string[][];
  @d z: Dep & I1;
  @d aa: (string | number)[];
  @d ab: string extends T1 ? number : number;
  @d ac: new () => Dep;
  @d ad: null;
  @d ae: void;
  @d af: Dep[];
  @d ag: -1;
  @d ah: 1n;
  @d ai: Dep | Dep;
  @d aj: Dep | Dep2;
  @d ak: never | string;
  @d al: unknown & string;
  @d am: Function;
  @d an: readonly string[];
  @d [Symbol.iterator]: number;
  @d ["comp" + 1]: number;
  @d static st: number;
  @d accessor acc: number = 1;
}
class Dep2 {}
for (const [t, l] of store) console.log(t === C ? "C" : t === C.prototype ? "C.prototype" : "?", l.join("\n  "));
console.log(log.join("\n"));
