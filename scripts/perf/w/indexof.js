const xs = [];
for (let i = 0; i < 1000; i++) xs.push(i);
let total = 0;
for (let r = 0; r < 5000; r++) total += xs.indexOf(r % 1000);
return total;
