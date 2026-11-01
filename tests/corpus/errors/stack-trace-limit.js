// difftest: stacks
// Error.stackTraceLimit (0105, D2030): a plain data property of Error that the VM reads
// without running code. Numbers are truncated and clamped; anything else records no frames.
const d = Object.getOwnPropertyDescriptor(Error, "stackTraceLimit");
console.log(JSON.stringify(d), Object.keys(Error).length);
function rec(n) { if (n === 0) throw new Error("x"); return rec(n - 1) + 1; }
// The recursive frames only: the harness adds frames of its own below them, which
// differ by engine.
function frames(f) {
  try { f(); } catch (e) { return e.stack.split("\n").filter((l) => /at (rec|go) /.test(l)).length; }
}
const deep = () => rec(30);
const runtime = () => { const go = (n) => (n === 0 ? null.x : go(n - 1) + 1); go(30); };
console.log("default", frames(deep), frames(runtime));
const values = [0, 1, 3, 2.7, 0.5, -1, -0, Infinity, -Infinity, NaN, "5", null, undefined, true, 1e10, 3n,
  { valueOf() { return 4; } }];
for (const v of values) {
  Error.stackTraceLimit = v;
  const shown = typeof v === "bigint" ? "3n" : typeof v === "object" && v !== null ? "object" : String(v);
  console.log(shown, frames(deep), frames(runtime));
}
Error.stackTraceLimit = 2;
console.log(new Error("e").stack);
function outer() { return middle(); }
function middle() { return inner(); }
function inner() { const o = {}; Error.captureStackTrace(o); return o; }
console.log(outer().stack);
function viaOpt() { const o = {}; Error.captureStackTrace(o, middle2); return o; }
function middle2() { return viaOpt(); }
function outer2() { return middle2(); }
console.log(outer2().stack);
Error.stackTraceLimit = 0;
console.log(JSON.stringify(new Error("e").stack), JSON.stringify(outer().stack));
try { null.x; } catch (e) { console.log(JSON.stringify(e.stack)); }
// A getter is never called.
let calls = 0;
Object.defineProperty(Error, "stackTraceLimit", { get() { calls++; return 5; }, configurable: true });
console.log(frames(deep), calls);
delete Error.stackTraceLimit;
console.log("stackTraceLimit" in Error, frames(deep));
// Only the realm's Error constructor counts.
class MyError extends Error {}
MyError.stackTraceLimit = 1;
Error.stackTraceLimit = 3;
console.log(frames(deep), frames(() => { throw new MyError("m"); }));
Error.stackTraceLimit = 10;
