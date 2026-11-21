let a = [];
for (let i = 0; i < N_ITER; i++) { a.push(i); if (a.length > 1000) a = []; }
return a.length;
