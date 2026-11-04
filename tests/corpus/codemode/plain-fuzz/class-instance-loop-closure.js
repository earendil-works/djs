// Found by the plain-dump fuzzer (0208): a class naming a variable of the codemode body,
// an instance of it, and a closure of a loop iteration waiting on a tool.
class C {
  m() { return tools; }
}
const o = new C();
for (let i = 0; i < 2; i++) {
  const h = async () => console.log(`${await tools.echo("ok")}-${i}`);
  await h();
}
return typeof o.m();
