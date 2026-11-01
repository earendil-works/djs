// Function.prototype.call/apply/bind/toString.
"use strict";
function probe(a, b) {
  return [this === undefined ? "undefined" : this, a, b, arguments.length];
}
console.log(probe.call(), probe.call(1, 2), probe.call("t", 2, 3, 4));
console.log(probe.apply(), probe.apply(1, [2, 3]), probe.apply(null, { length: 2, 0: "a" }));
const b = probe.bind("bound", 1);
console.log(b(2), b(2, 3), b.name, b.length, typeof b, Object.getOwnPropertyNames(b));
const bb = b.bind(null, 9);
console.log(bb(), bb.name, bb.length);
function Point(x, y) { this.x = x; this.y = y; }
const P = Point.bind(null, 1);
const p = new P(2);
console.log(p.x, p.y, p instanceof Point, p instanceof P, "prototype" in P);
try { probe.apply(null, 1); } catch (e) { console.log(String(e)); }
try { Function.prototype.call.call(1); } catch (e) { console.log(String(e)); }
try { Function.prototype.bind.call({}); } catch (e) { console.log(String(e)); }
console.log(probe.toString());
console.log((function () { return 1; }).toString(), ((a) => a * 2).toString());
console.log(Math.pow.toString());
console.log(Function.prototype.call.length, Function.prototype.apply.length, Function.prototype.bind.length);
console.log(Function.prototype.call.name, Object.getOwnPropertyNames(Function.prototype.call));
try { new Function.prototype.call(); } catch (e) { console.log(String(e)); }
try { new Math.pow(1, 2); } catch (e) { console.log(String(e)); }
