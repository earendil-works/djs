// Found by the plain-dump fuzzer (0211): in an async function, a `for of` over iterator
// helpers of a generator that captures a variable of the body, with nested
// `Promise.all` over mapped closures in the loop body.
let n = 0;
function* g(k) {
  for (let i = 0; i < k; i++) yield i + n;
}
const f = async (a) => {
  for (const x of g(3).map((x) => x * 2).take(2)) {
    const r = await Promise.all((await Promise.all([x, n].map(async (a) => await tools.echo(a + 1)))).map(async (b) => await tools.echo(b + x)));
    console.log(r);
  }
};
await f(1);
return "done";
