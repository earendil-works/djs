function f(a) { return a; }
let s = 0;
for (let i = 0; i < 3_000_000; i++) s += f(i);
return s;
