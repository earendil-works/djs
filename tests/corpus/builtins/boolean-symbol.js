// Boolean and Symbol.
console.log(Boolean(0), Boolean("x"), Boolean({}), typeof new Boolean(false), new Boolean(false).valueOf(), true.toString(), (false).valueOf());
console.log(Object.prototype.toString.call(new Boolean(true)), String(new Boolean(true)));
try { Boolean.prototype.toString.call(1); } catch (e) { console.log(String(e)); }
const s = Symbol("desc");
console.log(typeof s, s.toString(), s.description, Symbol().description, Symbol("").description, String(s));
console.log(Symbol.for("app") === Symbol.for("app"), Symbol.keyFor(Symbol.for("app")), Symbol.keyFor(s), Symbol.for("app") === Symbol("app"));
console.log(typeof Symbol.iterator, Symbol.iterator.toString(), Symbol.hasInstance.description, Symbol.toStringTag === Symbol.toStringTag);
try { new Symbol(); } catch (e) { console.log(String(e)); }
try { Symbol.keyFor("x"); } catch (e) { console.log(String(e)); }
try { s + ""; } catch (e) { console.log(String(e)); }
const o = Object(s);
console.log(typeof o, o.valueOf() === s, o.description, Object.prototype.toString.call(o));
const keyed = { [s]: 1, plain: 2 };
console.log(keyed[s], Object.keys(keyed), Object.getOwnPropertySymbols(keyed).length);
function F() {}
console.log(F[Symbol.hasInstance](new F()), Function.prototype[Symbol.hasInstance].call(F, {}));
