let out = "";
for (let i = 0; i < 10; i++) {
  if (i % 3 === 0) continue;
  if (i > 7) break;
  out += i;
}
console.log(out);
let n = 0;
while (n < 5) n++;
do { n += 10 } while (n < 30);
console.log(n);
outer: for (let a = 0; a < 3; a++) {
  for (let b = 0; b < 3; b++) {
    if (b === 1) continue outer;
    if (a === 2) break outer;
    console.log(a, b);
  }
}
