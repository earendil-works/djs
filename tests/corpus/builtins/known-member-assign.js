// Known builtins (D3300): `Math.floor`, `String`, `Number.isInteger` come from a table
// while the builtins are the base's. An assignment to a builtin's property ends that,
// also for code that ran before, and the call in progress keeps the callee it read.
function floor(x) { return Math.floor(x); }
function viaValue(a) { return a.map(Math.floor).join(); }
function str(x) { return String(x); }
function isInt(x) { return Number.isInteger(x); }
function all() {
  return [floor(7.5), viaValue([1.5, -1.5]), str(12), isInt(3), parseInt("42px"), parseFloat("1.5e1x"), Math.max(1, 2), Math.min(1, 2),
    Array.isArray([]), String.fromCharCode(65), isNaN("x"), Number.isFinite(1 / 0), Number.isNaN(NaN), typeof Math, Math.PI > 3, Math.abs(-2),
    Math.ceil(1.2), Math.round(2.5), Math.trunc(-2.7), Math.sqrt(16), Math.pow(2, 10), Math.sign(-3), Math.random() < 1, Number("0x10")].join(" ");
}
for (let i = 0; i < 3; i++) console.log("pristine", all());
const original = Math.floor;
// The patch happens while the arguments are evaluated: the callee was read before.
console.log("patch in arguments", Math.floor((Math.floor = (x) => "patched " + x, 2.5)));
for (let i = 0; i < 3; i++) console.log("patched", all());
// ... also while it waits for the argument.
const waiting = (async () => Math.floor(await 9.5))();
Math.floor = (x) => "again " + x;
console.log(await waiting, await (async () => Math.floor(await 9.5))());
Math.floor = original;
console.log("restored", all());
Math.max = function () { return "max " + (this === Math) + " " + arguments.length; };
console.log(Math.max(1, 2), [3].map(Math.max).join());
Number.isInteger = () => "never";
String.fromCharCode = (c) => "<" + c + ">";
console.log(all());
