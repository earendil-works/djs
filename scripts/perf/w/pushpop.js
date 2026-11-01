const stack = [];
let total = 0;
for (let i = 0; i < 1_000_000; i++) { stack.push(i); total += stack.pop(); }
return total;
