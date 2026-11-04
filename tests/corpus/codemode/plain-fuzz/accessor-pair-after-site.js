// Found by the plain-dump fuzzer (0208): an accessor pair after an await in an object
// literal.
let n = 0;
const o = { a: await tools.echo(1), get b() { return n; }, set b(v) { n = v; } };
o.b = 5;
console.log(JSON.stringify(o), n);
return o;
