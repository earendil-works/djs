// Known builtins (D3300): redefinitions through defineProperty, getters on the path, and
// values taken from the builtins.
const log = [];
function floor(x) { return Math.floor(x); }
function max(a, b) { return Math.max(a, b); }
function str(x) { return String(x); }
const show = () => { console.log(floor(2.5), max(1, 2), str(3), parseFloat("2.5"), log.join("|")); log.length = 0; };
for (let i = 0; i < 3; i++) show();
const { floor: f, max: m } = Math;
console.log(f(1.5), m(3, 4), [Math.floor][0](9.9), (0, Math.floor)(8.8), Math["floor"](7.7), Math?.floor(6.6), Math.floor?.(5.5));
Object.defineProperty(Math, "floor", { get() { log.push("get floor"); return (x) => "got " + x; }, configurable: true });
show();
Object.defineProperty(Math, "max", { value: () => "defined max", writable: false });
show();
const realMath = Math;
Object.defineProperty(globalThis, "Math", { get() { log.push("get Math"); return realMath; }, configurable: true });
show();
Object.defineProperty(globalThis, "String", { value: (x) => "S" + x });
show();
Object.defineProperty(globalThis, "parseFloat", { get() { log.push("get parseFloat"); return () => "pf"; } });
show();
console.log(Object.getOwnPropertyDescriptor(globalThis, "String").writable, typeof Object.getOwnPropertyDescriptor(globalThis, "Math").get);
