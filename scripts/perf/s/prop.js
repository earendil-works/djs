const o = { a: 1, b: 2, c: 3 };
let s = 0;
for (let i = 0; i < N_ITER; i++) { s += o.c; o.a = i; }
return s;
