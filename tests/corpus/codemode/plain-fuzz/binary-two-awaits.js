// Found by the plain-dump fuzzer (0208): two awaits in one binary expression; the
// continuation, dumped again at the second, must map back to the original.
const x = (await tools.echo(1)) + (await tools.echo(2));
console.log(x);
async function f(v) {
  return await tools.echo(v);
}
console.log((await f(3)) * (await f(4)));
return x;
