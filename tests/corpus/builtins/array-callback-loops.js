// The callback builtins' dense loops (0137, %EachDense): one callback frame entered again
// per element must behave exactly like a call per element, and hand over to the spec
// loop wherever the array stops being plain.
"use strict";
const a = [3, 1, 4, 1, 5, 9, 2, 6];
// Fresh locals and closures per element.
const fs = a.map((x, i) => { let y = x * 10; return () => y + i; });
console.log(fs.map((f) => f()).join(","));
// Default parameters, `arguments`, rest parameters and extra parameters.
console.log(a.map(function (x, i, arr, extra = "d") { return arguments.length + extra; }).join(""));
console.log(a.filter((x, ...rest) => rest[1] === a && rest[0] % 2 === 0).join(","));
console.log(a.map((x, i, arr, more) => more === undefined).every((b) => b));
// Parameters assigned inside the callback.
console.log(a.filter((x) => { x = 0; return true; }).join(","));
console.log(a.reduce((s, x) => { x = s; return s + 1; }, 0));
// Nested and recursive use.
console.log(JSON.stringify([[1, 2], [3]].map((r) => r.map((x) => [x].map((y) => y * 2)[0]))));
console.log([1, 2, 3].reduce((s, x) => s + [x, x].reduce((t, y) => t + y), 0));
// thisArg; sloppy and bound callbacks and proxies take the spec loop.
console.log([1, 2].map(function () { return this.k; }, { k: 7 }).join(","));
const sloppy = Function("x", "return typeof this");
console.log([1, 2].map(sloppy).join(","), [1].map(sloppy, 5).join(","));
console.log([1, 2, 3].map(Math.max.bind(null, 2)).join(","));
const p = new Proxy([1, 2, 3], {});
console.log(p.map((x) => x * 3).join(","), p.filter((x) => x > 1).length);
// Mutation while looping: growth is not visited, shrinking and holes stop the dense part.
const g = [1, 2, 3];
console.log(g.map((x) => { g.push(x); return x; }).join(","), g.length);
const s = [1, 2, 3, 4, 5];
console.log(JSON.stringify(s.map((x, i) => { if (i === 1) s.length = 3; return x; })));
const h = [1, 2, 3, 4];
let seen = [];
h.forEach((x, i) => { if (i === 0) delete h[2]; seen.push(x); });
console.log(seen.join(","));
const q = [1, 2, 3];
console.log(q.find((x, i) => { q[i + 1] = 0; return i === 2; }), q.findIndex((x) => x === 0));
// Early exits and the -1 / undefined results.
console.log([5, 6, 7].some((x) => x === 6), [5, 6, 7].every((x) => x < 7), [].every(() => false), [].some(() => true));
console.log([5, 6].find((x) => x > 9), [5, 6].findIndex((x) => x > 9), [5, , 7].find((x) => x === undefined));
// reduce with and without an initial value, over holes, and its errors.
console.log([, 2, , 4].reduce((s, x) => s + x), [1, 2].reduce((s, x) => s + x, 10), [7].reduce((s, x) => s + x));
try { [, ,].reduce((s, x) => s); } catch (e) { console.log(e.constructor.name); }
// Throws from the callback, at the first, middle and last element.
for (const at of [0, 3, 7]) {
  try { a.map((x, i) => { if (i === at) throw new RangeError("at " + i); return x; }); } catch (e) { console.log(e.message); }
}
// Generators and async callbacks (their frames are not reused).
console.log(a.map(function* (x) { yield x; }).map((it) => it.next().value).join(","));
const ps = [1, 2, 3].map(async (x) => x * 2);
Promise.all(ps).then((v) => console.log("async", v.join(",")));
// Species: a subclass gets its own results; filter into a non-plain species.
class Sub extends Array {}
const sub = Sub.from([1, 2, 3, 4]);
const m = sub.map((x) => x * 2);
const f = sub.filter((x) => x % 2 === 0);
console.log(m instanceof Sub, m.join(","), f instanceof Sub, f.join(","));
const leak = [1, 2, 3];
let leaked;
leak.constructor = { [Symbol.species]: function (n) { leaked = new Array(n); return leaked; } };
console.log(leak.filter((x) => { if (x === 2) leaked.push("x"); return true; }).join(","));
// Callback errors.
try { [1].forEach(null); } catch (e) { console.log(e.constructor.name); }
try { [1].reduce(); } catch (e) { console.log(e.constructor.name); }
