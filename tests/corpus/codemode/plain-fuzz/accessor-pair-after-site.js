// difftest: plain-xfail K defines a getter and a setter of one key with two DD literals: the setter's replaces the getter
// Found by the plain-dump fuzzer (0208): an accessor pair after an await in an object
// literal.
let n = 0;
const o = { a: await tools.echo(1), get b() { return n; }, set b(v) { n = v; } };
o.b = 5;
console.log(JSON.stringify(o), n);
return o;
