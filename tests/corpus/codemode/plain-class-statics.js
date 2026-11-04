// A plain dump makes a class by evaluating its text: its static blocks and static field
// initializers must not run again (0209). Their effects are in the dumped heap; the
// class text is written with them blanked, and the class still works after the load.
const log = [];
class P {
  static n = log.push("init");
  static { log.push("block"); P.made = log.length; }
  static f = () => log.length;
  static #c = 5;
  static label = "p";
  static c() { return P.#c; }
  *[Symbol.iterator]() { yield P.label; }
}
P.label = "changed";
console.log(JSON.stringify(log), P.c());
await tools.echo(1);
class Q extends P { static { log.push("q"); } }
console.log(JSON.stringify(log), P.n, P.made, P.f(), P.c(), P.label, [...new P()].join(), Q.c());
await tools.echo(2);
console.log(JSON.stringify(log), P.c(), Object.keys(P).join());
return log.length;
