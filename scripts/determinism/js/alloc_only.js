// Pure interpreter allocation: no builtins that suspend.
let keep = [];
for (let i = 0; i < 40000; i++) {
  const o = { i, a: [i, i + 1], s: "k" + i };
  if (i % 10 === 0) keep.push(o);
}
console.log(keep.length);
