// Known builtins (D3300) across suspension: the function and `this` of a call wait on
// the stack while an argument awaits.
const later = (v) => tools.echo(v);
async function f(x) { return Math.floor(await later(x)) + Math.max(await later(1), await later(x)) + String(await later(x)).length; }
async function g(x) { return [Number.isInteger(await later(x)), parseInt(await later("12px")), String.fromCharCode(await later(66)), Array.isArray(await later([]))].join(); }
let t = 0;
for (let i = 0; i < 4; i++) t += await f(i + 0.5);
console.log(t, await g(2), await g(2.5));
function* gen() { const a = Math.floor(yield 1); const b = String(yield a); return Math.max(yield b, a); }
const it = gen();
console.log(it.next().value, it.next(2.5).value, it.next(7).value, it.next(1).value);
