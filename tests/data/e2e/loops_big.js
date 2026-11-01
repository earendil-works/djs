let sum = 0;
for (let i = 0; i < 5000; i++) {
  const s = "item " + i;
  sum += s.length;
}
console.log(sum);
