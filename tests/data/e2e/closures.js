function counter() { let n = 0; return { inc: () => ++n, get: () => n }; }
const c = counter(), d = counter();
c.inc(); c.inc(); d.inc();
console.log(c.get(), d.get());
const fs = [];
for (let i = 0; i < 3; i++) fs[i] = () => i;
console.log(fs[0](), fs[1](), fs[2]());
function outer() { let x = 1; function mid() { function inner() { return x++; } return inner; } return mid(); }
const f = outer();
console.log(f(), f(), f());
