// Found by the plain-dump fuzzer (0211): a `for in` over a Proxy, waiting in its body.
for (const k in new Proxy({ a: 1, b: 2 }, {})) {
  console.log(k, await tools.echo(k));
}
return "done";
