// put_elem appends and fills holes on the fast path only where that is unobservable;
// strict equality and insert2/insert3 on the fast path (D1783).
const a = [];
for (let i = 0; i < 20; i++) a[i] = i * 2;
console.log(a.length, a[19], a.join(","));
const h = [1, , 3];
h[1] = 2;
console.log(h.length, h.join(","), 1 in h);
const g = [];
g[5] = "x";
console.log(g.length, 0 in g, g[5]);
const frozen = Object.freeze([1, 2]);
try { (function () { "use strict"; frozen[2] = 3; })(); } catch (e) { console.log(e.constructor.name); }
console.log(frozen.length);
const ne = Object.preventExtensions([1]);
try { ne[1] = 2; } catch (e) { console.log(e.constructor.name); }
console.log(ne.length, 1 in ne);
let seen = [];
Object.defineProperty(Array.prototype, "3", { set(v) { seen.push(v); }, configurable: true });
const p = [0, 1, 2];
p[3] = "via setter";
console.log(p.length, seen.join(","));
delete Array.prototype[3];
const ro = [1];
Object.defineProperty(ro, "length", { writable: false });
try { ro[1] = 5; } catch (e) { console.log(e.constructor.name); }
console.log(ro.length, ro[1]);
const ta = new Int8Array(4);
ta[2] = 300;
try { ta[10] = 1; ta[-1] = 1; } catch (e) { console.log(e.constructor.name); }
console.log(ta.join(","), ta.length, ta[10]);
let o = {};
let r = (o.x = 5);
let arr = [];
let k = 0;
let r2 = (arr[k++] = 7);
console.log(r, r2, k, arr[0]);
console.log(1 === 1, 1 === 1.0, 0 === -0, NaN === NaN, NaN !== NaN, 2 !== 3, 1.5 === 1.5, "a" === "a", 1 === "1", null === undefined, 2 ** 31 === 2147483648);
