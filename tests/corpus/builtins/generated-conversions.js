const log = [];
console.log(Math.floor(3.7), Math.floor(-0) === 0 && 1 / Math.floor(-0), Math.floor("4.5"), Math.floor(), Math.floor(null), Math.abs(-3), Math.round(2.5), Math.sign(-2), Math.trunc(-4.7), Math.clz32(1), Math.fround(5.5));
console.log(Math.floor({ valueOf() { log.push("valueOf"); return 9.9; } }), log.join());
console.log(Math.floor({ [Symbol.toPrimitive](h) { return h === "number" ? 1.5 : "x"; } }));
try { Math.floor(Symbol()); } catch (e) { console.log(e.name, e.message); console.log(e.stack); }
try { Math.floor(1n); } catch (e) { console.log(e.name, e.message); }
try { Math.floor({ valueOf() { throw new Error("boom"); } }); } catch (e) { console.log(e.stack); }
try { Math.floor({ valueOf() { return {}; }, toString() { return {}; } }); } catch (e) { console.log(e.name, e.message); console.log(e.stack); }
console.log(" hi ".trim() + "|", " hi ".trimStart() + "|", " hi ".trimEnd() + "|", "ABC".toLowerCase(), "abc".toUpperCase(), "ÄÖ".toLowerCase(), "ß".toUpperCase());
console.log(String.prototype.trim.call(42), String.prototype.toUpperCase.call(true));
console.log(String.prototype.trim.call({ toString() { return "  obj  "; } }) + "|");
try { String.prototype.trim.call(null); } catch (e) { console.log(e.name, e.message); console.log(e.stack); }
try { String.prototype.trim.call(Symbol()); } catch (e) { console.log(e.name, e.message); }
try { String.prototype.trim.call({ toString() { throw new TypeError("ts"); } }); } catch (e) { console.log(e.stack); }
console.log([1.5, { valueOf: () => 2.5 }, "3.5"].map(Math.floor).join());
console.log(Math.floor.name, Math.floor.length, "".trim.name, "".trim.length, typeof Math.floor, Math.floor.call(null, 7.2), Math.floor.apply(null, [8.8]), Reflect.apply(Math.floor, null, [{valueOf(){return 1.1}}]));
console.log(Math.floor.bind(null)({ valueOf: () => 6.6 }));
try { new Math.floor(1); } catch (e) { console.log(e.name, e.message); }
const order = [];
Math.floor({ valueOf() { order.push("a"); return 1; } }, { valueOf() { order.push("b"); return 2; } });
console.log(order.join());
async function f() { return Math.floor({ valueOf() { return 3.3; } }) + (await 1); }
f().then((v) => console.log("async", v));
let acc = 0;
for (let i = 0; i < 300; i++) {
  acc += Math.floor({ valueOf() { let s = 0; for (let k = 0; k < 5; k++) s += k; return i + s / 10; } });
  acc += String.prototype.trim.call({ toString() { return " " + i + " "; } }).length;
  acc += [i + 0.5, { valueOf: () => i }].map(Math.ceil)[1];
}
console.log(acc);
try { console.log(String.prototype.trim.call({ toString() { return null; } })); } catch (e) { console.log(e.name, e.message); }
try { console.log(String.prototype.toUpperCase.call({ toString() { return undefined; } })); } catch (e) { console.log(e.name, e.message); }
// Get sites (IsRegExp of startsWith, endsWith, includes).
{
const log = [];
console.log("hello".startsWith("he"), "hello".endsWith("lo"), "hello".includes("ll"), "hello".startsWith("l", 2), "hello".endsWith("l", 4), "hello".includes("h", 1));
try { "abc".startsWith(/a/); } catch (e) { console.log(e.name, e.message); console.log(e.stack); }
try { "abc".includes({ [Symbol.match]: true }); } catch (e) { console.log(e.name, e.message); }
const re = /a/; re[Symbol.match] = false; console.log("/a/".startsWith(re), "x/a/".includes(re));
console.log("abc".startsWith({ get [Symbol.match]() { log.push("match"); return undefined; }, toString() { log.push("str"); return "ab"; } }, { valueOf() { log.push("pos"); return 0; } }), log.join());
try { "abc".endsWith({ get [Symbol.match]() { throw new Error("getter"); } }); } catch (e) { console.log(e.message); }
try { String.prototype.includes.call(null, "a"); } catch (e) { console.log(e.name, e.message); }
console.log("a\u1234b".includes("\u1234"), "".startsWith(""), "abc".endsWith("", 1), "abc".includes("c", -5));
try { String.prototype.trim.call({ toString() { return Symbol(); } }); } catch (e) { console.log(e.name, e.message); console.log(e.stack); }
try { Math.floor({ valueOf() { return Symbol(); } }); } catch (e) { console.log(e.name, e.message); console.log(e.stack); }
}
// Errors of a builtin called again after a conversion.
{
function f() { try { return "x".padStart({ valueOf() { return 5; } }, { toString() { return Symbol(); } }); } catch (e) { return "caught " + e.message; } }
console.log(f());
console.log([{ valueOf() { return Symbol(); } }].map((x) => { try { return Math.abs(x); } catch (e) { return e.message; } })[0]);
try { [{ valueOf() { return Symbol(); } }].map(Math.abs); } catch (e) { console.log(e.message); console.log(e.stack); }
}
