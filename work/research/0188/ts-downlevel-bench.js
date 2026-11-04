// Await-heavy loop plus an interleaving probe (job counts are observable).
async function work(n) {
  let s = 0;
  for (let i = 0; i < n; i++) {
    s += await i;
    try { if (i % 7 === 0) s ^= await (i * 3); } finally { s++; }
  }
  return s;
}
async function a() { log.push("a1"); await null; log.push("a2"); await null; log.push("a3"); }
async function b() { log.push("b1"); await null; log.push("b2"); await null; log.push("b3"); }
async function r() { return Promise.resolve(1); }
const log = [];
a(); b(); r().then(() => log.push("r")); Promise.resolve().then(() => log.push("p1")).then(() => log.push("p2")).then(() => log.push("p3")).then(() => log.push("p4"));
const t = Date.now();
work(200000).then((s) => print(s, log.join(" ")));
