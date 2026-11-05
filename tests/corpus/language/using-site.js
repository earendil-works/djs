// A site in a `using` declaration's initializer: at the scope's start, after resources of
// the same declaration and before later ones, `await using` too (plain dumps, 0203).
// (tools.echo copies its value: the resources are made after it.)
const res = (tag) => ({ [Symbol.dispose]() { console.log("dispose", tag); } });
const ares = (tag) => ({
  async [Symbol.asyncDispose]() { console.log("adispose", tag, await tools.echo(tag)); },
});
using pre = res("pre");
await using r = (await tools.echo(0), ares("r"));
{
  using a = res("a"), b = (await tools.echo(0), res("b")), c = res("c");
  await using d = ares("d"), e = (await tools.echo(0), ares("e"));
  console.log("inner", await tools.echo(1));
}
for (using x = (await tools.echo(0), res("x")); ; ) { console.log("loop"); break; }
console.log("end");
