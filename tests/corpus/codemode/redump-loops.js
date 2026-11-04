// Re-dumps inside loops and scopes (plain dumps, 0207): a function and a generator
// declared in a loop body, an await in a `finally` on every iteration (the `await using`
// case is language/redump-using.js: the slim build has no `using`).
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
console.log("end");
