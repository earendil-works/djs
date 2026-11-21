const str = "hello world";
let s = 0;
for (let i = 0; i < N_ITER; i++) { s += str.charCodeAt(i & 7); }
return s;
