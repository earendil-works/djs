function f(a) { return a + 1; }
let s = 0;
for (let i = 0; i < N_ITER; i++) { s = f(s); }
return s;
