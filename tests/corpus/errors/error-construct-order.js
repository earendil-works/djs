// The Error constructors' steps in order (0266: their helpers are written out), the
// message and cause properties, and the frames a stack shows for every kind of name.
const log = [];
const msg = { toString() { log.push("message"); return "m"; } };
const nt = new Proxy(function () {}, {
  get(t, k) { log.push("get " + String(k)); return k === "prototype" ? TypeError.prototype : t[k]; },
});
const e = Reflect.construct(Error, [msg, { get cause() { log.push("cause"); return 1; } }], nt);
console.log(log.join(), Object.getPrototypeOf(e) === TypeError.prototype, e.message, e.cause);
console.log(JSON.stringify(Object.getOwnPropertyNames(new RangeError("r", { cause: 0 }))));
const d = Object.getOwnPropertyDescriptor(new Error("x"), "message");
console.log(JSON.stringify(d), Object.hasOwn(new Error(), "message"), Object.hasOwn(new Error(undefined), "message"));
console.log(new Error(42).message, typeof new Error(42).message, new Error(null).message, Error("no new") instanceof Error);
try { new Error(Symbol("s")); } catch (x) { console.log(x.constructor.name, String(x.message).length > 0); }
log.length = 0;
try { new Error({ toString() { throw new Error("inner"); } }); } catch (x) { console.log(x.message); }
class MyError extends Error { constructor(m) { super(m); this.name = "MyError"; } }
const me = new MyError("mine");
console.log(me instanceof MyError, me.message, String(me), Object.hasOwn(me, "message"));
console.log(new AggregateError([1, 2], "agg").errors.length, new AggregateError([]).message === "");

// Frames: named, anonymous, a method with an index name, a getter, a callback.
const frames = (err) => err.stack.split("\n").filter((l) => l.trim()).map((l) => l.trim().replace(/\(.*:(\d+):\d+\)/, "(L$1)")).slice(0, 3).join(" | ");
function outer() { return inner(); }
function inner() { return new Error("deep"); }
console.log(frames(outer()));
console.log(frames((() => new Error("arrow"))()));
const o = { 7() { return new Error("i"); }, get g() { return new Error("g"); } };
console.log(frames(o[7]()));
console.log(frames(o.g));
console.log(frames([1].map(() => new Error("cb"))[0]));
Error.stackTraceLimit = 1;
console.log(frames(outer()));
Error.stackTraceLimit = 0;
console.log(JSON.stringify(outer().stack));
Error.stackTraceLimit = 10;
Error.prepareStackTrace = (err, sites) => sites.slice(0, 2).map((s) => [s.getFunctionName(), typeof s.getFileName(), s.isNative()].join(":")).join(",");
console.log(outer().stack, o[7]().stack.split(",")[0]);
delete Error.prepareStackTrace;
