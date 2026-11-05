// Found by the plain-dump fuzzer (0208): an await in a default of a destructuring
// assignment whose target is a member.
const o = { a: 1 };
({ a: o.b = await tools.echo(3) } = { a: await tools.echo(undefined) });
console.log(JSON.stringify(o));
return o;
