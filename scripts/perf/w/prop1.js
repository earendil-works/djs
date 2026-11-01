const o = { a: 1, b: 2, c: 3, d: 4 };
let s = 0;
for (let i = 0; i < 3_000_000; i++) s += o.c;
return s;
