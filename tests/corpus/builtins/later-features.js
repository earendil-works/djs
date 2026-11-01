// Features promoted from test262's later group in 0097: Error.prototype.stack as an
// accessor (D1971) and Promise.allKeyed / allSettledKeyed (D1972), as codemode sees them.
const e = new TypeError("boom");
const d = Object.getOwnPropertyDescriptor(Error.prototype, "stack");
console.log(Object.getOwnPropertyNames(e).join(), typeof d.get, typeof d.set, d.enumerable, d.configurable);
console.log(typeof e.stack, e.stack.startsWith("    at "), JSON.stringify(e), JSON.stringify({ e }));
e.stack = "custom";
console.log(e.stack, Object.getOwnPropertyNames(e).join());
try { d.set.call(e, 5); } catch (x) { console.log(x.name); }
try { d.set.call(Error.prototype, "s"); } catch (x) { console.log(x.name); }
console.log(d.get.call({}), d.get.call(new Proxy(new Error("p"), {})), Error.prototype.stack);
class MyError extends Error {}
const m = new MyError("sub");
console.log(m.stack.split("\n").length > 1, m instanceof MyError);
try { null.x; } catch (x) { console.log(Object.getOwnPropertyNames(x).join(), typeof x.stack); }
const r1 = await Promise.allKeyed({ a: 1, b: Promise.resolve(2), [Symbol.for("s")]: 3 });
console.log(Object.getPrototypeOf(r1), JSON.stringify(r1), r1[Symbol.for("s")]);
const r2 = await Promise.allSettledKeyed({ ok: Promise.resolve("y"), bad: Promise.reject(new Error("n")) });
console.log(r2.ok.status, r2.ok.value, r2.bad.status, r2.bad.reason.message);
try { await Promise.allKeyed({ a: Promise.reject(new RangeError("first")), b: 1 }); } catch (x) { console.log(x.name, x.message); }
try { await Promise.allKeyed(5); } catch (x) { console.log(x.name); }
console.log(JSON.stringify(await Promise.allKeyed({})), Promise.allKeyed.length, Promise.allSettledKeyed.name);
