// Re-dumps inside an inner loop that a later iteration of its outer loop runs as copied
// text: for-in, for-of, for await, try/finally, a labelled continue, a pattern (0215).
async function* gen(n) { for (let i = 0; i < n; i++) yield i; }
let count = 0;
for (const a of [1, 2]) for (const b in a === 1 ? { c: 3 } : { a: 1, b: 2, c: 3 }) { console.log(await tools.echo(b)); count++; }
for (const a of [1, 2, 3]) for (const b of a === 1 ? ["x"] : ["y", "z", "w"]) { console.log(a, await tools.echo(b)); count++; }
for (const a of [1, 2]) for await (const b of gen(a + 1)) { console.log(a, await tools.echo(b)); count++; }
for (const a of [1, 2]) { try { console.log(await tools.echo(a)); } finally { console.log("fin", await tools.echo(a)); count++; } }
outer: for (const a of [1, 2]) for (const b of [1, 2, 3]) { if (b === 3) continue outer; console.log(a, b, await tools.echo(b)); }
for (const a of [1, 2]) { const [x, y = await tools.echo(a)] = [a]; console.log(x, y); }
return count;
