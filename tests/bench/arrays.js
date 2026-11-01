// Array push, indexed reads and writes, length, slice (array_*).
// bench: expect 49500151801000
const a = [];
for (let i = 0; i < 1_000_000; i++) a.push(i);
let sum = 0;
for (let r = 0; r < 3; r++) {
  for (let i = 0; i < a.length; i++) {
    a[i] = a[i] + 1;
    sum += a[i];
  }
}
for (let i = 0; i < 1000; i++) sum += a.slice(i, i + 100).length;
while (a.length > 1000) a.pop();
return sum * 33 + a.length;
