// Known builtins (D3300): a known global is replaced. Functions compiled and run before
// see the new binding, as do later reads.
function str(x) { return String(x); }
function floor(x) { return Math.floor(x); }
function num(x) { return Number(x) + Number.isInteger(x); }
const show = () => console.log(str(12), typeof str(12), floor(2.5), num(3), parseInt("7"), isNaN(1), Array.isArray([]));
for (let i = 0; i < 3; i++) show();
const S = String;
String = (x) => ["not a string", x];
show();
console.log(typeof String, String === S, String.length);
globalThis.Math = { floor: (x) => "floor " + x };
show();
globalThis.Number = function (x) { return 100; };
Number.isInteger = () => 5;
show();
parseInt = undefined;
try { show(); } catch (e) { console.log(e.name); }
globalThis["isNaN"] = () => "nan?";
Reflect.set(globalThis, "parseInt", () => "int");
show();
Array = { isArray: () => "array?" };
show();
