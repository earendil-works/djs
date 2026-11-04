"djs-plain 1";
globalThis.$dump = { R: Reflect.apply,
  G: Reflect.get,
  C: Reflect.construct,
  D: (o, k, v) => Object.defineProperty(o, k, { value: v, writable: true, enumerable: true, configurable: true }),
  A: Object.assign,
  L: (it, next) => { let first = true; const r = { next() { if (first) { first = false; return { value: undefined, done: false }; } return Reflect.apply(next ?? it.next, it, []); }, return(v) { const m = it.return; return m == null ? { done: true, value: v } : Reflect.apply(m, it, [v]); }, [Symbol.iterator]() { return r; } }; return r; },
  Q: (a, ...v) => { let n = a.length; for (const x of v) Object.defineProperty(a, n++, { value: x, writable: true, enumerable: true, configurable: true }); return a; } };
$dump.s2 = {};
async function h() {
  const get = () => late;
  let early;
  try {
    get();
  } catch (e) {
    early = e.name;
  }
  const v = await later(1);
  let late = v * 2;
  return early + " " + get();
}
$dump.p1 = later(1);
$dump.Ka3 = async function () {
let early = "ReferenceError";
Object.assign($dump.s2, { get: () => late });
const get = $dump.s2.get;
const v = (await $dump.p1);
let late = v * 2;
return early + " " + get();
};
Object.defineProperty($dump.Ka3, "$djs", { value: { f: h, at: "await#1", segs: [128,127,1,150,149,17,168,169,27], scaf: [["site",131,133]], temps: [], cmap: [] } });
$dump.a3 = $dump.Ka3.call(undefined);
$dump.p4 = Promise.prototype.then.call($dump.a3, print, undefined);
try { $dump.p4 } finally { delete globalThis.$dump; }
