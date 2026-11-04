// Found by the plain-dump fuzzer (0211): `for of` over a generator capturing a frame
// variable whose body awaits a Promise.all over closures of the iteration (the
// generator's continuation made in the callback's part of the loop's block).
let n = 0;
function* g(k) { for (let i = 0; i < k; i++) yield i + n; }
for (const x of g(2)) console.log(await Promise.all([1, 2].map(async (y) => await tools.echo(y + x))));
