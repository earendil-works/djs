// Found by the plain-dump fuzzer (0208): a class naming a variable of the codemode body,
// made by its static factory inside the async function `Promise.all` maps over.
class C {
  constructor(v) { this.v = v; }
  m() { return tools; }
  static make(v) { return new C(v); }
}
const r = await Promise.all([1, 2].map(async (x) => await tools.echo(x + C.make(2).v)));
console.log(r);
return r;
