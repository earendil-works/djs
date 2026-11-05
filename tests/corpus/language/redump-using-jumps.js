// A `continue` or `break` of a `for await` pending while an `await using` scope disposes,
// in the rest of an iteration a continuation runs in its scaffold (plain re-dumps, 0215).
async function* g(k) { for (let i = 0; i < k; i++) yield await tools.echo(i); }
const res = (n) => ({ async [Symbol.asyncDispose]() { console.log("bye", n, await tools.echo(n)); } });
for await (const x of g(2)) {
  try { await tools.echo("x"); throw new Error("ko"); } catch (e) { console.log(e.message); }
  { await using r = res("c" + x); if (true) continue; }
}
for await (const x of g(3)) {
  await tools.echo(x);
  { await using r = res("b" + x); if (x === 1) break; }
}
outer: for (const a of [1, 2]) {
  for await (const x of g(2)) {
    await tools.echo(x);
    { await using r = res("o" + a + x); if (x === 0) continue outer; }
  }
}
console.log("end");
