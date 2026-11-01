// instanceof with Symbol.hasInstance (0034) and %ThrowTypeError%.
const Even = { [Symbol.hasInstance](n) { return n % 2 === 0; } };
console.log(2 instanceof Even, 3 instanceof Even);
function F() {}
Object.defineProperty(F, Symbol.hasInstance, { value: () => true });
console.log({} instanceof F, [] instanceof Array, [] instanceof Object, ({}) instanceof Array);
const G = (() => 1).bind();
Object.defineProperty(G, "prototype", { get() { return Array.prototype; } });
try { console.log([] instanceof G); } catch (e) { console.log(String(e)); }
try { 1 instanceof {}; } catch (e) { console.log(String(e)); }
try { 1 instanceof { [Symbol.hasInstance]: 5 }; } catch (e) { console.log(String(e)); }
const B = F.bind(null); console.log({} instanceof B);

const d = Object.getOwnPropertyDescriptor(Function.prototype, "caller");
console.log(d.get === d.set, d.get.name === "", d.get.length, Object.isFrozen(d.get), Object.isExtensible(d.get));
