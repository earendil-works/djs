// Function calls, closures and recursion (func_call, func_closure_call).
// bench: expect 4750250
function add(a, b) { return a + b; }
function fib(n) { return n < 2 ? n : fib(n - 1) + fib(n - 2); }
const makeCounter = () => { let c = 0; return () => ++c; };
const counter = makeCounter();
let sum = 0;
for (let i = 0; i < 1_000_000; i++) sum = add(sum, i & 7) + (counter() & 1);
return sum + fib(25) * 10;
