// Found by the plain-dump fuzzer (0208): an await in a `finally` inside a loop, dumped
// again in the next iteration.
for (const x of [1, 2]) {
  try {
    console.log("try", x);
  } finally {
    console.log(await tools.echo(x));
  }
}
return "done";
