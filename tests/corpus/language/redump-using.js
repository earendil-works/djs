// An `await using` block whose disposal awaits after an earlier await, with a function in
// the block sharing its `let` (plain re-dumps, 0207).
await tools.echo(0);
{
  await using r = { async [Symbol.asyncDispose]() { console.log("bye", await tools.echo(2)); } };
  let n = 1;
  function f(y) { return n * 2 + y; }
  console.log(f(1));
}
console.log("end");
