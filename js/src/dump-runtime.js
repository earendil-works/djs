// The plain dump runtime (D2834, D2903): the helpers a plain dump calls as `$djs.NAME`.
// To run a dump outside djs, set `globalThis.$djs` to these and the host's hooks
// (`seed`, `resume`; see docs/plain-js-dumps.md, "The host interface"), then run the
// dump as a Script. Generated from crates/djs-core/src/vm/dump_runtime.rs:
//   DJS_UPDATE_DUMP_RUNTIME=1 cargo test -p djs-core dump_runtime

const matchAll = RegExp.prototype[Symbol.matchAll];
export const R = Reflect.apply;
export const G = Reflect.get;
export const C = Reflect.construct;
export const D = (o, k, v) => Object.defineProperty(o, k, { value: v, writable: true, enumerable: true, configurable: true });
export const A = Object.assign;
export const L = (it, next) => { let first = true; const r = { next() { if (first) { first = false; return { value: undefined, done: false }; } return Reflect.apply(next ?? it.next, it, []); }, get return() { const m = it.return; return m == null ? m : (...v) => Reflect.apply(m, it, v); }, [Symbol.iterator]() { return r; } }; return r; };
export const I = (it, next) => { const r = { next() { return Reflect.apply(next ?? it.next, it, []); }, get return() { const m = it.return; return m == null ? m : (...v) => Reflect.apply(m, it, v); }, [Symbol.iterator]() { return r; } }; return r; };
export const E = () => ({ [Symbol.iterator]() { return { next() { return { value: undefined, done: true }; } }; } });
export const Y = (it, next, a, ...first) => { let ph = !first.length; const m = (k) => { const f = it[k]; return f == null ? f : (...v) => Reflect.apply(f, it, v); }; const r = { next(...v) { if (first.length) return first.pop(); if (ph) { ph = false; return { value: undefined, done: false }; } return Reflect.apply(next, it, v); }, get throw() { return m('throw'); }, get return() { return m('return'); }, [a ? Symbol.asyncIterator : Symbol.iterator]() { return r; } }; return r; };
export const DD = (o, src) => { for (const k of Reflect.ownKeys(src)) { const d = Reflect.getOwnPropertyDescriptor(src, k); if ('get' in d) { if (d.get === undefined) delete d.get; if (d.set === undefined) delete d.set; } Object.defineProperty(o, k, d); } return o; };
export const U = ($djs_v, $djs_m, k) => k === 4 ? { [Symbol.asyncDispose]: () => Reflect.apply($djs_m, $djs_v, []) } : { [Symbol.dispose]: () => Reflect.apply($djs_m, $djs_v, []) };
export const M = ($djs_f, $djs_k) => (x, i) => Reflect.apply($djs_f, undefined, [x, i + $djs_k]);
export const FM = (it, next, $djs_f, k, inner, innerNext) => { const $djs_k = k - 1, $djs_in = { [Symbol.iterator]: () => L(inner, innerNext) }; const h = Iterator.prototype.flatMap.call(L(it, next), (x, i) => i === 0 ? $djs_in : Reflect.apply($djs_f, undefined, [x, i + $djs_k])); h.next(); return h; };
export const CC = (cur, curNext, items) => { const h = Iterator.concat({ [Symbol.iterator]: () => L(cur, curNext) }, ...items); h.next(); return h; };
export const CI = ($djs_m, $djs_o) => ({ [Symbol.iterator]: () => Reflect.apply($djs_m, $djs_o, []) });
export const HD = () => { const h = Iterator.prototype.map.call({ next: () => ({ value: undefined, done: true }) }, (x) => x); h.next(); return h; };
export const P = (k) => Reflect.ownKeys({ [k]: 0 })[0];
export const S = (o, ex) => { const r = {}, f = Object(o), ks = Reflect.ownKeys(f); for (let i = 0; i < ks.length; i++) { const k = ks[i]; let skip = false; for (let j = 0; j < ex.length; j++) if (ex[j] === k) skip = true; if (skip) continue; const d = Reflect.getOwnPropertyDescriptor(f, k); if (d && d.enumerable) Object.defineProperty(r, k, { value: f[k], writable: true, enumerable: true, configurable: true }); } return r; };
export const N = (it, next, ...first) => { const r = { next() { return first.length ? first.pop() : Reflect.apply(next, it, []); }, get return() { const m = it.return; return m == null ? m : (...v) => Reflect.apply(m, it, v); }, [Symbol.asyncIterator]() { return r; } }; return r; };
export const X = (it) => { let p; try { const m = it.return; if (m == null) return; p = Promise.resolve(Reflect.apply(m, it, [])); } catch { return; } return p.then(() => {}, () => {}); };
export const O = (r) => { if (Object(r) !== r) throw new TypeError('iterator must return an object'); };
export const fromSync = (it, next) => { const q = () => { try { const m = it.return; if (m != null) Reflect.apply(m, it, []); } catch {} }; const step = (f, close) => new Promise((res, rej) => { const r = f(); if (Object(r) !== r) throw new TypeError('iterator must return an object'); const done = !!r.done; let w; try { w = Promise.resolve(r.value); } catch (e) { if (!done && close) q(); throw e; } w.then((v) => res({ value: v, done }), !done && close ? (e) => { q(); rej(e); } : rej); }); return { next: (...a) => step(() => Reflect.apply(next, it, a), true), return: (...a) => { const m = it.return; return m == null ? Promise.resolve({ value: a[0], done: true }) : step(() => Reflect.apply(m, it, a), false); }, throw: (...a) => { const m = it.throw; if (m != null) return step(() => Reflect.apply(m, it, a), true); try { const c = it.return; if (c != null) { const r = Reflect.apply(c, it, []); if (Object(r) !== r) throw new TypeError('iterator must return an object'); } throw new TypeError('throw is not a method'); } catch (e) { return Promise.reject(e); } } }; };
export const Q = (a, ...v) => { let n = a.length; for (const x of v) Object.defineProperty(a, n++, { value: x, writable: true, enumerable: true, configurable: true }); return a; };
export const regexpIterator = (re, s, f) => { const it = Reflect.apply(matchAll, re, [s]); if (f & 4) while (!it.next().done); return it; };
export const closeThrow = (it) => (e) => { try { const m = it.return; if (m != null) Reflect.apply(m, it, []); } catch {} throw e; };
export const forIn = (o, keys) => { let i = 0; const r = { next() { while (i < keys.length) { const k = keys[i++]; if (o == null || k in o) return { value: k, done: false }; } return { value: undefined, done: true }; }, [Symbol.iterator]() { return r; } }; return r; };
export const advance = (it, n) => { const next = it.next; for (let i = 0; i < n; i++) Reflect.apply(next, it, []); return it; };
export const fn = (o) => { const d = Object.getOwnPropertyDescriptor(o, Reflect.ownKeys(o)[0]); return 'value' in d ? d.value : d.get ?? d.set; };
export const spent = (k) => { let f; new Promise((a, b) => { f = k ? b : a; a(); }); return f; };
export const rejected = (e) => { const p = Promise.reject(e); p.then(undefined, () => {}); return p; };
export const bytes = (b) => Uint8Array.fromBase64(b).buffer;
export const k = (f, r, p) => { Object.defineProperty(f, '$djs', { value: r }); if (p !== undefined) f.prototype = p; return f; };
export const keep = (o, keys) => { for (const k of Reflect.ownKeys(o)) if (!['length', 'name', 'prototype', 'constructor'].includes(k) && !keys.includes(k) && !delete o[k]) throw new TypeError('keep'); };

