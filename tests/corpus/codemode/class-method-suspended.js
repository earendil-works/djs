// A suspended async method of a class the body declares, awaited directly (plain dumps
// place the class and the method's continuation in the body's continuation, 0207).
class C {
  async fetch(x) { return await tools.echo(x); }
}
console.log(await new C().fetch(2));
const o = new C();
console.log(await o.fetch(3));
