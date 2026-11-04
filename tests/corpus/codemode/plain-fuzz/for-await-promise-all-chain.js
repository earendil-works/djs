// Found by the plain-dump fuzzer (0211): `for await` over an async generator whose body
// awaits a Promise.all over closures of the iteration (the generator's continuation and
// the closures' in the dump's callback).
async function* g(k) { for (let i = 0; i < k; i++) yield await tools.echo(i); }
for await (const x of g(1)) console.log(await Promise.all([1, 2].map(async (y) => await tools.echo(y + x))));
