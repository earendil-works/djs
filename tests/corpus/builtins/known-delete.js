// Known builtins (D3300): deleted builtins and deleted globals.
function floor(x) { return Math.floor(x); }
function pint(x) { return parseInt(x); }
function isArr(x) { return Array.isArray(x); }
const attempt = (f) => { try { return f(); } catch (e) { return e.name; } };
const show = () => console.log(attempt(() => floor(2.5)), attempt(() => pint("12")), attempt(() => isArr([])), attempt(() => typeof Math), attempt(() => typeof parseInt));
for (let i = 0; i < 3; i++) show();
console.log(delete Math.floor);
show();
console.log(delete globalThis.parseInt);
show();
console.log("floor" in Math, "parseInt" in globalThis, Object.keys(Math).length);
Math.floor = (x) => -x;
globalThis.parseInt = (x) => "p" + x;
show();
console.log(delete globalThis.Array);
show();
console.log(attempt(() => Array));
