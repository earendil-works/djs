// Found by the plain-dump fuzzer (0211): a class with an async method, declared in a
// loop body and naming the iteration's variable, dumped in the next iteration.
for (const x of [1, 7]) {
  const v = await tools.echo(x);
  class C {
    get(y) { return y + v; }
    async fetch(y) { return await tools.echo(y); }
  }
  console.log(new C().get(1));
}
return "done";
