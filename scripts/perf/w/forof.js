const xs = [];
for (let i = 0; i < 200_000; i++) xs.push(i % 977);
let total = 0;
for (let r = 0; r < 20; r++) { for (const x of xs) total += x & r; }
return total;
