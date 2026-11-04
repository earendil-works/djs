// In language/, not codemode/: the slim build has no `using` (D2411).
// Found by the plain-dump fuzzer (0208): an `await using` disposal waiting in a block
// with a hoisted function that captures a `let` of the block.
{
  await using r = { async [Symbol.asyncDispose]() { console.log("bye", await tools.echo(2)); } };
  let n = 1;
  function f(y) { return n * 2 + y; }
  console.log(f(1));
}
return "done";
