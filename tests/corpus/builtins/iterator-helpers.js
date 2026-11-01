// Iterator helpers (0051): laziness, trap order through a proxy-free logging iterator,
// closing on early exit and errors, and helpers over generators.
function source(n, log) {
  let i = 0;
  return Iterator.from({
    get next() { log.push("get next"); return () => { log.push("next " + i); return i < n ? { value: i++, done: false } : { value: undefined, done: true }; }; },
    get return() { log.push("get return"); return () => { log.push("return"); return {}; }; },
  });
}
const log = [];
const h = source(6, log).filter((x) => x % 2).map((x) => x * x);
log.push("created");
console.log(h.next().value, h.next().value, log.join());
log.length = 0;
console.log(h.return().done, h.next().done, log.join());
log.length = 0;
console.log(source(4, log).drop(2).toArray(), log.join());
log.length = 0;
console.log(source(4, log).take(0).next().done, log.join());
log.length = 0;
try { source(4, log).flatMap((x) => x).next(); } catch (e) { console.log(String(e), log.join()); }
log.length = 0;
try { source(4, log).every(() => { throw new RangeError("stop"); }); } catch (e) { console.log(String(e), log.join()); }
function* gen() { try { yield 1; yield 2; yield 3; } finally { console.log("gen finally"); } }
console.log(gen().map((x) => x + 1).take(2).toArray());
console.log(gen().drop(1).find((x) => x === 2));
console.log(Iterator.concat(gen(), [9]).reduce((a, b) => a + b, 0));
const lazy = Iterator.concat({ [Symbol.iterator]() { console.log("opened"); return [1][Symbol.iterator](); } });
console.log("before");
console.log(lazy.toArray());
console.log(typeof Iterator, Iterator.length, Iterator.name, Object.keys(Iterator.prototype).length);
console.log([...new Map([[1, 2]]).entries().map(([k, v]) => k + v)]);
for (const x of "héllo"[Symbol.iterator]().filter((c) => c !== "l")) console.log(x);
