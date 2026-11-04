// In language/, not codemode/: the slim build has no `using` (D2411).
// Found by the plain-dump fuzzer (0208): a continuation dumped again while it waits in
// the disposal of an `await using` at the end of a block.
await tools.echo(0);
{
  await using r = { async [Symbol.asyncDispose]() { console.log("bye", await tools.echo(1)); } };
  console.log("in");
}
console.log(await tools.echo(2));
return "done";
