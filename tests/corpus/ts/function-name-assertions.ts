// NamedEvaluation through TypeScript assertions (0108, D2070): tsc erases them, so the
// function is still the whole initializer; a call or member after it still is not.
const r: string[] = [];
const t1 = (function () {}) as any; r.push(t1.name);
const t2 = function () {} as any; r.push(t2.name);
const t3 = (() => 0)!; r.push(t3.name);
const t4 = (class {}) satisfies object; r.push((t4 as any).name);
const t5 = <any>(function () {}); r.push(t5.name);
const t6 = (() => 0) as () => number; r.push(t6.name);
let t7: any; t7 = function () {} as any; r.push(t7.name);
const t8 = ((function () {}) as any).name; r.push(t8);
const t9 = (<any>(() => () => 0))(); r.push(t9.name);
const t10 = (function () {} as any)!; r.push(t10.name);
console.log(JSON.stringify(r));
