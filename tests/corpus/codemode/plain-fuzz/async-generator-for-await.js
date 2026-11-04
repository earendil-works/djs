// difftest: plain-xfail K reads the async generator's continuation ($dump.aN) before the callback that makes it: TypeError
// Found by the plain-dump fuzzer (0208): `for await` over an async generator that is
// suspended at its yield while the loop body awaits a tool.
async function* g(k) {
  for (let i = 0; i < k; i++) yield await tools.echo(i);
}
for await (const x of g(2)) {
  console.log(x, await tools.echo(x * 10));
}
return "done";
