const log = [];
const p = Promise.resolve(0);
async function a() { log.push("a1"); await p; log.push("a2"); await p; log.push("a3"); }
async function* g() { yield 1; yield 2; }
async function c() { for await (const x of g()) log.push("g" + x); }
a(); c();
Promise.resolve().then(() => log.push("p1")).then(() => log.push("p2")).then(() => log.push("p3")).then(() => log.push("p4")).then(() => log.push("p5")).then(() => print(log.join(" ")));
