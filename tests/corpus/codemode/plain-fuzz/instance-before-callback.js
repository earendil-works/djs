// Found by the plain-dump fuzzer (0211): an instance of a class of the body, bound in the
// body and read by a function, made by the dump's callback after the class.
class C { get(x) { return x + tools.length; } }
const o = new C();
function f() { return o.get(1); }
await tools.echo(0);
console.log(f());
