// The String iterator (0034) and spread/Array.from with patched iteration.
const it = "a😀b"[Symbol.iterator]();
console.log(Object.prototype.toString.call(it), typeof it.next, it.next(), it.next(), it.next(), it.next(), it.next());
console.log([..."x😀y"], Array.from("héllo"), Array.from("ab", (c) => c + c));
const proto = Object.getPrototypeOf(it);
console.log(proto === Object.getPrototypeOf(""[Symbol.iterator]()), Object.getPrototypeOf(proto) === Object.getPrototypeOf(Object.getPrototypeOf([][Symbol.iterator]())), proto[Symbol.toStringTag], proto.next.name, String.prototype[Symbol.iterator].name);
try { proto.next.call([][Symbol.iterator]()); } catch (e) { console.log(String(e)); }
try { [][Symbol.iterator]().next.call("x"[Symbol.iterator]()); } catch (e) { console.log(String(e)); }
try { String.prototype[Symbol.iterator].call(null); } catch (e) { console.log(String(e)); }
const orig = String.prototype[Symbol.iterator];
String.prototype[Symbol.iterator] = function () { let d = false; return { next() { if (d) return { done: true }; d = true; return { value: "P", done: false }; } }; };
console.log([..."abc"], Array.from("abc"));
String.prototype[Symbol.iterator] = orig;
const nextOrig = proto.next;
proto.next = function () { return { done: true }; };
console.log([..."abc"], Array.from("abc"));
proto.next = nextOrig;
console.log([..."ok"], Array.from([1, , 3]), Array.from([1, 2, 3], (v, k) => v * k));
const arr = [1, 2];
console.log(Array.from(arr, (v) => { if (arr.length < 4) arr.push(v + 10); return v; }));
console.log(new AggregateError([1, 2]).errors, Object.fromEntries([["k", "v"]]), Object.groupBy([1, 2, 3], (v) => v % 2));
