// Re-dumps inside loops and scopes (plain dumps, 0207): a function and a generator
// declared in a loop body, an await in a `finally` on every iteration, an `await using`
// block whose disposal awaits, with a function in the block sharing its `let`.
for (const x of [1, 2]) {
  async function g() { return await tools.echo(x); }
  function* h() { yield x * 10; }
  console.log(await g(), h().next().value);
}
for (const x of [1, 2]) {
  try {
    console.log("try", x);
  } finally {
    console.log("finally", await tools.echo(x));
  }
}
await tools.echo(0);
{
  await using r = { async [Symbol.asyncDispose]() { console.log("bye", await tools.echo(2)); } };
  let n = 1;
  function f(y) { return n * 2 + y; }
  console.log(f(1));
}
console.log("end");
