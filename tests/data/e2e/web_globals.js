// DOMException, atob and btoa (0084, D1840); Function.prototype and array length sets.
console.log(btoa("hello"), atob("aGVsbG8="), atob(" aGVs bG8 "), btoa("\xff"));
for (const f of [() => atob("a"), () => btoa("\u0100")]) {
  try {
    f();
  } catch (e) {
    console.log(e instanceof DOMException, e instanceof Error, e.name, e.code, e.message);
  }
}
try { atob(); } catch (e) { console.log(e.name, e.message); }
const d = new DOMException("gone", "NotFoundError");
console.log(String(d), d.code, Error.isError(d), Object.keys(d).length, DOMException.NOT_FOUND_ERR);
console.log(Object.prototype.toString.call(d), Object.getPrototypeOf(DOMException) === Function.prototype);
try { DOMException.prototype.code; } catch (e) { console.log(e.name, e.message); }
class Mine extends DOMException { constructor() { super("m", "AbortError"); } }
console.log(new Mine().code, new Mine() instanceof Mine);

console.log(typeof Function.prototype, Function.prototype(1, 2), Object.prototype.toString.call(Function.prototype));
try { new Function.prototype(); } catch (e) { console.log(e.name); }

const a = [1, 2, 3, 4];
a.length = "2";
console.log(a.length);
a.length = new Number(1);
console.log(a.length, a);
const hints = [];
a.length = { [Symbol.toPrimitive](h) { hints.push(h); return 0; } };
console.log(a.length, hints.join());
try { a.length = 1.5; } catch (e) { console.log(e.name); }
console.log(Reflect.set(a, "length", { valueOf() { return 3; } }), a.length);
