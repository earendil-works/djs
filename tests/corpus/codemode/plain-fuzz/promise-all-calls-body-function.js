// Found by the plain-dump fuzzer (0211): `Promise.all` over a mapped async closure that
// calls a function of the body and assigns to an object of the body.
function id(x) { return x; }
const o = { value: 0 };
let n = 2;
console.log(await Promise.all([1, 2].map(async (x) => await tools.echo(id(x) + (o.value = n)))));
return o;
