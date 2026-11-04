// Found by the plain-dump fuzzer (0208): a class naming a variable of the codemode body,
// used by an async closure that waits.
class C {
  m() { return tools; }
}
const f = async (x) => (await tools.echo(x)) + typeof new C().m();
console.log(await f(1));
return "done";
