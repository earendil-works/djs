// Found by the plain-dump fuzzer (0208): functions declared in a loop body, made again
// in every iteration, while a continuation of the loop is dumped again.
for (const x of [1, 2]) {
  async function f() {
    return await tools.echo(x);
  }
  function* g() {
    yield x * 10;
  }
  console.log(await f(), [...g()]);
}
return "done";
