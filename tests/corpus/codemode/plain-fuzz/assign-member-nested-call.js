// Found by the plain-dump fuzzer (0208): a member assignment whose value awaits a call
// that itself waits more than once.
const o = {};
async function f(x) {
  for (let i = 0; i < 2; i++) await tools.echo(i);
  return x;
}
o["k"] = await f(await tools.echo(1));
console.log(JSON.stringify(o));
return o;
