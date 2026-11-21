class P { constructor() { this.v = 1; } m() { return this.v; } }
const p = new P();
let s = 0;
for (let i = 0; i < N_ITER; i++) { s += p.m(); }
return s;
