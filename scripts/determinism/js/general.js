// A mixed workload: objects, Map/Set with object keys, strings, sort, JSON, regexp, promises.
const objs = [];
for (let i = 0; i < 3000; i++) objs.push({ id: i, name: "n" + (i * 7919 % 1000), tags: [i % 3, i % 5] });
const m = new Map();
const s = new Set();
for (const o of objs) { m.set(o, o.id); if (o.id % 2) s.add(o); }
for (const o of objs) if (o.id % 3 === 0) m.delete(o);
let acc = 0;
for (const [k, v] of m) acc = (acc * 31 + v + k.tags[0]) | 0;
const wm = new WeakMap();
for (const o of objs) wm.set(o, { x: o.id });
const sorted = objs.slice().sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0));
let str = "";
for (let i = 0; i < 2000; i++) str += sorted[i].name + ",";
const json = JSON.stringify(sorted.slice(0, 200));
const re = /n(\d+)/g;
let count = 0;
for (const mm of str.matchAll(re)) count += +mm[1];
console.log(acc, s.size, m.size, str.length, json.length, count, Math.sin(acc), Math.random());
const order = [];
const p = Promise.all([1, 2, 3].map(async (x) => { await null; order.push(x); await Promise.resolve(); order.push(-x); return x * 2; }));
Promise.resolve().then(() => order.push("t"));
p.then((v) => { console.log(order.join(" "), v.join(" ")); });
