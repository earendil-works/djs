// Empty loops and integer arithmetic (dquickts microbench: empty_loop, int_arith).
// bench: expect 31249995000000
let sum = 0;
for (let i = 0; i < 5_000_000; i++) {
  sum += i * 3 - (i >> 1);
}
let j = 3_000_000;
while (j > 0) j--;
return sum;
