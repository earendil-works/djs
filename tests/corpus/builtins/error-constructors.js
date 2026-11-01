// Error constructors: message, cause, prototype chains, toString, internal errors.
const names = ["Error", "TypeError", "RangeError", "ReferenceError", "SyntaxError", "EvalError", "URIError"];
for (let i = 0; i < names.length; i++) {
  const C = globalThis[names[i]];
  const e = new C("m" + i, { cause: i });
  console.log(names[i], C.name, C.length, e.message, e.cause, e.name, String(e));
  console.log(e instanceof C, e instanceof Error, e.constructor === C,
    Object.getPrototypeOf(C) === (i === 0 ? Function.prototype : Error),
    C.prototype.constructor === C, Object.getOwnPropertyNames(C.prototype).indexOf("name") >= 0);
  const f = C();
  console.log(f.message, f.hasOwnProperty("message"), f.hasOwnProperty("cause"), String(f));
}
console.log(new Error(undefined).hasOwnProperty("message"), new Error(5).message, new Error("x", {}).hasOwnProperty("cause"));
console.log(new Error("x", { cause: undefined }).hasOwnProperty("cause"), new Error("x", 5).hasOwnProperty("cause"));
const d = Object.getOwnPropertyDescriptor(new Error("a"), "message");
console.log(d.value, d.writable, d.enumerable, d.configurable);
console.log(Error.prototype.toString.call({ name: "N", message: "M" }), Error.prototype.toString.call({}),
  Error.prototype.toString.call({ name: "", message: "only" }), Error.prototype.toString.call({ name: "only" }));
try { Error.prototype.toString.call(1); } catch (e) { console.log(String(e)); }
try { null.x; } catch (e) { console.log(e.constructor === TypeError, e instanceof TypeError, e.constructor.name); }
try { undefinedVariable; } catch (e) { console.log(e.constructor === ReferenceError, String(e)); }
try { new Array(-1); } catch (e) { console.log(e.constructor === RangeError, String(e)); }
try { throw new TypeError("thrown"); } catch (e) { console.log(String(e), e.stack.length > 0); }
const ownNames = Object.getOwnPropertyNames(new Error("a", { cause: 1 }));
const withoutStack = [];
for (let i = 0; i < ownNames.length; i++) if (ownNames[i] !== "stack") withoutStack.push(ownNames[i]);
console.log(withoutStack);
