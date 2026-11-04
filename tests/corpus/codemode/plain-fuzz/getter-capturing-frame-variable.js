// Found by the plain-dump fuzzer (0208): an object literal's getter that captures a
// variable of the waiting frame (written as `get: ,`).
let n = 0;
const o = { a: 1, get m() { n++; return n; }, set m(v) { n = v; } };
await tools.echo(1);
console.log(o.m, o.m);
o.m = 10;
console.log(await tools.echo(o.m), n);
return o;
