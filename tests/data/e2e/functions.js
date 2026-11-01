function add(a, b = 10) { return a + b; }
const mul = (a, b) => a * b;
function fact(n) { return n <= 1 ? 1 : n * fact(n - 1); }
console.log(add(1), add(1, 2), mul(3, 4), fact(10), add.length, add.name, mul.name);
function sum(...xs) { let t = 0; for (let i = 0; i < xs.length; i++) t += xs[i]; return t; }
console.log(sum(), sum(1, 2, 3), sum.length);
function args() { return arguments.length + ":" + arguments[1]; }
console.log(args(1, "two", 3), typeof function () {}, typeof (() => 1));
