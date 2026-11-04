// difftest: plain-xfail the instance's prototype, a class made in K's block, is written as an empty reference: load TypeError
// Found by the plain-dump fuzzer (0208): an instance of a class naming a variable of the
// codemode body, captured by an async function that is not running.
class C {
  m() { return tools; }
}
const o = new C();
async function f() {
  for (let i = 0; i < 1; i++) {
    let [] = [await tools.echo(1), o];
  }
}
console.log(typeof o.m());
return await tools.echo(1);
