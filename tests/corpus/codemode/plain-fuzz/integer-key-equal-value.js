// Found by the plain-dump fuzzer (0208): an object whose integer-like key reads like its
// value (`{ 5: 5 }`), live across an await.
const o = { 5: 5, x: 1 };
o[2] = 2;
await tools.echo(1);
console.log(JSON.stringify(o));
return o;
