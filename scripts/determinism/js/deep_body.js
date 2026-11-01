const r = [];
let o = {}; let cur = o; for (let i = 0; i < 4990; i++) { cur.a = {}; cur = cur.a; }
try { r.push(JSON.stringify(o).length); } catch (e) { r.push(String(e)); }
let a = []; let c = a; for (let i = 0; i < 4990; i++) { const n = []; c.push(n); c = n; }
try { r.push(JSON.stringify(a).length); } catch (e) { r.push(String(e)); }
try { r.push(JSON.parse("[".repeat(4990) + "]".repeat(4990)).length); } catch (e) { r.push(String(e)); }
try { r.push(String(a).length); } catch (e) { r.push(String(e)); }
try { r.push(new RegExp("(".repeat(250) + "a" + ")".repeat(250)).test("a")); } catch (e) { r.push(String(e)); }
function rec(n) { return n === 0 ? 0 : 1 + rec(n - 1); }
try { r.push(rec(100000)); } catch (e) { r.push(String(e)); }
let p = Promise.resolve(0); for (let i = 0; i < 10000; i++) p = p.then((x) => x + 1);
r.push(await p);
let res; const pp = new Promise((f) => (res = f)); let q = pp; for (let i = 0; i < 10000; i++) q = Promise.resolve(q);
res(1); r.push(await q);
await tool(0);
return r.join(" | ");