/** The helpers by name: `globalThis.$djs = { ...helpers, seed, resume }`. */
export const helpers = { R, G, C, D, A, L, I, E, Y, DD, U, M, FM, CC, CI, HD, P, S, N, X, O, fromSync, Q, regexpIterator, closeThrow, forIn, advance, fn, spent, rejected, bytes, k, keep };

/** The same as a Script whose completion value is `helpers`, for a host that evaluates it in
 * a djs VM itself (`vm_dump_runtime`). */
export const script = `"use strict";
(() => {
const matchAll = RegExp.prototype[Symbol.matchAll];
const R = Reflect.apply;
const G = Reflect.get;
const C = Reflect.construct;
const D = (o, k, v) => Object.defineProperty(o, k, { value: v, writable: true, enumerable: true, configurable: true });
const A = Object.assign;
const L = (it, next) => { let first = true; const r = { next() { if (first) { first = false; return { value: undefined, done: false }; } return Reflect.apply(next ?? it.next, it, []); }, get return() { const m = it.return; return m == null ? m : (...v) => Reflect.apply(m, it, v); }, [Symbol.iterator]() { return r; } }; return r; };
const I = (it, next) => { const r = { next() { return Reflect.apply(next ?? it.next, it, []); }, get return() { const m = it.return; return m == null ? m : (...v) => Reflect.apply(m, it, v); }, [Symbol.iterator]() { return r; } }; return r; };
const E = () => ({ [Symbol.iterator]() { return { next() { return { value: undefined, done: true }; } }; } });
const Y = (it, next, a, ...first) => { let ph = !first.length; const m = (k) => { const f = it[k]; return f == null ? f : (...v) => Reflect.apply(f, it, v); }; const r = { next(...v) { if (first.length) return first.pop(); if (ph) { ph = false; return { value: undefined, done: false }; } return Reflect.apply(next, it, v); }, get throw() { return m('throw'); }, get return() { return m('return'); }, [a ? Symbol.asyncIterator : Symbol.iterator]() { return r; } }; return r; };
const DD = (o, src) => { for (const k of Reflect.ownKeys(src)) { const d = Reflect.getOwnPropertyDescriptor(src, k); if ('get' in d) { if (d.get === undefined) delete d.get; if (d.set === undefined) delete d.set; } Object.defineProperty(o, k, d); } return o; };
const U = ($djs_v, $djs_m, k) => k === 4 ? { [Symbol.asyncDispose]: () => Reflect.apply($djs_m, $djs_v, []) } : { [Symbol.dispose]: () => Reflect.apply($djs_m, $djs_v, []) };
const M = ($djs_f, $djs_k) => (x, i) => Reflect.apply($djs_f, undefined, [x, i + $djs_k]);
const FM = (it, next, $djs_f, k, inner, innerNext) => { const $djs_k = k - 1, $djs_in = { [Symbol.iterator]: () => L(inner, innerNext) }; const h = Iterator.prototype.flatMap.call(L(it, next), (x, i) => i === 0 ? $djs_in : Reflect.apply($djs_f, undefined, [x, i + $djs_k])); h.next(); return h; };
const CC = (cur, curNext, items) => { const h = Iterator.concat({ [Symbol.iterator]: () => L(cur, curNext) }, ...items); h.next(); return h; };
const CI = ($djs_m, $djs_o) => ({ [Symbol.iterator]: () => Reflect.apply($djs_m, $djs_o, []) });
const HD = () => { const h = Iterator.prototype.map.call({ next: () => ({ value: undefined, done: true }) }, (x) => x); h.next(); return h; };
const P = (k) => Reflect.ownKeys({ [k]: 0 })[0];
const S = (o, ex) => { const r = {}, f = Object(o), ks = Reflect.ownKeys(f); for (let i = 0; i < ks.length; i++) { const k = ks[i]; let skip = false; for (let j = 0; j < ex.length; j++) if (ex[j] === k) skip = true; if (skip) continue; const d = Reflect.getOwnPropertyDescriptor(f, k); if (d && d.enumerable) Object.defineProperty(r, k, { value: f[k], writable: true, enumerable: true, configurable: true }); } return r; };
const N = (it, next, ...first) => { const r = { next() { return first.length ? first.pop() : Reflect.apply(next, it, []); }, get return() { const m = it.return; return m == null ? m : (...v) => Reflect.apply(m, it, v); }, [Symbol.asyncIterator]() { return r; } }; return r; };
const X = (it) => { let p; try { const m = it.return; if (m == null) return; p = Promise.resolve(Reflect.apply(m, it, [])); } catch { return; } return p.then(() => {}, () => {}); };
const O = (r) => { if (Object(r) !== r) throw new TypeError('iterator must return an object'); };
const fromSync = (it, next) => { const q = () => { try { const m = it.return; if (m != null) Reflect.apply(m, it, []); } catch {} }; const step = (f, close) => new Promise((res, rej) => { const r = f(); if (Object(r) !== r) throw new TypeError('iterator must return an object'); const done = !!r.done; let w; try { w = Promise.resolve(r.value); } catch (e) { if (!done && close) q(); throw e; } w.then((v) => res({ value: v, done }), !done && close ? (e) => { q(); rej(e); } : rej); }); return { next: (...a) => step(() => Reflect.apply(next, it, a), true), return: (...a) => { const m = it.return; return m == null ? Promise.resolve({ value: a[0], done: true }) : step(() => Reflect.apply(m, it, a), false); }, throw: (...a) => { const m = it.throw; if (m != null) return step(() => Reflect.apply(m, it, a), true); try { const c = it.return; if (c != null) { const r = Reflect.apply(c, it, []); if (Object(r) !== r) throw new TypeError('iterator must return an object'); } throw new TypeError('throw is not a method'); } catch (e) { return Promise.reject(e); } } }; };
const Q = (a, ...v) => { let n = a.length; for (const x of v) Object.defineProperty(a, n++, { value: x, writable: true, enumerable: true, configurable: true }); return a; };
const regexpIterator = (re, s, f) => { const it = Reflect.apply(matchAll, re, [s]); if (f & 4) while (!it.next().done); return it; };
const closeThrow = (it) => (e) => { try { const m = it.return; if (m != null) Reflect.apply(m, it, []); } catch {} throw e; };
const forIn = (o, keys) => { let i = 0; const r = { next() { while (i < keys.length) { const k = keys[i++]; if (o == null || k in o) return { value: k, done: false }; } return { value: undefined, done: true }; }, [Symbol.iterator]() { return r; } }; return r; };
const advance = (it, n) => { const next = it.next; for (let i = 0; i < n; i++) Reflect.apply(next, it, []); return it; };
const fn = (o) => { const d = Object.getOwnPropertyDescriptor(o, Reflect.ownKeys(o)[0]); return 'value' in d ? d.value : d.get ?? d.set; };
const spent = (k) => { let f; new Promise((a, b) => { f = k ? b : a; a(); }); return f; };
const rejected = (e) => { const p = Promise.reject(e); p.then(undefined, () => {}); return p; };
const bytes = (b) => Uint8Array.fromBase64(b).buffer;
const k = (f, r, p) => { Object.defineProperty(f, '$djs', { value: r }); if (p !== undefined) f.prototype = p; return f; };
const keep = (o, keys) => { for (const k of Reflect.ownKeys(o)) if (!['length', 'name', 'prototype', 'constructor'].includes(k) && !keys.includes(k) && !delete o[k]) throw new TypeError('keep'); };
return { R, G, C, D, A, L, I, E, Y, DD, U, M, FM, CC, CI, HD, P, S, N, X, O, fromSync, Q, regexpIterator, closeThrow, forIn, advance, fn, spent, rejected, bytes, k, keep };
})();
`;
