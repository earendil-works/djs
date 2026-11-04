// difftest: plain-xfail K reads the generator's continuation before the callback that makes it: TypeError
// Found by the plain-dump fuzzer (0208): `for of` over a generator that captures a
// variable of the frame, with an await in the loop body.
let n = 1;
function* g(k) {
  for (let i = 0; i < k; i++) yield i + n;
}
for (const x of g(2)) {
  n += await tools.echo(x);
}
return n;
