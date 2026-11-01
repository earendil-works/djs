// difftest: stacks
// Error.prepareStackTrace (0106, D2050), as qjs implements it: called with (error,
// CallSite[]) and `this` = Error when a stack is recorded; its result is the stack.
// File names differ by harness, so they are only checked for type, and the harness's own
// frames (from its `run` on) are left out.
const log = [];
const mine = (sites) => {
  const i = sites.findIndex((s) => s.getFunctionName() === "run");
  return i < 0 ? sites : sites.slice(0, i);
};
const show = (s) => [s.getFunctionName(), typeof s.getFileName(), s.getLineNumber(), s.getColumnNumber(),
  s.isNative(), s.isConstructor(), typeof s.getFunction()].join("|");
Error.prepareStackTrace = function (err, sites) {
  log.push([this === Error, err instanceof Error ? err.message : typeof err, Array.isArray(sites), mine(sites).length].join());
  return mine(sites).map(show);
};
function f() { return new Error("a"); }
const e1 = f();
console.log(log.splice(0).join(";"), JSON.stringify(e1.stack));
try { null.x; } catch (e) { console.log(log.splice(0).join(";"), JSON.stringify(e.stack)); }
try { [].reduce((a) => a); } catch (e) { console.log(log.splice(0).join(";"), JSON.stringify(e.stack)); }
const o = {};
Error.captureStackTrace(o);
console.log(log.splice(0).join(";"), JSON.stringify(o.stack));
class C { constructor() { this.e = new Error("c"); } }
console.log(JSON.stringify(new C().e.stack));
const indirect = eval;
try { indirect("("); } catch (e) { console.log(JSON.stringify(e.stack)); }
try { Function("a b"); } catch (e) { console.log(JSON.stringify(e.stack)); }
try { indirect("null.x"); } catch (e) { console.log(JSON.stringify(e.stack)); }
try { Function("null.x")(); } catch (e) { console.log(JSON.stringify(e.stack)); }
function thrower() { throw new TypeError("deep"); }
function middle() { return [1].map(() => thrower()); }
try { middle(); } catch (e) { console.log(JSON.stringify(e.stack)); }
async function af() { await null; return new Error("af").stack; }
console.log(JSON.stringify(await af()));
log.length = 0;

// The CallSite objects.
Error.prepareStackTrace = (e, sites) => sites;
const site = new Error().stack[0];
const proto = Object.getPrototypeOf(site);
console.log(Object.prototype.toString.call(site), Object.getOwnPropertyNames(proto).join(),
  Object.keys(site).length, Object.getPrototypeOf(proto) === Object.prototype);
console.log(JSON.stringify(Object.getOwnPropertyDescriptor(proto, Symbol.toStringTag)), proto.isNative.length);
try { proto.getFileName.call({}); } catch (e) { console.log(e.name, e.message); }
try { proto.isNative.call(1); } catch (e) { console.log(e.name, e.message); }
const named = { m() { return new Error().stack[0].getFunctionName(); } };
class K { static s() { return new Error().stack[0].getFunctionName(); } }
const bound = function bf() { return new Error().stack[0].getFunctionName(); }.bind(null);
console.log(named.m(), K.s(), bound(), [1].map(() => new Error().stack[1].getFunction() === Array.prototype.map)[0]);
const arrow = () => new Error().stack[0].getFunctionName();
console.log(arrow(), (function () { return new Error().stack[0].getFunctionName(); }).call());

// Results, failures and recursion.
Error.prepareStackTrace = () => { throw new Error("inner"); };
console.log(new Error("x").stack);
Error.prepareStackTrace = () => 42;
console.log(new Error("x").stack, typeof new Error("x").stack);
Error.prepareStackTrace = () => undefined;
console.log(new Error("x").stack, "stack" in new Error("x"));
Error.prepareStackTrace = (e) => {
  const inner = new Error("in");
  return "outer:" + typeof e.stack + ":" + typeof inner.stack + ":" + inner.stack.split("\n").length;
};
console.log(new Error("x").stack);
Error.prepareStackTrace = 5;
console.log(new Error("x").stack);
let n = 0;
Error.prepareStackTrace = (e, s) => { n++; return "p" + s.length; };
Error.stackTraceLimit = 1;
console.log(new Error("x").stack);
Error.stackTraceLimit = 0;
console.log(new Error("x").stack, n);
Error.stackTraceLimit = 10;
const er = new Error("x");
console.log(n, er.stack, n, er.stack, n);
er.stack = "set";
console.log(er.stack);
Object.defineProperty(Error, "prepareStackTrace", { get() { log.push("getter"); return () => "g"; }, configurable: true });
console.log(new Error("x").stack, log.join());
delete Error.prepareStackTrace;
console.log(new Error("x").stack);
