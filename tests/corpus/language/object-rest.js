// Object rest in destructuring (0280, D3512): the Rust copy where no getter runs, and the
// builtin for getters, proxies, strings and computed keys.
const out = [];
const O = { id: 7, name: "e", 2: "two", [Symbol.for("s")]: 1, count: 3 };
{ const { id, ...r } = O; out.push(JSON.stringify(r), Object.getOwnPropertySymbols(r).length); }
{ const { id, name: nm, ...r } = O; out.push(JSON.stringify(r)); }
{ const k = "count"; const { [k]: c, 2: t, ...r } = O; out.push(JSON.stringify(r), c, t); }
{ const { ...all } = [1, 2]; out.push(JSON.stringify(all)); }
{ const { a, ...r } = "hi"; out.push(JSON.stringify(r)); }
{ const { a, ...r } = 5; out.push(JSON.stringify(r)); }
let calls = 0; const G = { a: 1, get b() { calls++; return 2; }, get c() { calls++; return 3; } };
{ const { b, ...r } = G; out.push(JSON.stringify(r), calls); }
const P = new Proxy({ x: 1, y: 2 }, {}); { const { x, ...r } = P; out.push(JSON.stringify(r)); }
const H = Object.create({ inh: 1 }); H.own = 2; Object.defineProperty(H, "hid", { value: 3 }); { const { ...r } = H; out.push(JSON.stringify(r)); }
{ const sym = Symbol.for("s"); const { [sym]: s, ...r } = O; out.push(Object.getOwnPropertySymbols(r).length, s); }
function f({ id, ...rest }) { return Object.keys(rest).join(); } out.push(f(O));
console.log(out.join(" | "));
