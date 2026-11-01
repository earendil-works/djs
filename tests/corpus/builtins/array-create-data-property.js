// CreateDataPropertyOrThrow into map's and filter's results defines own elements: index
// setters on Array.prototype are never called (0129: the dense store skips the prototype
// chain), while a non-extensible or fixed-length result still throws.
let log = "";
Object.defineProperty(Array.prototype, "0", { set(v) { log += "setter " + v + ";"; }, get() { return "proto"; }, configurable: true });
const m = [1, 2, 3].map((x) => x * 2);
const f = [1, 2, 3, 4].filter((x) => x % 2 === 0);
console.log(m.length, m[0], m[2], f.length, f[0], f[1], m.hasOwnProperty(0), log);
const plain = [];
plain[0] = "set";
console.log(plain.length, plain.hasOwnProperty(0), log);
delete Array.prototype[0];
function species(make) {
  const a = [1, 2, 3];
  a.constructor = { [Symbol.species]: function () { return make(); } };
  return a;
}
for (const [name, make] of [
  ["non-extensible", () => Object.preventExtensions([])],
  ["fixed length", () => Object.defineProperty([], "length", { writable: false })],
  ["frozen element", () => Object.freeze([9])],
]) {
  try { console.log(name, JSON.stringify(species(make).map((x) => x))); } catch (e) { console.log(name, e.constructor.name); }
  try { console.log(name, JSON.stringify(species(make).filter(() => true))); } catch (e) { console.log(name, e.constructor.name); }
}
const holes = [, , 3].map((x) => x);
console.log(holes.length, 0 in holes, 2 in holes);
