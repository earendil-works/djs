// Error.captureStackTrace as qjs has it (0102, D2010). Frames are shown by name only,
// and only this case's functions (the codemode harness's own frames differ by engine).
const mine = ["inner", "middle", "outer", "thrower", "MyError"];
const names = (s) => s.split("\n").filter(Boolean).map((l) => l.trim().split(" ")[1]).filter((n) => mine.includes(n)).join(",");
function inner(o, f) { Error.captureStackTrace(o, f); return o; }
function middle(o, f) { return inner(o, f); }
function outer(o, f) { return middle(o, f); }
const a = outer({});
const d = Object.getOwnPropertyDescriptor(a, "stack");
console.log(names(a.stack), d.writable, d.enumerable, d.configurable);
console.log(names(outer({}, middle).stack), names(outer({}, outer).stack));
console.log(JSON.stringify(outer({}, function notOnTheStack() {}).stack));
const e = new Error("x");
Error.captureStackTrace(e);
console.log(Object.getOwnPropertyNames(e).join(), names(e.stack));
class MyError extends Error {
  constructor(m) { super(m); Error.captureStackTrace(this, MyError); }
}
function thrower() { throw new MyError("mine"); }
try { thrower(); } catch (x) { console.log(x.message, names(x.stack)); }
for (const bad of [1, "s", undefined, null]) {
  try { Error.captureStackTrace(bad); } catch (x) { console.log(x.name, x.message); }
}
try { Error.captureStackTrace(Object.freeze({})); } catch (x) { console.log(x.name, x.message); }
const f = Object.freeze({ stack: "old" });
try { Error.captureStackTrace(f); } catch (x) { console.log(x.name); }
const g = Object.preventExtensions({ stack: "old" });
Error.captureStackTrace(g);
console.log(g.stack === "old", typeof g.stack);
console.log(Error.captureStackTrace.length, Error.captureStackTrace.name, Error.captureStackTrace({}), Object.keys(Error).join());
