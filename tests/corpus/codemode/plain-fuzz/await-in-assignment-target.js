// Found by the plain-dump fuzzer (0208): an await in an assignment's target (a computed
// key, a pattern's default around the site). Plain dumps refuse these on purpose.
const o = { a: 1 };
o[await tools.echo("a")] = await tools.echo(2);
console.log(o.a);
({ a: o.b = await tools.echo(3) } = { a: await tools.echo(undefined) });
console.log(JSON.stringify(o));
return o;
