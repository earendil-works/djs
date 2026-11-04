// Found by the plain-dump fuzzer (0211): a TypeScript class with a parameter property,
// made again by a plain dump.
class C {
  constructor(public value: number) {}
  get twice() { return this.value * 2; }
}
await tools.echo(0);
console.log(new C(4).twice, new C(5).value);
return new C(1);
