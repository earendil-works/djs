// Global variable caches (0062, D1620): the same site sees every kind of change.
function t(f) {
  try {
    console.log(f());
  } catch (e) {
    console.log(e);
  }
}
var counter = 0;
function bump() { counter = counter + 1; return counter; }
for (let i = 0; i < 5; i++) bump();
console.log(counter);
function readG() { return typeof g === "undefined" ? "missing" : g; }
console.log(readG());
globalThis.g = 1;
console.log(readG());
globalThis.g = 2;
console.log(readG());
delete globalThis.g;
console.log(readG());
Object.defineProperty(globalThis, "g", { get() { return "getter"; }, configurable: true });
console.log(readG());
Object.defineProperty(globalThis, "g", { value: 0, writable: true, configurable: true });
function writeG(v) { g = v; return g; }
console.log(writeG(6), writeG(7));
// (qjs keeps throwing after a read-only global becomes writable again; node does not.
// That case is in vm/tests.rs.)
Object.defineProperty(globalThis, "ro", { value: "frozen", writable: false, configurable: true });
function writeRo() { ro = 1; }
t(writeRo);
t(writeRo);
// Many other globals before it: the global object's table is rehashed and reordered.
for (let i = 0; i < 50; i++) globalThis["filler" + i] = i;
delete globalThis.filler3;
console.log(writeG(8), readG(), filler49);
// A lexical binding (in its TDZ, then initialized).
function readL() { return lex; }
t(readL);
let lex = "lexical";
console.log(readL());
function writeL(v) { lex = v; return lex; }
console.log(writeL("x"), writeL("y"), readL());
const k = 1;
function writeK() { k = 2; }
t(writeK);
t(writeK);
// Reads of builtins in a loop.
let s = 0;
for (let i = 0; i < 100; i++) s += Math.max(i, 50) + Number.MAX_SAFE_INTEGER % 7;
console.log(s, typeof JSON, typeof undeclaredThing);
t(() => undeclaredThing);
