// Found by the plain-dump fuzzer (0211): `for of` over a generator that captures a
// variable of the body, whose loop body waits on `Promise.all` over closures of the
// iteration.
let n = 0;
function* g(k) {
  for (let i = 0; i < k; i++) yield i + n;
}
for (const x of g(2)) {
  console.log(await Promise.all([1, 2].map(async (y) => await tools.echo(y + x))));
}
return "done";
