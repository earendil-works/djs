// Found by the plain-dump fuzzer (0211): an array with a getter at an index, followed by
// a pushed element, live across an await.
let n = 1;
const v = Object.defineProperty([5, 6], 2, { get: () => n, enumerable: true, configurable: true });
v.push(7);
await tools.echo(0);
n = 2;
console.log(JSON.stringify(v), v.length);
return v;
