// Element access and `in` with string keys in loops (0268, D3402): the tier-0 loop's
// paths (dense elements read in the loop, string keys through the heap's helpers) must
// give what the general path gives for own and inherited data properties, missing keys,
// accessors, proxies, arrays, typed arrays, frozen objects and keys that no object has.
function key(...parts) {
  return parts.join("");
}
const log = [];
const proto = { inherited: 1, get viaGetter() { log.push("get"); return 7; } };
const o = Object.create(proto);
o.alpha = 1;
o.beta = "b";
Object.defineProperty(o, "ro", { value: 5, writable: false, enumerable: true });
Object.defineProperty(o, "acc", { get() { log.push("acc"); return 9; }, set(v) { log.push("set " + v); } });
const frozen = Object.freeze({ f: 1 });
const arr = [10, 20, , 40];
arr.extra = "x";
const ta = new Uint8Array([1, 2, 3]);
const px = new Proxy({ p: 1 }, {
  get(t, k) { log.push("px get " + String(k)); return t[k]; },
  has(t, k) { log.push("px has " + String(k)); return k in t; },
  set(t, k, v) { log.push("px set " + String(k)); t[k] = v; return true; },
});
const viaProxy = Object.create(px);
const targets = { o, frozen, arr, ta, px, viaProxy, fn: function named() {}, str: Object("hey") };
const keys = ["alpha", "beta", "ro", "acc", "inherited", "viaGetter", "missing", "never-" + "interned-" + log.length,
  "f", "0", "2", "3", "7", "length", "extra", "p", "name", "toString", "-0", "1.5", ""];
for (let round = 0; round < 3; round++) {
  for (const name of Object.keys(targets)) {
    const t = targets[name];
    const out = [];
    for (const k0 of keys) {
      const k = key(k0.slice(0, 1), k0.slice(1));
      let r;
      try { r = t[k]; } catch (e) { r = "throws " + e.name; }
      let h;
      try { h = k in t; } catch (e) { h = "throws " + e.name; }
      out.push(k + "=" + (typeof r === "function" ? "fn" : String(r)) + "/" + h);
    }
    if (round === 2) console.log(name, out.join(" "));
  }
}
console.log(log.length, log.slice(0, 6).join(","));
// Stores: only an own writable data property changes in the loop's path.
const counts = {};
const words = "a b a c b a d a".split(" ");
for (let i = 0; i < 40; i++) {
  const w = words[i % words.length];
  counts[w] = (counts[w] || 0) + 1;
}
console.log(JSON.stringify(counts));
const w = { x: 1, y: 2 };
Object.defineProperty(w, "fixed", { value: 1, writable: false });
const child = Object.create(w);
const refused = [];
function put(t, k, v) {
  try {
    t[k] = v;
  } catch (e) {
    refused.push(k + ":" + e.name);
  }
}
for (let i = 0; i < 5; i++) {
  w[key("x")] = i;
  put(w, key("fix", "ed"), i);
  child[key("y")] = i * 2;
  arr[key("", "1")] = i;
  arr[key("ex", "tra")] = "e" + i;
  ta[key("", "1")] = 250 + i;
  put(o, key("ac", "c"), i);
  put(o, key("r", "o"), i);
  put(frozen, key("f"), i);
  put(frozen, key("new", "key"), i);
  px[key("p")] = i;
  arr[key("len", "gth")] = 4 + (i & 1);
}
console.log(w.x, w.fixed, w.y, Object.keys(child), child.y, arr.join(), arr.extra, ta.join(), frozen.f, px.p, arr.length, o.ro);
console.log(refused.join(" "));
console.log(log.length, log.slice(-3).join(","));
// Dense reads: holes, out of range, negative and fractional keys, a string receiver.
let sum = 0;
const holes = [1, , 3];
Array.prototype[1] = 100;
for (let i = -1; i < 5; i++) sum += (holes[i] || 0) + ("xyz"[i] ? 1 : 0) + (holes[i + 0.5] || 0);
delete Array.prototype[1];
console.log(sum);
