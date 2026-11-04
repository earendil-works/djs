// difftest: plain-xfail the continuation of an async method of a class in K's block is never assigned: TypeError at load
// Found by the plain-dump fuzzer (0211): an async method waiting, called on an instance
// nothing else holds.
class C {
  async fetch(x) { return await tools.echo(x); }
}
console.log(await new C().fetch(2));
return "done";
