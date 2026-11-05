// Found by the plain-dump fuzzer (0211): `Array.fromAsync` over an async generator,
// waiting on a tool inside it.
async function* g(k) {
  for (let i = 0; i < k; i++) yield await tools.echo(i);
}
console.log(await Array.fromAsync(g(2)));
return "done";
